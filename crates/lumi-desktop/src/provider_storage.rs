//! Opt-in provider recovery through the existing OS-backed broker.
//! Endpoint, model and key are one protected tuple, never split between
//! editable project metadata and a saved key that could be redirected.
use crate::ProviderSession;
use lumi_protocol::SecretRef;
use lumi_secrets::{SecretBackend, SecretBroker, SecretError, SecretValue};
use serde::{Deserialize, Serialize};

const REFERENCE: &str = "providers/desktop-default/v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedProvider {
    version: u32,
    family: String,
    endpoint: String,
    model: String,
    api_key: String,
}

pub fn validate_provider(session: &ProviderSession) -> Result<(), String> {
    if !matches!(session.family.as_str(), "openai" | "openai-compatible") {
        return Err("unsupported provider family".into());
    }
    if session.model.trim().is_empty()
        || session.model.len() > 256
        || session.api_key.trim().is_empty()
        || session.api_key.len() > 8192
        || session.api_key.chars().any(char::is_control)
        || session.endpoint.len() > 2048
    {
        return Err("provider configuration is empty, oversized or malformed".into());
    }
    let url = url::Url::parse(&session.endpoint).map_err(|_| "invalid provider endpoint")?;
    let loopback = matches!(
        url.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
    );
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
        || !(url.scheme() == "https" || url.scheme() == "http" && loopback)
    {
        return Err("provider endpoint requires HTTPS, or loopback HTTP, without URL credentials, query or fragment".into());
    }
    Ok(())
}

pub fn remember_provider<B: SecretBackend>(
    broker: &SecretBroker<B>,
    session: &ProviderSession,
) -> Result<(), String> {
    validate_provider(session)?;
    let saved = SavedProvider {
        version: 1,
        family: session.family.clone(),
        endpoint: session.endpoint.clone(),
        model: session.model.clone(),
        api_key: session.api_key.clone(),
    };
    let encoded =
        serde_json::to_string(&saved).map_err(|_| "could not encode provider configuration")?;
    broker.store(&SecretRef(REFERENCE.into()), &SecretValue::new(encoded))
        .map_err(|_| "OS credential store could not save the provider; configuration was not marked remembered".into())
}

pub fn restore_provider<B: SecretBackend>(
    broker: &SecretBroker<B>,
) -> Result<Option<ProviderSession>, String> {
    let value = match broker.resolve(&SecretRef(REFERENCE.into()), "restore-desktop-provider") {
        Ok(value) => value,
        Err(SecretError::NotFound { .. }) => return Ok(None),
        Err(_) => return Err("OS credential store is unavailable; reconnect the provider".into()),
    };
    if value.expose().len() > 16_384 {
        return Err("saved provider configuration exceeds its size limit".into());
    }
    let saved: SavedProvider = serde_json::from_str(value.expose())
        .map_err(|_| "saved provider configuration is invalid; reconnect the provider")?;
    if saved.version != 1 {
        return Err("unsupported saved provider version".into());
    }
    let session = ProviderSession {
        family: saved.family,
        endpoint: saved.endpoint,
        model: saved.model,
        api_key: saved.api_key,
    };
    validate_provider(&session)?;
    Ok(Some(session))
}

pub fn forget_provider<B: SecretBackend>(broker: &SecretBroker<B>) -> Result<(), String> {
    broker
        .revoke(&SecretRef(REFERENCE.into()))
        .map(|_| ())
        .map_err(|_| {
            "OS credential store could not remove the saved provider; removal is not confirmed"
                .into()
        })
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn remember_os_provider(session: &ProviderSession) -> Result<(), String> {
    remember_provider(
        &SecretBroker::new(lumi_secrets::KeyringBackend::new()),
        session,
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn remember_os_provider(_session: &ProviderSession) -> Result<(), String> {
    Err("secure provider persistence is unavailable on this platform".into())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn restore_os_provider() -> Result<Option<ProviderSession>, String> {
    restore_provider(&SecretBroker::new(lumi_secrets::KeyringBackend::new()))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn restore_os_provider() -> Result<Option<ProviderSession>, String> {
    Err("secure provider persistence is unavailable on this platform".into())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn forget_os_provider() -> Result<(), String> {
    forget_provider(&SecretBroker::new(lumi_secrets::KeyringBackend::new()))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn forget_os_provider() -> Result<(), String> {
    Err("secure provider persistence is unavailable on this platform".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_secrets::InMemorySecretBackend;

    fn session() -> ProviderSession {
        ProviderSession {
            family: "openai-compatible".into(),
            endpoint: "https://models.example/v1".into(),
            model: "fixture".into(),
            api_key: ["fixture", "not", "a", "real", "key"].concat(),
        }
    }

    #[test]
    fn protected_tuple_roundtrips_and_forget_revokes_it() {
        let broker = SecretBroker::new(InMemorySecretBackend::default());
        assert!(restore_provider(&broker).unwrap().is_none());
        let expected = session();
        remember_provider(&broker, &expected).unwrap();
        assert_eq!(restore_provider(&broker).unwrap(), Some(expected.clone()));
        assert!(!format!("{:?}", broker.audit_trail()).contains(&expected.api_key));
        assert!(!format!("{expected:?}").contains(&expected.api_key));
        forget_provider(&broker).unwrap();
        assert!(restore_provider(&broker).unwrap().is_none());
    }

    #[test]
    fn corrupt_store_does_not_leak_secret_content_in_errors() {
        let broker = SecretBroker::new(InMemorySecretBackend::default());
        broker
            .store(
                &SecretRef(REFERENCE.into()),
                &SecretValue::new("private-unparseable-material"),
            )
            .unwrap();
        let error = restore_provider(&broker).unwrap_err();
        assert!(!error.contains("private-unparseable-material"));
    }

    #[test]
    fn remote_plaintext_and_url_credentials_are_refused() {
        for endpoint in [
            "http://remote.example",
            "https://user:pass@example.com",
            "https://example.com?key=x",
            "file:///tmp/key",
        ] {
            let mut value = session();
            value.endpoint = endpoint.into();
            assert!(validate_provider(&value).is_err(), "{endpoint}");
        }
        let mut value = session();
        value.endpoint = "http://127.0.0.1:11434/v1".into();
        assert!(validate_provider(&value).is_ok());
    }
}
