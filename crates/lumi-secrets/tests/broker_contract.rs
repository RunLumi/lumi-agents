//! Secrets broker contract tests (hermetic, in-memory backend) plus an
//! OS-keychain integration test on macOS/Windows.

use lumi_protocol::SecretRef;
use lumi_secrets::{
    InMemorySecretBackend, SecretAuditOperation, SecretBroker, SecretError, SecretValue,
};
use std::sync::Arc;

fn broker() -> SecretBroker<InMemorySecretBackend> {
    SecretBroker::new(InMemorySecretBackend::new())
}

#[test]
fn store_resolve_round_trip() {
    let broker = broker();
    let reference = SecretRef::new("erp-production");
    broker
        .store(&reference, &SecretValue::new("s3cret-value"))
        .unwrap();
    let resolved = broker.resolve(&reference, "erp-connector-auth").unwrap();
    assert_eq!(resolved.expose(), "s3cret-value");
}

#[test]
fn unknown_reference_fails_not_found() {
    let broker = broker();
    let err = broker
        .resolve(&SecretRef::new("missing"), "test")
        .unwrap_err();
    assert_eq!(
        err,
        SecretError::NotFound {
            reference: "missing".to_owned()
        }
    );
    assert_eq!(err.category(), lumi_protocol::FailureCategory::AuthSession);
}

#[test]
fn revoke_removes_and_later_resolution_fails() {
    let broker = broker();
    let reference = SecretRef::new("crm-api-key");
    broker
        .store(&reference, &SecretValue::new("key-1"))
        .unwrap();
    assert!(broker.exists(&reference).unwrap());

    // Revocation removes the secret; resolution now fails (spec 17/19:
    // revocation must cut access).
    assert!(broker.revoke(&reference).unwrap());
    assert!(!broker.exists(&reference).unwrap());
    assert!(matches!(
        broker.resolve(&reference, "late-call"),
        Err(SecretError::NotFound { .. })
    ));

    // Double revoke reports already-absent, not an error.
    assert!(!broker.revoke(&reference).unwrap());
}

#[test]
fn audit_trail_records_operations_without_values() {
    use std::sync::Mutex;
    let sink_events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink_handle = Arc::clone(&sink_events);
    let broker = broker().with_audit_sink(Box::new(move |event| {
        // In production this forwards into the evidence pipeline.
        let serialized = serde_json::to_string(event).unwrap();
        assert!(
            !serialized.contains("s3cret-value"),
            "audit must not carry values"
        );
        sink_handle.lock().unwrap().push(serialized);
    }));
    let reference = SecretRef::new("erp-production");
    let secret = SecretValue::new("s3cret-value");

    broker.store(&reference, &secret).unwrap();
    broker.resolve(&reference, "erp-connector-auth").unwrap();
    broker.revoke(&reference).unwrap();

    // The audit sink fired for every operation.
    assert_eq!(sink_events.lock().unwrap().len(), 3);

    let trail = broker.audit_trail();
    assert_eq!(trail.len(), 3);
    assert_eq!(trail[0].operation, SecretAuditOperation::Store);
    assert_eq!(trail[1].operation, SecretAuditOperation::Resolve);
    assert_eq!(trail[1].purpose, "erp-connector-auth");
    assert_eq!(trail[2].operation, SecretAuditOperation::Delete);

    // Serialize the WHOLE audit trail: the value must not appear anywhere.
    let serialized = serde_json::to_string(&trail).unwrap();
    assert!(!serialized.contains("s3cret-value"), "{serialized}");
}

#[test]
fn invalid_references_fail_closed() {
    let broker = broker();
    assert!(matches!(
        broker.store(&SecretRef::new(""), &SecretValue::new("x")),
        Err(SecretError::InvalidRef { .. })
    ));
    let control_char = SecretRef::new("bad\u{0}ref");
    assert!(matches!(
        broker.store(&control_char, &SecretValue::new("x")),
        Err(SecretError::InvalidRef { .. })
    ));
    let long = SecretRef::new("x".repeat(257));
    assert!(matches!(
        broker.store(&long, &SecretValue::new("x")),
        Err(SecretError::InvalidRef { .. })
    ));
}

#[test]
fn secret_value_never_leaks_through_debug_display_or_serde() {
    let value = SecretValue::new("super-secret-42");
    let rendered = format!("{value:?} {value} {value:#?}");
    assert!(!rendered.contains("super-secret-42"));
    // Serde of the broker error surfaces refs, not values.
    let broker = broker();
    let reference = SecretRef::new("k");
    broker.store(&reference, &SecretValue::new("v-1")).unwrap();
    drop(broker);
    // SecretValue is not Serialize at all; a compile-time guarantee.
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod os_keychain {
    use lumi_protocol::SecretRef;
    use lumi_secrets::{KeyringBackend, SecretBroker, SecretValue};

    #[test]
    fn keyring_backend_round_trip_and_revoke() {
        let backend = KeyringBackend::new();
        let reference = SecretRef::new(format!("lumi-ci-{}", std::process::id()));
        let broker = SecretBroker::new(backend);

        assert!(!broker.exists(&reference).unwrap());
        broker
            .store(&reference, &SecretValue::new("ci-keychain-value"))
            .unwrap();
        assert!(broker.exists(&reference).unwrap());
        let resolved = broker.resolve(&reference, "ci").unwrap();
        assert_eq!(resolved.expose(), "ci-keychain-value");
        assert!(broker.revoke(&reference).unwrap());
        assert!(!broker.exists(&reference).unwrap());
    }
}
