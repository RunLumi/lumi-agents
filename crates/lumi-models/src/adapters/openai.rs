//! OpenAI chat-completions adapter.

use crate::capabilities::{text_model, tool_model, ModelCapability, ModelInfo};
use crate::driver::ProviderDriver;
use crate::error::{ModelError, TransportError};
use crate::request::{ModelMessage, ModelRequest};
use crate::response::{FinishReason, ModelResponse, ToolCall, Usage};
use crate::transport::{HttpResponse, HttpTransport};
use serde_json::json;
use std::time::Instant;

/// The OpenAI driver family (`chat/completions` protocol).
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenAIDriver;

impl OpenAIDriver {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub(crate) const fn wire() -> OpenAIWire {
        OpenAIWire { name: "openai" }
    }
}

/// Shared implementation used by both `openai` and `openai-compatible`
/// drivers with their own identities (spec 09 §9.13).
pub(crate) struct OpenAIWire {
    pub name: &'static str,
}

impl OpenAIWire {
    pub(crate) fn build_body(&self, request: &ModelRequest) -> serde_json::Value {
        let mut messages = Vec::with_capacity(request.messages.len());
        let mut pending_tool_results: Vec<(String, String)> = Vec::new();
        for message in &request.messages {
            match message {
                ModelMessage::System { content } => messages.push(json!({
                    "role": "system",
                    "content": content,
                })),
                ModelMessage::User { content } => messages.push(json!({
                    "role": "user",
                    "content": content,
                })),
                ModelMessage::UserWithImage {
                    text,
                    image_mime_type,
                    image_base64,
                } => messages.push(json!({
                    "role": "user",
                    "content": [
                        {"type": "text", "text": text},
                        {"type": "image_url", "image_url": {
                            "url": format!("data:{image_mime_type};base64,{image_base64}")
                        }},
                    ],
                })),
                ModelMessage::Assistant { content } => messages.push(json!({
                    "role": "assistant",
                    "content": content,
                })),
                ModelMessage::AssistantToolCall { call } => {
                    messages.push(json!({
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": call.id,
                            "type": "function",
                            "function": {
                                "name": call.name,
                                "arguments": call.arguments.to_string(),
                            },
                        }],
                    }));
                }
                ModelMessage::ToolResult {
                    tool_call_id,
                    content,
                } => {
                    pending_tool_results.push((tool_call_id.clone(), content.clone()));
                }
            }
        }
        for (id, content) in pending_tool_results {
            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": content,
            }));
        }

        let mut body = json!({
            "model": request.model,
            "messages": messages,
        });
        let obj = body.as_object_mut().expect("body is an object");
        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        },
                    })
                })
                .collect();
            obj.insert("tools".to_owned(), json!(tools));
            obj.insert("tool_choice".to_owned(), json!("auto"));
        }
        if request.structured_output_schema.is_some() {
            obj.insert("response_format".to_owned(), json!({"type": "json_object"}));
        }
        if let Some(max) = request.max_output_tokens {
            obj.insert("max_tokens".to_owned(), json!(max));
        }
        if let Some(temp) = request.temperature {
            obj.insert("temperature".to_owned(), json!(temp));
        }
        body
    }

    pub(crate) fn url(&self, endpoint: &str) -> String {
        format!("{}/v1/chat/completions", endpoint.trim_end_matches('/'))
    }

    pub(crate) fn parse_success(
        &self,
        request: &ModelRequest,
        response: &HttpResponse,
    ) -> Result<ModelResponse, ModelError> {
        let value = super::wire::parse_json(response, self.name)?;
        let id = value.get("id").and_then(|v| v.as_str()).map(str::to_owned);
        let choice = value
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| ModelError::format("openai: missing choices[0]"))?;
        let message = choice.get("message").cloned().unwrap_or_default();
        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .map(str::to_owned);
        let tool_calls: Vec<ToolCall> = message
            .get("tool_calls")
            .and_then(|t| t.as_array())
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|call| {
                        Some(ToolCall {
                            id: call.get("id")?.as_str()?.to_owned(),
                            name: call.get("function")?.get("name")?.as_str()?.to_owned(),
                            arguments: call
                                .get("function")?
                                .get("arguments")
                                .and_then(|a| a.as_str())
                                .and_then(|a| serde_json::from_str(a).ok())
                                .unwrap_or(serde_json::Value::Null),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Malformed tool output must be surfaced, not silently accepted
        // (spec 09 §9.15): function arguments must parse as an object.
        for call in &tool_calls {
            if !call.arguments.is_object() {
                return Err(ModelError::format(format!(
                    "openai: tool call {} arguments are not a JSON object",
                    call.name
                )));
            }
        }

        let finish_reason = match choice.get("finish_reason").and_then(|f| f.as_str()) {
            Some("tool_calls") => FinishReason::ToolCall,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::Refusal,
            // Unknown reasons conservatively map to Stop; provider detail
            // stays available via provider_request_id + audit.
            _ => FinishReason::Stop,
        };

        let usage = value.get("usage").map(|u| Usage {
            input_tokens: u
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or_default(),
            output_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or_default(),
        });
        let Some(usage) = usage else {
            // Usage reporting is a declared capability: absence is a
            // format failure, not silent zero-cost.
            return Err(ModelError::format("openai: response missing usage"));
        };

        let structured = if request.structured_output_schema.is_some() {
            let content = content.clone().unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(v) if v.is_object() => Some(v),
                _ => {
                    return Err(ModelError::format(
                        "openai: structured output is not a JSON object",
                    ))
                }
            }
        } else {
            None
        };

        let refusal = message
            .get("refusal")
            .and_then(|r| r.as_str())
            .map(str::to_owned);

        Ok(ModelResponse {
            request_id: request.request_id.clone(),
            content,
            tool_calls,
            structured,
            usage,
            finish_reason,
            provider_request_id: id,
            latency_ms: 0,
            safety_signal: refusal,
        })
    }

    pub(crate) fn complete(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelError> {
        super::wire::validate_request(request, self.name)?;
        let body = self.build_body(request).to_string();
        let response = transport
            .post_json(
                &self.url(endpoint),
                auth_headers,
                &body,
                std::time::Duration::from_millis(request.timeout_ms),
            )
            .map_err(|e| match e {
                TransportError::Timeout => ModelError::timeout(),
                TransportError::Connect(detail) => ModelError::connect(detail),
                TransportError::Malformed(detail) => ModelError::format(detail),
                TransportError::NoTransport => ModelError::connect("no HTTP transport compiled"),
            })?;
        if response.status != 200 {
            return Err(super::wire::map_http_error(self.name, &response));
        }
        self.parse_success(request, &response)
    }

    pub(crate) fn describe(&self, model: &str) -> ModelInfo {
        // Conservative catalog: reasoning/tool models for known modern
        // ids; plain text otherwise. Deployments refine via instance
        // catalog.
        let mut info = tool_model(model);
        if model.contains("mini") || model.contains("embed") {
            info = text_model(model);
        }
        if model.contains("embed") {
            info.capabilities = std::collections::BTreeSet::from([
                ModelCapability::Text,
                ModelCapability::UsageReporting,
            ]);
        }
        info.with_context(128_000).with_regions(["us"])
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
            .map_err(|e| match e {
                TransportError::Timeout => ModelError::timeout(),
                TransportError::Connect(detail) => ModelError::connect(detail),
                TransportError::Malformed(detail) => ModelError::format(detail),
                TransportError::NoTransport => {
                    ModelError::format("models listing unsupported by transport")
                }
            })?;
        if response.status != 200 {
            return Err(super::wire::map_http_error(self.name, &response));
        }
        let value = super::wire::parse_json(&response, self.name)?;
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

impl ProviderDriver for OpenAIDriver {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn describe(&self, model: &str) -> ModelInfo {
        OpenAIWire::describe(&Self::wire(), model)
    }

    fn complete(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
        request: &ModelRequest,
    ) -> Result<ModelResponse, ModelError> {
        let start = Instant::now();
        let mut response =
            OpenAIWire::complete(&Self::wire(), transport, endpoint, auth_headers, request)?;
        response.latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        Ok(response)
    }

    fn list_models(
        &self,
        transport: &dyn HttpTransport,
        endpoint: &str,
        auth_headers: &[(String, String)],
    ) -> Result<Vec<String>, ModelError> {
        OpenAIWire::list_models(&Self::wire(), transport, endpoint, auth_headers)
    }
}
