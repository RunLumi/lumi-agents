//! Provider adapter families (spec 09 §9.3).
//!
//! Each adapter translates the normalized request/response to exactly one
//! provider wire protocol and back, mapping provider errors onto the
//! canonical failure taxonomy. All adapters share the contract suite in
//! `tests/contract.rs` — including `openai-compatible`, whose
//! compatibility is proven there rather than assumed (spec 09 §9.13).

pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod openai_compatible;

pub use anthropic::AnthropicDriver;
pub use gemini::GeminiDriver;
pub use openai::OpenAIDriver;
pub use openai_compatible::OpenAICompatibleDriver;

use crate::error::ModelError;
use crate::request::{ModelMessage, ModelRequest};
use crate::transport::HttpResponse;

/// Shared helpers for JSON-wire adapters.
pub(crate) mod wire {
    use super::*;

    /// Maps an HTTP error status + provider error body to a normalized
    /// [`ModelError`]. Shared heuristics; adapters refine with
    /// provider-specific codes.
    pub(crate) fn map_http_error(adapter: &str, response: &HttpResponse) -> ModelError {
        let body = &response.body;
        let code = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| {
                let error = v.get("error")?;
                error
                    .get("code")
                    .and_then(|c| c.as_str().map(str::to_owned))
                    .or_else(|| {
                        error
                            .get("type")
                            .and_then(|t| t.as_str().map(str::to_owned))
                    })
            });
        let message = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.get("message").and_then(|m| m.as_str().map(str::to_owned)))
            })
            .unwrap_or_else(|| body.chars().take(200).collect());

        match response.status {
            429 => ModelError::rate_limit(format!("{adapter} rate limited: {message}")),
            401 | 403 => ModelError::auth(format!("{adapter} auth rejected: {message}")),
            500..=599 => ModelError::unavailable(format!("{adapter} server error: {message}")),
            400 if message.to_lowercase().contains("context length")
                || message.to_lowercase().contains("context_length")
                || code.as_deref() == Some("context_length_exceeded") =>
            {
                ModelError::context_limit(format!("{adapter}: {message}"))
            }
            400..=499 => {
                let mut e = ModelError::new(
                    lumi_protocol::FailureCategory::ModelReasoning,
                    format!("{adapter} rejected request: {message}"),
                    false,
                );
                e.native_code = code;
                e
            }
            status => {
                ModelError::unavailable(format!("{adapter} unexpected status {status}: {message}"))
            }
        }
    }

    /// Parses a successful response body as JSON.
    pub(crate) fn parse_json(
        response: &HttpResponse,
        adapter: &str,
    ) -> Result<serde_json::Value, ModelError> {
        serde_json::from_str(&response.body)
            .map_err(|e| ModelError::format(format!("{adapter} malformed JSON body: {e}")))
    }

    /// Ensures the messages list is non-empty (providers reject it).
    pub(crate) fn validate_request(
        request: &ModelRequest,
        adapter: &str,
    ) -> Result<(), ModelError> {
        if request.messages.is_empty() {
            return Err(ModelError::format(format!(
                "{adapter}: request has no messages"
            )));
        }
        for message in &request.messages {
            if let ModelMessage::User { content } = message {
                if content.is_empty() {
                    return Err(ModelError::format(format!("{adapter}: empty user message")));
                }
            }
        }
        Ok(())
    }
}
