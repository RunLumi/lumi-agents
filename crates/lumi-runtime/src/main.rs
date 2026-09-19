//! Dry-run smoke binary: one normalized action through the policy gate.
//!
//! This stays minimal on purpose; the executable vertical slice lives in
//! `lumi-orchestrator` (specs 02/11/16/12 land there). Until then this
//! binary proves the protocol→policy→dispatch pipeline compiles and runs.

use lumi_policy::{
    evaluate, ApprovalLedger, CapabilityGrant, CapabilityRegistry, DeviceExecutionState,
    GrantSource, PolicyContext, ResourceScope,
};
use lumi_protocol::{
    ActionId, ActionProposal, AuthenticationStrength, Capability, ExecutionStatus, Principal,
    PrincipalId, PrincipalKind, ResourceRef, ResourceType, RiskClass, RunId, Target, TaskId,
    TenantId, Timestamp,
};
use lumi_runtime::{dispatch, Executor};
use std::collections::BTreeSet;

struct DryRunExecutor;

impl Executor for DryRunExecutor {
    fn execute(&mut self, action: &ActionProposal) -> lumi_protocol::ExecutionResult {
        println!(
            "dry-run execute: capability={} target={} operation={}",
            action.capability.as_str(),
            action.target.canonical,
            action.operation
        );
        lumi_protocol::ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status: ExecutionStatus::Success,
            started_at: Timestamp::now(),
            ended_at: Timestamp::now(),
            grounding: None,
            observation_ids: vec![],
            error: None,
        }
    }
}

fn main() {
    let principal = Principal {
        principal_id: PrincipalId::parse("user-bootstrap").unwrap(),
        tenant_id: TenantId::parse("tenant-bootstrap").unwrap(),
        kind: PrincipalKind::User,
        authenticated_at: Some(Timestamp::now()),
        authentication_strength: Some(AuthenticationStrength::Mfa),
    };
    let registry = CapabilityRegistry::default().grant(CapabilityGrant {
        grant_id: "g-files-read".to_owned(),
        tenant_id: principal.tenant_id.clone(),
        principal_id: None,
        capability: Capability::well_known(lumi_protocol::capabilities::FILES_READ),
        resource_scope: ResourceScope::all_of([ResourceType::well_known(ResourceType::FILE)]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: GrantSource::DeviceDefault,
    });
    let action = ActionProposal::builder(
        ActionId::parse("act-bootstrap").unwrap(),
        TaskId::parse("task-bootstrap").unwrap(),
        RunId::parse("run-bootstrap").unwrap(),
        principal,
        Capability::well_known(lumi_protocol::capabilities::FILES_READ),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::FILE),
            id: "fixtures/hello.txt".to_owned(),
            sensitivity: None,
        },
        Target::canonical("file://fixtures/hello.txt"),
        "read_file",
        RiskClass::Read,
    )
    .unwrap();

    let mut ledger = ApprovalLedger::new();
    let mut executor = DryRunExecutor;
    let outcome = dispatch(
        |action| {
            evaluate(
                action,
                &[],
                &PolicyContext {
                    now: Timestamp::now(),
                    device_state: DeviceExecutionState::Trusted,
                    registry: Some(&registry),
                },
            )
        },
        &mut ledger,
        &mut executor,
        &action,
        None,
        Timestamp::now(),
    );
    println!("policy outcome: {outcome:?}");
}
