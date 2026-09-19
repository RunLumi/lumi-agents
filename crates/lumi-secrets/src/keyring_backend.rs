//! OS keychain backend (macOS Keychain / Windows Credential Manager) via
//! the `keyring` crate. Compiled only on those platforms; Linux remains
//! out of v1 product scope (spec 07 §7.3).

use crate::broker::{SecretBackend, SecretError};
use crate::value::SecretValue;
use lumi_protocol::SecretRef;

/// Keychain-backed store. All refs live under one Lumi service name so
/// revocation/cleanup can enumerate them by prefix.
#[derive(Debug, Clone)]
pub struct KeyringBackend {
    service: String,
}

impl KeyringBackend {
    #[must_use]
    pub fn new() -> Self {
        Self {
            service: "lumi-agents".to_owned(),
        }
    }

    #[must_use]
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, reference: &SecretRef) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(&self.service, reference.as_str()).map_err(|e| SecretError::Backend {
            reference: reference.as_str().to_owned(),
            detail: format!("keyring entry: {e}"),
        })
    }

    fn map(reference: &SecretRef, e: keyring::Error) -> SecretError {
        match e {
            keyring::Error::NoEntry => SecretError::NotFound {
                reference: reference.as_str().to_owned(),
            },
            other => SecretError::Backend {
                reference: reference.as_str().to_owned(),
                detail: other.to_string(),
            },
        }
    }
}

impl Default for KeyringBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretBackend for KeyringBackend {
    fn store(&self, reference: &SecretRef, value: &SecretValue) -> Result<(), SecretError> {
        self.entry(reference)?
            .set_password(value.expose())
            .map_err(|e| Self::map(reference, e))
    }

    fn resolve(&self, reference: &SecretRef) -> Result<SecretValue, SecretError> {
        let password = self
            .entry(reference)?
            .get_password()
            .map_err(|e| Self::map(reference, e))?;
        Ok(SecretValue::new(password))
    }

    fn delete(&self, reference: &SecretRef) -> Result<bool, SecretError> {
        match self.entry(reference)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(Self::map(reference, e)),
        }
    }

    fn exists(&self, reference: &SecretRef) -> Result<bool, SecretError> {
        match self.entry(reference)?.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(Self::map(reference, e)),
        }
    }
}
