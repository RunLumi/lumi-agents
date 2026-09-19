//! In-memory backend: reference semantics for tests and hermetic CI.
//! Values are held in zeroizing storage and never serialized.

use crate::broker::{SecretBackend, SecretError};
use crate::value::SecretValue;
use lumi_protocol::SecretRef;
use std::collections::HashMap;
use std::sync::Mutex;

/// Thread-safe in-memory secret store.
#[derive(Debug, Default)]
pub struct InMemorySecretBackend {
    secrets: Mutex<HashMap<SecretRef, SecretValue>>,
}

impl InMemorySecretBackend {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretBackend for InMemorySecretBackend {
    fn store(&self, reference: &SecretRef, value: &SecretValue) -> Result<(), SecretError> {
        self.secrets
            .lock()
            .map_err(|_| SecretError::Backend {
                reference: reference.as_str().to_owned(),
                detail: "poisoned store".to_owned(),
            })?
            .insert(reference.clone(), value.clone());
        Ok(())
    }

    fn resolve(&self, reference: &SecretRef) -> Result<SecretValue, SecretError> {
        self.secrets
            .lock()
            .map_err(|_| SecretError::Backend {
                reference: reference.as_str().to_owned(),
                detail: "poisoned store".to_owned(),
            })?
            .get(reference)
            .cloned()
            .ok_or_else(|| SecretError::NotFound {
                reference: reference.as_str().to_owned(),
            })
    }

    fn delete(&self, reference: &SecretRef) -> Result<bool, SecretError> {
        let mut secrets = self.secrets.lock().map_err(|_| SecretError::Backend {
            reference: reference.as_str().to_owned(),
            detail: "poisoned store".to_owned(),
        })?;
        Ok(secrets.remove(reference).is_some())
    }

    fn exists(&self, reference: &SecretRef) -> Result<bool, SecretError> {
        let secrets = self.secrets.lock().map_err(|_| SecretError::Backend {
            reference: reference.as_str().to_owned(),
            detail: "poisoned store".to_owned(),
        })?;
        Ok(secrets.contains_key(reference))
    }
}
