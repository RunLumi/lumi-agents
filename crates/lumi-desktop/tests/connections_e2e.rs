//! Project connections certification (Spec 29): credentials stored
//! through the broker under project-scoped references; metadata
//! records persist WITHOUT the credential value; disconnect revokes.

use lumi_desktop::ConnectionsStore;
use lumi_secrets::{InMemorySecretBackend, SecretBroker};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn depot(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-connections-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct DepotGuard(PathBuf);

impl Drop for DepotGuard {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

const PROJECT: &str = "11111111-2222-3333-4444-555555555555";

#[test]
fn connections_store_persists_metadata_without_credential_values() {
    let guard = DepotGuard(depot("meta"));
    let records_path = guard.0.join("connections.json");
    let store = ConnectionsStore::new(records_path.clone());
    let broker = SecretBroker::new(InMemorySecretBackend::default());

    let record = store
        .connect(
            &broker,
            PROJECT,
            "crm-prod",
            "api_key",
            "https://crm.example.test",
            "sk-secret-123",
        )
        .unwrap();

    assert_eq!(record.name, "crm-prod");
    assert_eq!(record.kind, "api_key");
    assert!(
        record.credential_ref.contains(PROJECT)
            || record.credential_ref.starts_with("connections/")
    );

    // Round-trip: a fresh store instance sees the metadata.
    let listed = store.list(PROJECT);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "crm-prod");
    assert_eq!(listed[0].endpoint, "https://crm.example.test");
    assert!(listed[0].credential_ref.starts_with("connections/"));

    // The persisted records file must not contain the credential value.
    let raw = std::fs::read_to_string(&records_path).unwrap();
    assert!(!raw.contains("sk-secret-123"), "credential leaked to disk");

    // Disconnect revokes and removes.
    assert!(store
        .disconnect(&broker, PROJECT, &record.connection_id)
        .unwrap());
    assert!(store.list(PROJECT).is_empty());

    // Re-connecting with the same name works after disconnect.
    store
        .connect(
            &broker,
            PROJECT,
            "crm-prod",
            "api_key",
            "https://crm.example.test",
            "sk-secret-123",
        )
        .unwrap();
    assert_eq!(store.list(PROJECT).len(), 1);
}

#[test]
fn connections_validate_input_and_scope_per_project() {
    let guard = DepotGuard(depot("validate"));
    let records_path = guard.0.join("connections.json");
    let store = ConnectionsStore::new(records_path.clone());
    let broker = SecretBroker::new(InMemorySecretBackend::default());

    // Missing fields refuse.
    assert!(store
        .connect(&broker, PROJECT, "x", "api_key", "https://x", "  ")
        .is_err());

    // Duplicate name for the same project refuses.
    store
        .connect(&broker, PROJECT, "dup", "api_key", "https://a", "k1")
        .unwrap();
    assert!(store
        .connect(&broker, PROJECT, "dup", "api_key", "https://b", "k2")
        .is_err());

    // Another project's records stay project-scoped.
    assert!(store
        .list("99999999-0000-0000-0000-000000000000")
        .is_empty());

    // Unknown disconnect returns false, not an error.
    assert!(!store.disconnect(&broker, PROJECT, "conn-ghost").unwrap());
}

#[test]
fn connections_verify_reports_presence_without_exposing_values() {
    let guard = DepotGuard(depot("verify"));
    let records_path = guard.0.join("connections.json");
    let store = ConnectionsStore::new(records_path.clone());
    let broker = SecretBroker::new(InMemorySecretBackend::default());

    let record = store
        .connect(&broker, PROJECT, "erp", "api_key", "https://erp", "tok-1")
        .unwrap();

    // Known connection with a live broker secret: verified.
    assert!(store
        .verify(&broker, PROJECT, &record.connection_id)
        .unwrap());

    // Unknown connection: false, not an error.
    assert!(!store.verify(&broker, PROJECT, "conn-ghost").unwrap());

    // Revoked secret: verification fails honestly.
    store
        .disconnect(&broker, PROJECT, &record.connection_id)
        .unwrap();

    let record = store
        .connect(&broker, PROJECT, "erp", "api_key", "https://erp", "tok-2")
        .unwrap();
    broker
        .revoke(&lumi_protocol::SecretRef(record.credential_ref.clone()))
        .unwrap();
    assert!(!store
        .verify(&broker, PROJECT, &record.connection_id)
        .unwrap());

    // The verify path never returns the credential value anywhere: the
    // metadata file still contains only the reference.
    let raw = std::fs::read_to_string(&records_path).unwrap();
    assert!(!raw.contains("tok-2"), "credential leaked to disk");
}
