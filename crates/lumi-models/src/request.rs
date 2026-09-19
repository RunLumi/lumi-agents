//! Normalized model request (spec 09 §9.5).

use crate::response::ToolCall;
use serde::{Deserialize, Serialize};

/// One conversation message. Roles mirror the common denominator across
/// providers; adapters translate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "role")]
pub enum ModelMessage {
    /// Trusted system/task instructions. Never derived from untrusted
    /// content (prompt-injection rule).
    System {
        content: String,
    },
    User {
        content: String,
    },
    /// User turn carrying a local image (vision-capable roles only).
    UserWithImage {
        text: String,
        image_mime_type: String,
        image_base64: String,
    },
    /// Assistant turn (for multi-turn context).
    Assistant {
        content: String,
    },
    /// The model's own tool call, replayed as context.
    AssistantToolCall {
        call: ToolCall,
    },
    /// Tool result feeding back to the model.
    ToolResult {
        tool_call_id: String,
        content: String,
    },
}

/// A tool the model may call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema for the arguments object.
    pub parameters: serde_json::Value,
}

/// What the request is for (spec 09 §9.9 roles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    Planner,
    Extractor,
    Classifier,
    Verifier,
    Code,
    VisionGrounder,
    Summarizer,
    Embedding,
}

/// The normalized request (spec 09 §9.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    /// Stable request id for correlation/audit.
    pub request_id: String,
    pub role: ModelRole,
    /// Provider-agnostic model id (adapter maps to its own catalog id).
    pub model: String,
    pub messages: Vec<ModelMessage>,
    /// Tools offered, when the role needs them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolSpec>,
    /// Ask for JSON-constrained output matching this schema name. Adapters
    /// translate to provider-appropriate mechanisms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_output_schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    /// Sampling temperature; `None` = provider default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Wall-clock timeout for this request.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    60_000
}

impl ModelRequest {
    /// Convenience text completion constructor.
    #[must_use]
    pub fn text(request_id: impl Into<String>, role: ModelRole, model: &str, prompt: &str) -> Self {
        Self {
            request_id: request_id.into(),
            role,
            model: model.to_owned(),
            messages: vec![ModelMessage::User {
                content: prompt.to_owned(),
            }],
            tools: Vec::new(),
            structured_output_schema: None,
            max_output_tokens: None,
            temperature: None,
            timeout_ms: default_timeout_ms(),
        }
    }
}
