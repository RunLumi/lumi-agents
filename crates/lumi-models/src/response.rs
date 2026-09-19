//! Normalized model response (spec 09 §9.6).

use serde::{Deserialize, Serialize};

/// A tool call the model produced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Arguments as a JSON object value.
    pub arguments: serde_json::Value,
}

/// Token usage reported by the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Why generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCall,
    /// Provider safety/refusal signal (spec 09 §9.14): surfaced, never
    /// overriding Lumi policy.
    Refusal,
    Error,
}

/// The normalized response (spec 09 §9.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResponse {
    pub request_id: String,
    /// Assistant text content, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls requested by the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Structured output payload when requested and produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured: Option<serde_json::Value>,
    pub usage: Usage,
    pub finish_reason: FinishReason,
    /// Provider correlation id (request id at the provider).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request_id: Option<String>,
    /// Wall-clock latency of the call.
    pub latency_ms: u64,
    /// Provider safety/refusal detail, when signaled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_signal: Option<String>,
}
