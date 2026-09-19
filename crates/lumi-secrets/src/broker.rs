//! The broker: audited, backend-agnostic secret operations.

use crate::value::SecretValue;
use lumi_protocol::{SecretRef, Timestamp};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Storage/lookup backend. Values cross this boundary zeroized-on-drop;
/// implementations MUST NOT log or persist them outside their store.
pub trait SecretBackend: std::fmt::Debug + Send + Sync {
    fn store(&self, reference: &SecretRef, value: &SecretValue) -> Result<(), SecretError>;
    fn resolve(&self, reference: &SecretRef) -> Result<SecretValue, SecretError>;
    /// Removes the secret. Returns true when it existed (revocation
    /// evidence), false when it was already absent.
    fn delete(&self, reference: &SecretRef) -> Result<bool, SecretError>;
    fn exists(&self, reference: &SecretRef) -> Result<bool, SecretError>;
}

/// Broker failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// The reference is not stored.
    NotFound { reference: String },
    /// The reference violates the ref-name contract (empty, control
    /// characters, or oversized — refs are auditable identifiers).
    InvalidRef { reference: String, reason: String },
    /// The backend failed.
    Backend { reference: String, detail: String },
}

impl SecretError {
    #[must_use]
    pub const fn category(&self) -> lumi_protocol::FailureCategory {
        match self {
            // A missing/invalid secret is an auth-session problem the
            // retry layer maps to REFRESH_AUTH.
            Self::NotFound { .. } | Self::InvalidRef { .. } => {
                lumi_protocol::FailureCategory::AuthSession
            }
            Self::Backend { .. } => lumi_protocol::FailureCategory::UpstreamDriver,
        }
    }
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { reference } => write!(f, "secret not found: {reference:?}"),
            Self::InvalidRef { reference, reason } => {
                write!(f, "invalid secret ref {reference:?}: {reason}")
            }
            Self::Backend { reference, detail } => {
                write!(f, "secret backend error for {reference:?}: {detail}")
            }
        }
    }
}

impl std::error::Error for SecretError {}

/// What the broker did (audit record — contains the REF, never a value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretAuditEvent {
    pub reference: String,
    pub operation: SecretAuditOperation,
    pub purpose: String,
    pub succeeded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretAuditOperation {
    Store,
    Resolve,
    Delete,
    Exists,
}

/// The audit sink signature: receives the event (reference only, never
/// a value) for forwarding into the evidence pipeline.
pub type AuditSink = Box<dyn Fn(&SecretAuditEvent) + Send + Sync>;

/// The secrets broker: validates references, delegates to the backend,
/// and records an audit event for every operation.
///
/// The audit trail is append-only in memory and additionally emitted
/// through the optional [`AuditSink`] callback (e.g. into the
/// `lumi-audit` evidence pipeline). Events never contain values.
pub struct SecretBroker<B: SecretBackend> {
    backend: B,
    audit_log: Mutex<Vec<SecretAuditEvent>>,
    audit_sink: Option<AuditSink>,
}

impl<B: SecretBackend> SecretBroker<B> {
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            audit_log: Mutex::new(Vec::new()),
            audit_sink: None,
        }
    }

    /// Installs an audit sink (e.g. forwarding into the evidence
    /// pipeline). Events contain references only.
    pub fn with_audit_sink(mut self, sink: Box<dyn Fn(&SecretAuditEvent) + Send + Sync>) -> Self {
        self.audit_sink = Some(sink);
        self
    }

    /// Validates the reference name (refs are auditable identifiers:
    /// non-empty, bounded, printable).
    fn validate(reference: &SecretRef) -> Result<(), SecretError> {
        let name = reference.as_str();
        if name.is_empty() || name.len() > 256 {
            return Err(SecretError::InvalidRef {
                reference: name.to_owned(),
                reason: "ref must be 1..=256 characters".to_owned(),
            });
        }
        if name.chars().any(|c| c.is_control()) {
            return Err(SecretError::InvalidRef {
                reference: name.to_owned(),
                reason: "ref must not contain control characters".to_owned(),
            });
        }
        Ok(())
    }

    fn audit(&self, event: SecretAuditEvent) {
        if let Some(sink) = &self.audit_sink {
            sink(&event);
        }
        if let Ok(mut log) = self.audit_log.lock() {
            log.push(event);
        }
    }

    /// Stores/overwrites the secret for `reference`.
    ///
    /// # Errors
    /// [`SecretError`] on invalid refs or backend failure.
    pub fn store(&self, reference: &SecretRef, value: &SecretValue) -> Result<(), SecretError> {
        Self::validate(reference)?;
        let result = self.backend.store(reference, value);
        self.audit(SecretAuditEvent {
            reference: reference.as_str().to_owned(),
            operation: SecretAuditOperation::Store,
            purpose: "store".to_owned(),
            succeeded: result.is_ok(),
            detail: result.as_ref().err().map(|e| e.to_string()),
            at: Timestamp::now(),
        });
        result
    }

    /// Resolves a secret at the executor boundary. `purpose` is recorded
    /// in the audit event (e.g. `"cua-driver-auth"`).
    ///
    /// # Errors
    /// [`SecretError::NotFound`] when absent; backend failures otherwise.
    pub fn resolve(
        &self,
        reference: &SecretRef,
        purpose: &str,
    ) -> Result<SecretValue, SecretError> {
        Self::validate(reference)?;
        let result = self.backend.resolve(reference);
        self.audit(SecretAuditEvent {
            reference: reference.as_str().to_owned(),
            operation: SecretAuditOperation::Resolve,
            purpose: purpose.to_owned(),
            succeeded: result.is_ok(),
            detail: result.as_ref().err().map(|e| e.to_string()),
            at: Timestamp::now(),
        });
        result
    }

    /// Revokes the secret. Returns true when it existed.
    ///
    /// # Errors
    /// [`SecretError`] on invalid refs or backend failure.
    pub fn revoke(&self, reference: &SecretRef) -> Result<bool, SecretError> {
        Self::validate(reference)?;
        let result = self.backend.delete(reference);
        self.audit(SecretAuditEvent {
            reference: reference.as_str().to_owned(),
            operation: SecretAuditOperation::Delete,
            purpose: "revoke".to_owned(),
            succeeded: result.is_ok(),
            detail: result.as_ref().err().map(|e| e.to_string()),
            at: Timestamp::now(),
        });
        result
    }

    /// Existence probe (does not resolve the value).
    ///
    /// # Errors
    /// [`SecretError`] on invalid refs or backend failure.
    pub fn exists(&self, reference: &SecretRef) -> Result<bool, SecretError> {
        Self::validate(reference)?;
        let result = self.backend.exists(reference);
        self.audit(SecretAuditEvent {
            reference: reference.as_str().to_owned(),
            operation: SecretAuditOperation::Exists,
            purpose: "exists".to_owned(),
            succeeded: result.is_ok(),
            detail: result.as_ref().err().map(|e| e.to_string()),
            at: Timestamp::now(),
        });
        result
    }

    /// The in-memory audit trail (refs and outcomes only).
    #[must_use]
    pub fn audit_trail(&self) -> Vec<SecretAuditEvent> {
        self.audit_log
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }
}
