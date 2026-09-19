//! Error taxonomy, retry classes, and the canonical error envelope
//! (spec 18, spec 03 §3.8).
//!
//! Adapter/provider-native raw errors MUST NOT be the only persisted
//! failure representation: every failure is normalized into an
//! [`ErrorEnvelope`] carrying a stable [`FailureCategory`] and a
//! [`RecoveryAction`] hint. Native diagnostics MAY ride along.

use crate::canonical::{canonical_json, sha256_hex};
use serde::{Deserialize, Serialize};

/// Stable failure categories (spec 18 §18.2). Unknown values fail on
/// deserialization rather than coercing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureCategory {
    ModelReasoning,
    ModelFormat,
    ModelContextLimit,
    ModelRefusal,
    ProviderRateLimit,
    ProviderUnavailable,
    PolicyDenyExpected,
    PolicyBug,
    ApprovalTimeout,
    ApprovalInvalid,
    ConnectorFailure,
    BrowserSelector,
    BrowserState,
    NativeElement,
    NativeSession,
    VisionGrounding,
    Filesystem,
    ShellExecution,
    OsPermission,
    AuthSession,
    UpstreamDriver,
    Network,
    Postcondition,
    AmbiguousState,
    BudgetExceeded,
    Crash,
    UserCancel,
    VersionIncompatible,
    DataPolicy,
    SecurityViolation,
}

impl FailureCategory {
    /// Default recovery hint for this category (spec 18 §18.3). Workflows
    /// MAY narrow this (never widen automatic retries for consequential
    /// side effects).
    #[must_use]
    pub const fn default_recovery(self) -> RecoveryAction {
        match self {
            Self::ModelReasoning
            | Self::ModelFormat
            | Self::ProviderRateLimit
            | Self::ProviderUnavailable
            | Self::ConnectorFailure
            | Self::Network => RecoveryAction::RetryBackoff,
            Self::ModelContextLimit | Self::ModelRefusal => RecoveryAction::RequireUser,
            Self::PolicyDenyExpected => RecoveryAction::RequireUser,
            Self::PolicyBug
            | Self::ApprovalInvalid
            | Self::Filesystem
            | Self::ShellExecution
            | Self::BudgetExceeded
            | Self::Crash
            | Self::UserCancel
            | Self::VersionIncompatible
            | Self::DataPolicy
            | Self::SecurityViolation => RecoveryAction::Terminal,
            Self::ApprovalTimeout | Self::OsPermission => RecoveryAction::RequireUser,
            Self::BrowserSelector
            | Self::BrowserState
            | Self::NativeElement
            | Self::NativeSession
            | Self::VisionGrounding => RecoveryAction::Reobserve,
            Self::AuthSession => RecoveryAction::RefreshAuth,
            Self::UpstreamDriver => RecoveryAction::RetryImmediate,
            Self::Postcondition | Self::AmbiguousState => RecoveryAction::AmbiguousNoRetry,
        }
    }

    /// Maps an adapter-native diagnostic to the canonical category.
    ///
    /// This is the single registration point adapters use so fixture tests
    /// can prove native-to-canonical mapping (spec 03 §3.12).
    #[must_use]
    pub fn from_adapter(adapter: &str, native_code: &str) -> Self {
        match (adapter, native_code) {
            ("playwright", "TimeoutError") => Self::BrowserState,
            ("playwright", "LocatorNotFound") => Self::BrowserSelector,
            ("playwright", "TargetClosed") => Self::BrowserState,
            ("playwright", "NavigationInterrupted") => Self::Network,
            ("cua", "ElementNotFound") => Self::NativeElement,
            ("cua", "SessionLost") => Self::NativeSession,
            ("cua", "PermissionDenied") => Self::OsPermission,
            ("http", "401") | ("http", "403") => Self::AuthSession,
            ("http", "429") => Self::ProviderRateLimit,
            ("http", "5xx") => Self::ProviderUnavailable,
            ("shell", "Timeout") => Self::ShellExecution,
            ("shell", "SpawnFailed") => Self::ShellExecution,
            ("fs", "NotFound") => Self::Filesystem,
            ("fs", "PermissionDenied") => Self::OsPermission,
            _ => Self::ConnectorFailure,
        }
    }
}

/// Recovery behavior selected for a failure (spec 18 §18.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecoveryAction {
    /// Retry now (short transient blips, driver restart).
    RetryImmediate,
    /// Retry with bounded exponential/jittered backoff.
    RetryBackoff,
    /// Refresh authentication, then retry once.
    RefreshAuth,
    /// Re-observe external state before doing anything else.
    Reobserve,
    /// Route to human approval.
    RequireApproval,
    /// Route to the human exception queue.
    RequireUser,
    /// Terminal: do not retry.
    Terminal,
    /// State may or may not have taken effect: verify external postcondition
    /// first; automatic retry is FORBIDDEN.
    AmbiguousNoRetry,
}

/// Retry classification of a step/action (spec 02 §2.11). Distinct from
/// [`RecoveryAction`]: this class gates *automatic* retry loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetryClass {
    /// Safe to auto-retry with backoff.
    Transient,
    /// Rate limited; honor Retry-After/backoff.
    RateLimit,
    /// Refreshable authentication; refresh then retry once.
    AuthRefreshable,
    /// Observed state went stale; re-observe then retry.
    StaleState,
    /// Do not retry automatically.
    NonRetryable,
    /// A consequential side effect may have happened; MUST verify external
    /// state before any retry decision.
    AmbiguousSideEffect,
}

impl RetryClass {
    /// Only retryable classes MAY auto-retry (spec 02 §2.11).
    #[must_use]
    pub const fn allows_auto_retry(self) -> bool {
        matches!(
            self,
            Self::Transient | Self::RateLimit | Self::AuthRefreshable | Self::StaleState
        )
    }

    /// Retry class implied by a failure category.
    #[must_use]
    pub const fn from_failure(category: FailureCategory) -> Self {
        match category {
            FailureCategory::ProviderRateLimit => Self::RateLimit,
            FailureCategory::AuthSession => Self::AuthRefreshable,
            FailureCategory::BrowserSelector
            | FailureCategory::BrowserState
            | FailureCategory::NativeElement
            | FailureCategory::VisionGrounding => Self::StaleState,
            FailureCategory::ModelReasoning
            | FailureCategory::ModelFormat
            | FailureCategory::ProviderUnavailable
            | FailureCategory::ConnectorFailure
            | FailureCategory::Network
            | FailureCategory::UpstreamDriver => Self::Transient,
            FailureCategory::AmbiguousState | FailureCategory::Postcondition => {
                Self::AmbiguousSideEffect
            }
            _ => Self::NonRetryable,
        }
    }
}

/// Canonical error envelope (spec 03 §3.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub category: FailureCategory,
    /// Human-readable, secret-free summary.
    pub message: String,
    /// Adapter identity, e.g. `playwright/1.40`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_adapter: Option<String>,
    /// Adapter-native error code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_code: Option<String>,
    /// Structured adapter diagnostics. Field-level redaction applies before
    /// persistence/egress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_detail: Option<serde_json::Value>,
    /// Recovery hint; defaults to the category mapping and may be narrowed
    /// by workflow policy.
    pub recovery: RecoveryAction,
    /// Retry classification derived from the category.
    pub retry_class: RetryClass,
}

impl ErrorEnvelope {
    /// Builds an envelope with the category's default recovery hints.
    #[must_use]
    pub fn new(category: FailureCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
            native_adapter: None,
            native_code: None,
            native_detail: None,
            recovery: category.default_recovery(),
            retry_class: RetryClass::from_failure(category),
        }
    }

    /// Builds from adapter-native diagnostics, normalizing the category.
    #[must_use]
    pub fn from_adapter(adapter: &str, native_code: &str, message: impl Into<String>) -> Self {
        let category = FailureCategory::from_adapter(adapter, native_code);
        let mut envelope = Self::new(category, message);
        envelope.native_adapter = Some(adapter.to_owned());
        envelope.native_code = Some(native_code.to_owned());
        envelope
    }

    /// Attaches structured native diagnostics.
    #[must_use]
    pub fn with_detail(mut self, detail: serde_json::Value) -> Self {
        self.native_detail = Some(detail);
        self
    }

    /// Stable digest of the envelope for audit correlation. Canonical JSON
    /// keeps this deterministic across processes.
    #[must_use]
    pub fn digest(&self) -> String {
        let value = serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({
                "category": self.category,
                "message": self.message,
            })
        });
        sha256_hex(canonical_json(&value).as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_recovery_table() {
        assert_eq!(
            FailureCategory::ProviderRateLimit.default_recovery(),
            RecoveryAction::RetryBackoff
        );
        assert_eq!(
            FailureCategory::AmbiguousState.default_recovery(),
            RecoveryAction::AmbiguousNoRetry
        );
        assert_eq!(
            FailureCategory::BrowserSelector.default_recovery(),
            RecoveryAction::Reobserve
        );
        assert_eq!(
            FailureCategory::PolicyBug.default_recovery(),
            RecoveryAction::Terminal
        );
        assert_eq!(
            FailureCategory::AuthSession.default_recovery(),
            RecoveryAction::RefreshAuth
        );
    }

    #[test]
    fn adapter_mapping_fixtures() {
        assert_eq!(
            FailureCategory::from_adapter("playwright", "TimeoutError"),
            FailureCategory::BrowserState
        );
        assert_eq!(
            FailureCategory::from_adapter("playwright", "LocatorNotFound"),
            FailureCategory::BrowserSelector
        );
        assert_eq!(
            FailureCategory::from_adapter("cua", "ElementNotFound"),
            FailureCategory::NativeElement
        );
        assert_eq!(
            FailureCategory::from_adapter("http", "429"),
            FailureCategory::ProviderRateLimit
        );
    }

    #[test]
    fn ambiguous_side_effects_never_auto_retry() {
        let ambiguous = [
            FailureCategory::AmbiguousState,
            FailureCategory::Postcondition,
        ];
        for category in ambiguous {
            let envelope = ErrorEnvelope::new(category, "submit timed out");
            assert_eq!(envelope.recovery, RecoveryAction::AmbiguousNoRetry);
            assert_eq!(envelope.retry_class, RetryClass::AmbiguousSideEffect);
            assert!(!envelope.retry_class.allows_auto_retry());
        }
    }

    #[test]
    fn retry_classes_gate_auto_retry() {
        assert!(RetryClass::Transient.allows_auto_retry());
        assert!(RetryClass::RateLimit.allows_auto_retry());
        assert!(RetryClass::AuthRefreshable.allows_auto_retry());
        assert!(RetryClass::StaleState.allows_auto_retry());
        assert!(!RetryClass::NonRetryable.allows_auto_retry());
        assert!(!RetryClass::AmbiguousSideEffect.allows_auto_retry());
    }

    #[test]
    fn envelope_digest_is_stable() {
        let a = ErrorEnvelope::from_adapter("playwright", "TimeoutError", "timeout").with_detail(
            serde_json::json!({"url": "https://example.test/submit", "timeout_ms": 5000}),
        );
        let b = ErrorEnvelope::from_adapter("playwright", "TimeoutError", "timeout").with_detail(
            serde_json::json!({"timeout_ms": 5000, "url": "https://example.test/submit"}),
        );
        // Key order in the detail object must not change the digest.
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn envelope_serde_round_trip() {
        let e = ErrorEnvelope::from_adapter("cua", "SessionLost", "driver session ended");
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("NATIVE_SESSION"));
        let back: ErrorEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }
}
