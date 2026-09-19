//! Normalized model request/response and errors (spec 09 §9.5–9.6, §9.12).

use lumi_protocol::FailureCategory;

/// Transport-level failure (no HTTP status semantics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// Request timed out.
    Timeout,
    /// Connection failed.
    Connect(String),
    /// Response was not valid HTTP/JSON framing.
    Malformed(String),
    /// The transport has no implementation compiled in.
    NoTransport,
}

/// Normalized model error (spec 09 §9.6 retryability + §9.12 distinct
/// context-limit errors). Category maps onto the canonical failure
/// taxonomy so recovery is uniform with executors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelError {
    pub category: FailureCategory,
    pub message: String,
    pub retryable: bool,
    /// Provider-native error code, e.g. `insufficient_quota`.
    pub native_code: Option<String>,
}

impl ModelError {
    #[must_use]
    pub fn new(category: FailureCategory, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            category,
            message: message.into(),
            retryable,
            native_code: None,
        }
    }

    #[must_use]
    pub fn context_limit(message: impl Into<String>) -> Self {
        Self::new(FailureCategory::ModelContextLimit, message, false)
    }

    #[must_use]
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::new(FailureCategory::ProviderRateLimit, message, true)
    }

    #[must_use]
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(FailureCategory::ProviderUnavailable, message, true)
    }

    #[must_use]
    pub fn auth(message: impl Into<String>) -> Self {
        Self::new(FailureCategory::AuthSession, message, false)
    }

    #[must_use]
    pub fn format(message: impl Into<String>) -> Self {
        Self::new(FailureCategory::ModelFormat, message, true)
    }

    #[must_use]
    pub fn refusal(message: impl Into<String>) -> Self {
        Self::new(FailureCategory::ModelRefusal, message, false)
    }

    #[must_use]
    pub fn timeout() -> Self {
        Self::new(FailureCategory::Network, "model request timed out", true)
    }

    #[must_use]
    pub fn connect(message: impl Into<String>) -> Self {
        Self::new(FailureCategory::Network, message, true)
    }
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.category.as_str(), self.message)
    }
}

impl std::error::Error for ModelError {}
