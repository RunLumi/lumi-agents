//! Provider instance lifecycle and routing guarantee tests (spec 09 §9.19).

use lumi_models::capabilities::ModelCapability;
use lumi_models::driver::ProviderDriver;
use lumi_models::instance::{
    complete_on_instance, InstanceStatus, ProviderInstance, HEALTH_PROBE_TIMEOUT,
};
use lumi_models::pricing::PriceTable;
use lumi_models::request::{ModelRequest, ModelRole};
use lumi_models::response::Usage;
use lumi_models::routing::{ensure_admission, route, Candidate, TenantModelPolicy};
use lumi_models::snapshot::{DataClassification, RouteSnapshot, SnapshotValidation};
use lumi_models::transport::{FixtureResponse, FixtureTransport};
use std::collections::BTreeSet;
use std::time::Duration;

fn openai_instance(id: &str) -> ProviderInstance {
    ProviderInstance::new(
        id,
        "openai",
        format!("label-{id}"),
        "https://api.openai.test",
        lumi_protocol::SecretRef::new(format!("secret-{id}")),
        "openai",
    )
    .with_regions(vec!["us"])
}

fn request() -> ModelRequest {
    ModelRequest::text("req-1", ModelRole::Planner, "gpt-4o-mini", "hello")
}

#[test]
fn two_accounts_same_driver_stay_isolated() {
    let mut a = openai_instance("inst-a");
    let b = openai_instance("inst-b");
    // Mutating one instance's catalog/status cannot leak into the other.
    a.catalog.insert(
        "gpt-4o-mini".to_owned(),
        lumi_models::capabilities::text_model("gpt-4o-mini"),
    );
    a.revoke();
    assert_eq!(a.status, InstanceStatus::Revoked);
    assert!(a.catalog.is_empty());
    assert_eq!(b.status, InstanceStatus::Active);
    assert!(b.catalog.is_empty());
    assert_ne!(a.credential_ref, b.credential_ref);
    assert_ne!(a.instance_id, b.instance_id);
}

#[test]
fn revoked_instance_blocks_new_admission() {
    let mut instance = openai_instance("inst-a");
    let transport = FixtureTransport::serving(vec![FixtureResponse::Http {
        status: 200,
        body: serde_json::json!({"id": "x", "choices": [{"message": {"content": "ok"}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 1, "completion_tokens": 1}}).to_string(),
    }]);
    let driver = lumi_models::adapters::OpenAIDriver::new();

    // Active: request admitted.
    let auth = driver.auth_headers("test-key");
    assert!(complete_on_instance(&driver, &instance, &auth, &transport, &request()).is_ok());

    // Revoked: new work refused BEFORE any transport call.
    instance.revoke();
    let error = complete_on_instance(&driver, &instance, &auth, &transport, &request())
        .expect_err("revoked instance must refuse new work");
    assert_eq!(
        error.category,
        lumi_protocol::FailureCategory::SecurityViolation
    );
    // No new request hit the transport.
    assert_eq!(transport.requests().len(), 1);

    // Routing-level guard agrees.
    assert!(ensure_admission(InstanceStatus::Revoked).is_err());
    assert!(ensure_admission(InstanceStatus::Paused).is_err());
    assert!(ensure_admission(InstanceStatus::Active).is_ok());
}

#[test]
fn local_only_never_falls_back_to_cloud() {
    let policy = TenantModelPolicy::local_only("1.0.0");
    let candidates = vec![
        Candidate::new("local-1", "openai-compatible", "ollama", "llama3.1").local(),
        Candidate::new("cloud-1", "openai", "openai", "gpt-4o-mini").with_regions(&["us"]),
    ];
    let required = BTreeSet::from([ModelCapability::Text]);
    let (choice, snapshot) = route(&candidates, &policy, &request(), &required, |c, cap| {
        cap == ModelCapability::Text && (c.local || c.provider_name == "openai")
    })
    .unwrap();
    assert!(choice.primary.local);
    assert!(choice.fallbacks.iter().all(|f| f.local));
    assert_eq!(snapshot.data_classification, DataClassification::LocalOnly);
}

#[test]
fn route_snapshot_survives_resume_and_refuses_widening() {
    let policy = TenantModelPolicy::cloud("1.0.0", ["anthropic"]);
    let candidates = vec![
        Candidate::new("inst-a", "anthropic", "anthropic", "claude-sonnet").with_regions(&["us"]),
    ];
    let required = BTreeSet::new();
    let (_choice, snapshot) =
        route(&candidates, &policy, &request(), &required, |_, _| true).unwrap();

    // Persist + reload (serde round trip stands in for durable storage).
    let persisted = serde_json::to_string(&snapshot).unwrap();
    let reloaded: RouteSnapshot = serde_json::from_str(&persisted).unwrap();

    // Same policy at resume: reuse.
    assert_eq!(
        reloaded.validate_for_resume("anthropic", true, "1.0.0"),
        SnapshotValidation::Reusable
    );
    // Policy tightened: explicit re-evaluation, never silent rerouting.
    match reloaded.validate_for_resume("anthropic", true, "2.0.0-tightened") {
        SnapshotValidation::ReEvaluate { reason } => {
            assert!(reason.contains("policy version changed"), "{reason}");
        }
        other => panic!("expected re-evaluation, got {other:?}"),
    }
}

#[test]
fn usage_costs_flow_into_the_economics_ledger() {
    let mut table = PriceTable::new();
    table.set(
        "gpt-4o-mini",
        lumi_models::pricing::ModelPrice {
            input_per_1k: 150,
            output_per_1k: 600,
        },
    );
    let usage = Usage {
        input_tokens: 10_000,
        output_tokens: 1_000,
    };
    // 10 * 150 + 1 * 600 = 2100 micro-USD.
    assert_eq!(table.cost_micro_usd("gpt-4o-mini", &usage), Some(2_100));
    // Unknown model: None, so callers cannot under-count silently.
    assert_eq!(table.cost_micro_usd("mystery", &usage), None);
    let _ = HEALTH_PROBE_TIMEOUT;
    let _ = Duration::ZERO;
}
