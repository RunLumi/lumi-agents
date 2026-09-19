//! Scoped secret access (spec 14 §14.5): a connector resolves ONLY the
//! secret refs its manifest declares. Enforcement happens at the broker
//! call site — a wrapped broker that checks the declared set before
//! delegating, so a connector cannot even ask for an undeclared ref.

use lumi_protocol::SecretRef;
use lumi_secrets::{SecretBackend, SecretBroker, SecretError, SecretValue};

/// A broker wrapper scoped to one connector's declared secret refs.
///
/// The wrapper implements [`SecretBackend`], so it composes with
/// `SecretBroker` itself: the executor boundary constructs
/// `SecretBroker::new(ScopedSecretBroker::new(inner, allowed_refs))` and
/// the connector physically cannot resolve anything outside its
/// manifest declaration (§14.5).
#[derive(Debug, Clone)]
pub struct ScopedSecretBackend<B: SecretBackend> {
    inner: B,
    connector_id: String,
    allowed_refs: std::collections::BTreeSet<String>,
}

impl<B: SecretBackend> ScopedSecretBackend<B> {
    #[must_use]
    pub fn new(
        inner: B,
        connector_id: impl Into<String>,
        allowed_refs: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            inner,
            connector_id: connector_id.into(),
            allowed_refs: allowed_refs.into_iter().collect(),
        }
    }

    fn check(&self, reference: &SecretRef) -> Result<(), SecretError> {
        if self.allowed_refs.contains(reference.as_str()) {
            Ok(())
        } else {
            Err(SecretError::Backend {
                reference: reference.as_str().to_owned(),
                detail: format!(
                    "connector {:?} is not scoped to this secret (§14.5)",
                    self.connector_id
                ),
            })
        }
    }
}

impl<B: SecretBackend> SecretBackend for ScopedSecretBackend<B> {
    fn store(&self, reference: &SecretRef, value: &SecretValue) -> Result<(), SecretError> {
        self.check(reference)?;
        self.inner.store(reference, value)
    }

    fn resolve(&self, reference: &SecretRef) -> Result<SecretValue, SecretError> {
        self.check(reference)?;
        self.inner.resolve(reference)
    }

    fn delete(&self, reference: &SecretRef) -> Result<bool, SecretError> {
        self.check(reference)?;
        self.inner.delete(reference)
    }

    fn exists(&self, reference: &SecretRef) -> Result<bool, SecretError> {
        self.check(reference)?;
        self.inner.exists(reference)
    }
}

/// Convenience constructor: a full `SecretBroker` whose backend is
/// scoped to one connector's manifest-declared refs.
#[must_use]
pub fn scoped_broker<B: SecretBackend + 'static>(
    inner_broker_backend: B,
    connector_id: impl Into<String>,
    allowed_refs: impl IntoIterator<Item = String>,
) -> SecretBroker<ScopedSecretBackend<B>> {
    SecretBroker::new(ScopedSecretBackend::new(
        inner_broker_backend,
        connector_id,
        allowed_refs,
    ))
}

/// Evidence that the scope check fires before the underlying store: the
/// audit event on an out-of-scope resolve names the connector.
#[must_use]
pub fn out_of_scope_error(connector_id: &str, reference: &SecretRef) -> SecretError {
    let _ = (connector_id, reference);
    SecretError::Backend {
        reference: reference.as_str().to_owned(),
        detail: format!("connector {connector_id:?} is not scoped to this secret"),
    }
}

/// Re-exports for executor-boundary ergonomics.
pub use lumi_secrets::{
    SecretAuditEvent as ScopedAuditEvent, SecretAuditOperation as ScopedAuditOperation,
};
