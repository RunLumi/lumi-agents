//! Anthropic messages adapter.

use crate::capabilities::{tool_model, ModelCapability, ModelInfo};
use crate::driver::ProviderDriver;
use crate::error::{ModelError, TransportError};
use crate::request::{ModelMessage, ModelRequest};
use crate::response::{FinishReason, ModelResponse, ToolCall, Usage};
use crate::transport::{HttpResponse, HttpTransport};
use serde_json::json;
use std::time::Instant;

/// The Anthropic driver family (`/v1/messages` protocol).
#[derive(Debug, Default, Clone, Copy)]
pub struct AnthropicDriver;

impl AnthropicDriver {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub(crate) const WIRE: Wire = Wire {};
}

pub(crate) struct Wire {}

impl Wire {
    pub(crate) const API_VERSION: &str = "2023-06-01";

    pub(crate) fn build_body(&self, request: &ModelRequest) -> serde_json::Value {
        let mut system_parts: Vec<String> = Vec::new();
        let mut messages: Vec<serde_json::Value> = Vec::new();
        for message in &request.messages {
            match message {
                ModelMessage::System { content } => system_parts.push(content.clone()),
                ModelMessage::User { content } => messages.push(json!({
                    "role": "user",
                    "content": [{"type": "text", "text": content}],
                })),
                ModelMessage::UserWithImage {
                    text,
                    image_mime_type,
                    image_base64,
                } => messages.push(json!({
                    "role": "user",
                    "content": [
                        {"type": "image", "source": {
                            "type": "base64",
                            "media_type": image_mime_type,
                            "data": image_base64,
                        }},
                        {"type": "text", "text": text},
                    ],
                })),
                ModelMessage::Assistant { content } => messages.push(json!({
                    "role": "assistant",
                    "content": [{"type": "text", "text": content}],
                })),
                ModelMessage::AssistantToolCall { call } => messages.push(json!({
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": call.id,
                        "name": call.name,
                        "input": call.arguments,
                    }],
                })),
                ModelMessage::ToolResult {
                    tool_call_id,
                    content,
                } => messages.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_call_id,
                        "content": content,
                    }],
                })),
            }
        }

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            // Anthropic requires an explicit max_tokens.
            "max_tokens": request.max_output_tokens.unwrap_or(4_096),
        });
        let obj = body.as_object_mut().expect("body is an object");
        if !system_parts.is_empty() {
            obj.insert("system".to_owned(), json!(system_parts.join("\n")));
        }
        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            obj.insert("tools".to_owned(), json!(tools));
        }
        if let Some(temp) = request.temperature {
            obj.insert("temperature".to_owned(), json!(temp));
        }
        body
    }

    pub(crate) fn url(&self, endpoint: &str) -> String {
        format!("{}/v1/messages", endpoint.trim_end_matches('/'))
    }

    pub(crate) fn parse_success(
        &self,
        request: &ModelRequest,
        response: &HttpResponse,
    ) -> Result<ModelResponse, ModelError> {
        let value = super::wire::parse_json(response, "anthropic")?;
        let id = value.get("id").and_then(|v| v.as_str()).map(str::to_owned);
        let content_blocks = value
            .get("content")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        for block in &content_blocks {
            match block.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        text_parts.push(text.to_owned());
                    }
                }
                Some("tool_use") => {
                    let call = ToolCall {
                        id: block
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                        name: block
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                        arguments: block
                            .get("input")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    };
                    if !call.arguments.is_object() {
                        return Err(ModelError::format(
                            "anthropic: tool_use input is not a JSON object",
                        ));
                    }
                    tool_calls.push(call);
                }
                _ => {}
            }
        }

        let stop_reason = value.get("stop_reason").and_then(|s| s.as_str());
        let finish_reason = match stop_reason {
            Some("tool_use") => FinishReason::ToolCall,
            Some("max_tokens") => FinishReason::Length,
            Some("refusal") => FinishReason::Refusal,
            // end_turn and unknowns map to Stop.
            _ => FinishReason::Stop,
        };

        let usage = value.get("usage").map(|u| Usage {
            input_tokens: u
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or_default(),
            output_tokens: u
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or_default(),
        });
        let Some(usage) = usage else {
            return Err(ModelError::format("anthropic: response missing usage"));
        };

        // Structured output: Anthropic has no native JSON mode; the
        // adapter extracts a JSON object from the assistant text and
        // fails loudly when absent (spec 09 §9.13 honesty for v1).
        let structured = if request.structured_output_schema.is_some() {
            let text = text_parts.join("");
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) if v.is_object() => Some(v),
                _ => {
                    return Err(ModelError::format(
                        "anthropic: structured output is not a JSON object",
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
            provider_request_id: id,
            latency_ms: 0,
            safety_signal: None,
        })
    }

    pub(crate) fn complete(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelError> {
        super::wire::validate_request(request, "anthropic")?;
        let body = self.build_body(request).to_string();
        let response = transport
            .post_json(
                &self.url(endpoint),
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
        info.with_context(200_000).with_regions(["us"])
    }

    pub(crate) fn list_models(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
    ) -> Result<Vec<String>, ModelError> {
        let response = transport
            .get_json(
                &format!("{}/v1/models", endpoint.trim_end_matches('/')),
                auth_headers,
                std::time::Duration::from_secs(10),
            )
            .map_err(map_transport)?;
        if response.status != 200 {
            return Err(map_error_body(&response));
        }
        let value = super::wire::parse_json(&response, "anthropic")?;
        Ok(value
            .get("data")
            .and_then(|d| d.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(str::to_owned))
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
    let error_type = parsed
        .as_ref()
        .and_then(|v| v.get("error"))
        .and_then(|e| e.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_owned();
    let message = parsed
        .as_ref()
        .and_then(|v| v.get("error"))
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or(&response.body)
        .to_owned();
    match response.status {
        429 => ModelError::rate_limit(format!("anthropic rate limited: {message}")),
        401 | 403 => ModelError::auth(format!("anthropic auth rejected: {message}")),
        500..=599 => ModelError::unavailable(format!("anthropic server error: {message}")),
        400 if error_type == "invalid_request_error"
            && message.to_lowercase().contains("context length") =>
        {
            ModelError::context_limit(format!("anthropic: {message}"))
        }
        400..=499 => {
            let mut e = ModelError::new(
                lumi_protocol::FailureCategory::ModelReasoning,
                format!("anthropic rejected request: {message}"),
                false,
            );
            e.native_code = Some(error_type);
            e
        }
        status => {
            ModelError::unavailable(format!("anthropic unexpected status {status}: {message}"))
        }
    }
}

impl ProviderDriver for AnthropicDriver {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn auth_headers(&self, credential: &str) -> Vec<(String, String)> {
        vec![
            ("x-api-key".to_owned(), credential.to_owned()),
            ("anthropic-version".to_owned(), Wire::API_VERSION.to_owned()),
        ]
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
