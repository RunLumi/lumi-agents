//! Role Pack v0 composition and authority-ceiling contracts.

use lumi_packs::{RolePack, RoleWorkItem, WorkflowPack};
use lumi_protocol::{Capability, RiskClass};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn role() -> RolePack {
    let path = repo_root().join("roles/finance-operations-associate/role.json");
    let text = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn packs() -> Vec<lumi_packs::WorkflowPack> {
    let packs_dir = repo_root().join("packs");
    ["invoice-reconciliation", "expense-report-audit"]
        .into_iter()
        .map(|id| {
            let text = std::fs::read_to_string(packs_dir.join(id).join("pack.json")).unwrap();
            let pack: WorkflowPack = serde_json::from_str(&text).unwrap();
            pack.validate().unwrap();
            pack
        })
        .collect()
}

#[test]
fn finance_role_composes_exact_pack_versions_and_authority() {
    let role = role();
    let packs = packs();
    role.validate(&packs).expect("role composition is valid");
    assert_eq!(role.workflow_packs.len(), 2);
    assert!(role
        .authority
        .required_approval_risk_classes
        .contains(&RiskClass::ExternalWrite));
}

#[test]
fn missing_workflow_reference_fails_closed() {
    let mut role = role();
    role.workflow_packs[0].pack_id = "missing-pack".to_owned();
    let errors = role.validate(&packs()).unwrap_err();
    assert!(errors.iter().any(|error| error.contains("not available")));
}

#[test]
fn workflow_version_mismatch_fails_closed() {
    let mut role = role();
    role.workflow_packs[0].version = "9.9.9".to_owned();
    let errors = role.validate(&packs()).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.contains("required version")));
}

#[test]
fn duplicate_workflow_id_fails_closed_even_with_different_version() {
    let mut role = role();
    role.workflow_packs[1].pack_id = role.workflow_packs[0].pack_id.clone();
    role.workflow_packs[1].version = "9.9.9".to_owned();
    let errors = role.validate(&packs()).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.contains("referenced more than once")));
}

#[test]
fn capability_expansion_fails_closed() {
    let mut role = role();
    role.authority
        .allowed_capabilities
        .insert(Capability::well_known("shell.execute"));
    let errors = role.validate(&packs()).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.contains("expands beyond composed capabilities")));
}

#[test]
fn approval_narrowing_fails_closed() {
    let mut role = role();
    role.authority
        .required_approval_risk_classes
        .remove(&RiskClass::ExternalWrite);
    let errors = role.validate(&packs()).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.contains("without a role approval ceiling")));
}

#[test]
fn budget_expansion_fails_closed() {
    let mut role = role();
    role.authority.max_actions = 99;
    let errors = role.validate(&packs()).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.contains("max_actions") && error.contains("widens")));
}

fn work_item(workflow_id: &str, inputs: serde_json::Value) -> RoleWorkItem {
    RoleWorkItem {
        work_item_id: "invoice-batch-2026-09".to_owned(),
        workflow_id: workflow_id.to_owned(),
        inputs,
    }
}

#[test]
fn work_item_requires_stable_identity_and_exact_workflow() {
    let role = role();
    let mut item = work_item(
        "invoice-reconciliation",
        serde_json::json!({
            "work_item_id": "invoice-batch-2026-09",
            "source_system": "erp",
            "period": "2026-09",
            "ledger_account": "GL-1000"
        }),
    );
    role.validate_work_item(&item).unwrap();
    item.work_item_id = "".to_owned();
    assert!(role.validate_work_item(&item).is_err());
    item.work_item_id = "invoice-batch-2026-09".to_owned();
    item.workflow_id = "quote-followup".to_owned();
    assert!(role.validate_work_item(&item).is_err());
}

#[test]
fn work_item_rejects_wrong_payload_types_and_unknown_fields() {
    let role = role();
    let wrong_type = work_item(
        "invoice-reconciliation",
        serde_json::json!({
            "work_item_id": "invoice-batch-2026-09",
            "source_system": "erp",
            "period": 202609,
            "ledger_account": "GL-1000"
        }),
    );
    assert!(role.validate_work_item(&wrong_type).is_err());
    let unknown_field = work_item(
        "invoice-reconciliation",
        serde_json::json!({
            "work_item_id": "invoice-batch-2026-09",
            "source_system": "erp",
            "period": "2026-09",
            "ledger_account": "GL-1000",
            "unexpected": true
        }),
    );
    let errors = role.validate_work_item(&unknown_field).unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.contains("unknown work item")));
}

#[test]
fn role_schema_rejects_unknown_fields_and_wrong_version() {
    let mut raw = serde_json::to_value(role()).unwrap();
    raw["unsupported_future_field"] = serde_json::json!(true);
    assert!(serde_json::from_value::<RolePack>(raw).is_err());
    let mut nested = serde_json::to_value(role()).unwrap();
    nested["work_item_schema"]["unsupported_future_field"] = serde_json::json!(true);
    assert!(serde_json::from_value::<RolePack>(nested).is_err());
    let mut role = role();
    role.schema_version = 1;
    let errors = role.validate(&packs()).unwrap_err();
    assert!(errors.iter().any(|error| error.contains("schema_version")));
}

#[test]
fn same_version_pack_content_changes_digest() {
    let role = role();
    let original = packs();
    let first = role.content_digest(&original).unwrap();
    let mut changed = original.clone();
    changed[0].steps[0].target_template = "erp://changed.example/invoices".to_owned();
    let second = role.content_digest(&changed).unwrap();
    assert_ne!(first, second);
}

#[test]
fn v1_pack_corpus_remains_three_packs() {
    let packs_dir = repo_root().join("packs");
    let count = std::fs::read_dir(packs_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("pack.json").is_file())
        .count();
    assert_eq!(count, 3);
}
