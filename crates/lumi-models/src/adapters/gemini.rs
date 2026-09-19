//! Gemini generateContent adapter.

use crate::capabilities::{tool_model, ModelCapability, ModelInfo};
use crate::driver::ProviderDriver;
use crate::error::{ModelError, TransportError};
use crate::request::{ModelMessage, ModelRequest};
use crate::response::{FinishReason, ModelResponse, ToolCall, Usage};
use crate::transport::{HttpResponse, HttpTransport};
use serde_json::json;
use std::time::Instant;

/// The Gemini driver family (`:generateContent` protocol).
#[derive(Debug, Default, Clone, Copy)]
pub struct GeminiDriver;

impl GeminiDriver {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub(crate) const WIRE: Wire = Wire {};
}

pub(crate) struct Wire {}

impl Wire {
    pub(crate) fn build_body(&self, request: &ModelRequest) -> serde_json::Value {
        let mut system_parts: Vec<String> = Vec::new();
        let mut contents: Vec<serde_json::Value> = Vec::new();
        for message in &request.messages {
            match message {
                ModelMessage::System { content } => system_parts.push(content.clone()),
                ModelMessage::User { content } => contents.push(json!({
                    "role": "user",
                    "parts": [{"text": content}],
                })),
                ModelMessage::UserWithImage {
                    text,
                    image_mime_type,
                    image_base64,
                } => contents.push(json!({
                    "role": "user",
                    "parts": [
                        {"inline_data": {"mime_type": image_mime_type, "data": image_base64}},
                        {"text": text},
                    ],
                })),
                ModelMessage::Assistant { content } => contents.push(json!({
                    "role": "model",
                    "parts": [{"text": content}],
                })),
                ModelMessage::AssistantToolCall { call } => contents.push(json!({
                    "role": "model",
                    "parts": [{"functionCall": {"name": call.name, "args": call.arguments}}],
                })),
                ModelMessage::ToolResult {
                    tool_call_id,
                    content,
                } => {
                    let _ = tool_call_id;
                    contents.push(json!({
                        "role": "user",
                        "parts": [{"functionResponse": {
                            "name": "tool",
                            "response": {"result": content},
                        }}],
                    }));
                }
            }
        }

        let mut body = json!({ "contents": contents });
        let obj = body.as_object_mut().expect("body is an object");
        if !system_parts.is_empty() {
            obj.insert(
                "systemInstruction".to_owned(),
                json!({"parts": [{"text": system_parts.join("\n")}]}),
            );
        }
        let mut generation_config = json!({});
        if !request.tools.is_empty() {
            let declarations: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    })
                })
                .collect();
            obj.insert(
                "tools".to_owned(),
                json!([{"function_declarations": declarations}]),
            );
        }
        if request.structured_output_schema.is_some() {
            generation_config["responseMimeType"] = json!("application/json");
        }
        if let Some(max) = request.max_output_tokens {
            generation_config["maxOutputTokens"] = json!(max);
        }
        if let Some(temp) = request.temperature {
            generation_config["temperature"] = json!(temp);
        }
        if generation_config.as_object().is_some_and(|o| !o.is_empty()) {
            obj.insert("generationConfig".to_owned(), generation_config);
        }
        body
    }

    pub(crate) fn url(&self, endpoint: &str, model: &str) -> String {
        format!(
            "{}/v1beta/models/{}:generateContent",
            endpoint.trim_end_matches('/'),
            model
        )
    }

    pub(crate) fn parse_success(
        &self,
        request: &ModelRequest,
        response: &HttpResponse,
    ) -> Result<ModelResponse, ModelError> {
        let value = super::wire::parse_json(response, "gemini")?;
        let candidate = value
            .get("candidates")
            .and_then(|c| c.get(0))
            .ok_or_else(|| ModelError::format("gemini: missing candidates[0]"))?;
        let parts = candidate
            .get("content")
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();

        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        for part in &parts {
            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                text_parts.push(text.to_owned());
            }
            if let Some(call) = part.get("functionCall") {
                let tool_call = ToolCall {
                    id: format!("gemini-call-{}", tool_calls.len()),
                    name: call
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or_default()
                        .to_owned(),
                    arguments: call.get("args").cloned().unwrap_or(serde_json::Value::Null),
                };
                if !tool_call.arguments.is_object() {
                    return Err(ModelError::format(
                        "gemini: functionCall args is not a JSON object",
                    ));
                }
                tool_calls.push(tool_call);
            }
        }

        // Gemini reports STOP even when functionCall parts are present:
        // presence of calls decides the finish reason.
        let finish_reason = if !tool_calls.is_empty() {
            FinishReason::ToolCall
        } else {
            match candidate.get("finishReason").and_then(|f| f.as_str()) {
                Some("MAX_TOKENS") => FinishReason::Length,
                Some("SAFETY") | Some("PROHIBITED_CONTENT") => FinishReason::Refusal,
                _ => FinishReason::Stop,
            }
        };

        let metadata = value.get("usageMetadata");
        let usage = Usage {
            input_tokens: metadata
                .and_then(|m| m.get("promptTokenCount"))
                .and_then(|v| v.as_u64())
                .unwrap_or_default(),
            output_tokens: metadata
                .and_then(|m| m.get("candidatesTokenCount"))
                .and_then(|v| v.as_u64())
                .unwrap_or_default(),
        };

        let structured = if request.structured_output_schema.is_some() {
            let text = text_parts.join("");
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) if v.is_object() => Some(v),
                _ => {
                    return Err(ModelError::format(
                        "gemini: structured output is not a JSON object",
                    ))
                }
            }
        } else {
            None
        };

        Ok(ModelResponse {
            request_id: request.request_id.clone(),
            content: if text_parts.is_empty() {
                None
            } else {
                Some(text_parts.join(""))
            },
            tool_calls,
            structured,
            usage,
            finish_reason,
            provider_request_id: value
                .get("responseId")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            latency_ms: 0,
            safety_signal: candidate
                .get("finishReason")
                .and_then(|f| f.as_str())
                .filter(|f| *f == "SAFETY" || *f == "PROHIBITED_CONTENT")
                .map(str::to_owned),
        })
    }

    pub(crate) fn complete(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelError> {
        super::wire::validate_request(request, "gemini")?;
        let body = self.build_body(request).to_string();
        let response = transport
            .post_json(
                &self.url(endpoint, &request.model),
                auth_headers,
                &body,
                std::time::Duration::from_millis(request.timeout_ms),
            )
            .map_err(map_transport)?;
        if response.status != 200 {
            return Err(map_error_body(&response));
        }
        self.parse_success(request, &response)
    }

    pub(crate) fn describe(&self, model: &str) -> ModelInfo {
        let mut info = tool_model(model);
        info.capabilities.insert(ModelCapability::Vision);
        info.with_context(1_000_000).with_regions(["us"])
    }

    pub(crate) fn list_models(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
    ) -> Result<Vec<String>, ModelError> {
        let response = transport
            .get_json(
                &format!("{}/v1beta/models", endpoint.trim_end_matches('/')),
                auth_headers,
                std::time::Duration::from_secs(10),
            )
            .map_err(map_transport)?;
        if response.status != 200 {
            return Err(map_error_body(&response));
        }
        let value = super::wire::parse_json(&response, "gemini")?;
        Ok(value
            .get("models")
            .and_then(|d| d.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|m| m.get("name").and_then(|i| i.as_str()).map(str::to_owned))
                    .map(|name| name.trim_start_matches("models/").to_owned())
                    .collect()
            })
            .unwrap_or_default())
    }
}

fn map_transport(e: TransportError) -> ModelError {
    match e {
        TransportError::Timeout => ModelError::timeout(),
        TransportError::Connect(detail) => ModelError::connect(detail),
        TransportError::Malformed(detail) => ModelError::format(detail),
        TransportError::NoTransport => ModelError::connect("no HTTP transport compiled"),
    }
}

fn map_error_body(response: &HttpResponse) -> ModelError {
    let parsed = serde_json::from_str::<serde_json::Value>(&response.body).ok();
    let message = parsed
        .as_ref()
        .and_then(|v| v.get("error"))
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or(&response.body)
        .to_owned();
    let status_code = parsed
        .as_ref()
        .and_then(|v| v.get("error"))
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_u64());
    match response.status {
        429 => ModelError::rate_limit(format!("gemini rate limited: {message}")),
        401 | 403 => ModelError::auth(format!("gemini auth rejected: {message}")),
        500..=599 => ModelError::unavailable(format!("gemini server error: {message}")),
        400 if message.to_lowercase().contains("token limit")
            || message.to_lowercase().contains("token count")
            || message.to_lowercase().contains("context") =>
        {
            ModelError::context_limit(format!("gemini: {message}"))
        }
        400..=499 => {
            let mut e = ModelError::new(
                lumi_protocol::FailureCategory::ModelReasoning,
                format!("gemini rejected request: {message}"),
                false,
            );
            e.native_code = status_code.map(|c| c.to_string());
            e
        }
        status => ModelError::unavailable(format!("gemini unexpected status {status}: {message}")),
    }
}

impl ProviderDriver for GeminiDriver {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn auth_headers(&self, credential: &str) -> Vec<(String, String)> {
        vec![("x-goog-api-key".to_owned(), credential.to_owned())]
    }

    fn describe(&self, model: &str) -> ModelInfo {
        Self::WIRE.describe(model)
    }

    fn complete(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelError> {
        let start = Instant::now();
        let mut response = Self::WIRE.complete(transport, endpoint, auth_headers, request)?;
        response.latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        Ok(response)
    }

    fn list_models(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
    ) -> Result<Vec<String>, ModelError> {
        Self::WIRE.list_models(transport, endpoint, auth_headers)
    }
}
