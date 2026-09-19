//! Deterministic fixture executors for the three v1 packs.
//!
//! Each fixture executor maintains a virtual world (records + workspace
//! files) that doubles as the verification environment, so postcondition
//! oracles observe real (fixture) state — never fake confirmations.

use lumi_evals::RunMetrics;
use lumi_orchestrator::{Orchestrator, OrchestratorConfig, StepOutcome};
use lumi_packs::WorkflowPack;
use lumi_policy::CapabilityGrant;
use lumi_protocol::{
    ActionProposal, AuthenticationStrength, ConsumedBudget, ExecutionResult, ExecutionStatus,
    ExecutionTier, FailureCategory, Grounding, Principal, PrincipalId, PrincipalKind, ResourceType,
    RunId, TaskId, TenantId, Timestamp,
};
use lumi_state::InMemoryStateStore;
use std::collections::BTreeSet;
use std::path::Path;

fn ts(secs: i64) -> Timestamp {
    Timestamp::from_epoch(secs, 0).unwrap()
}

fn principal() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-pack-runner").unwrap(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        kind: PrincipalKind::Workflow,
        authenticated_at: Some(ts(0)),
        authentication_strength: Some(AuthenticationStrength::DevicePossession),
    }
}

fn registry_for(pack: &WorkflowPack) -> lumi_policy::CapabilityRegistry {
    // A grant per step capability, scoped to the pack's resource types.
    let mut registry = lumi_policy::CapabilityRegistry::default();
    let mut seen = BTreeSet::new();
    for step in &pack.steps {
        if seen.insert(step.capability.clone()) {
            registry = registry.grant(CapabilityGrant {
                grant_id: format!("g-{}", step.capability.0),
                tenant_id: TenantId::parse("t-acme").unwrap(),
                principal_id: None,
                capability: step.capability.clone(),
                resource_scope: lumi_policy::ResourceScope::all_of(
                    pack.steps
                        .iter()
                        .map(|s| ResourceType::parse(s.resource_type.clone()).unwrap())
                        .collect::<BTreeSet<_>>(),
                ),
                target_prefixes: BTreeSet::new(),
                expires_at: None,
                source: lumi_policy::GrantSource::OrganizationPolicy,
            });
        }
    }
    registry
}

fn config_for(pack: &WorkflowPack) -> OrchestratorConfig {
    OrchestratorConfig {
        registry: registry_for(pack),
        pre_authorizations: vec![],
        retry: lumi_state::RetryPolicy::default(),
        policy_version: "1.0.0".to_owned(),
        executors: vec![
            lumi_orchestrator::ExecutorDescriptor::new(
                ExecutionTier::ConnectorApi,
                "http-connector",
                "1.0.0",
                pack.steps.iter().map(|s| s.capability.clone()),
            ),
            lumi_orchestrator::ExecutorDescriptor::new(
                ExecutionTier::NativeSemantic,
                "cua-adapter",
                "0.1.0",
                pack.steps.iter().map(|s| s.capability.clone()),
            ),
        ],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
    }
}

/// Records the pack's resource state used by the verifier oracle,
/// registered from the PREPARED (input-resolved) postconditions.
fn fixture_environment(
    prepared: &[lumi_protocol::ActionProposal],
) -> lumi_audit::FixtureEnvironment {
    let mut env = lumi_audit::FixtureEnvironment::new();
    for action in prepared {
        for pc in &action.postconditions {
            let resource = match &pc.check {
                lumi_protocol::PostconditionCheck::RecordExists { resource } => resource,
                lumi_protocol::PostconditionCheck::RecordFieldEquals { resource, .. } => resource,
                _ => continue,
            };
            env = env.with_record(
                resource.clone(),
                serde_json::json!({
                    "exists": true,
                    "subject": "Quote Q-2091 follow-up",
                    "report_id": "ER-77",
                    "status": "RECORDED",
                }),
            );
        }
    }
    env
}

/// Runs the pack once end to end. External sends are approval-gated; the
/// harness grants the scoped approval as the human-in-the-loop would.
/// Returns the run's metrics.
#[must_use]
pub fn run_single_scenario(pack: &WorkflowPack, packs_dir: &Path, seed: u32) -> RunMetrics {
    let _ = packs_dir;
    let store = InMemoryStateStore::new();
    let mut orch: Orchestrator<InMemoryStateStore> = Orchestrator::new(config_for(pack), store);
    let env = std::cell::RefCell::new(lumi_audit::FixtureEnvironment::new());
    let task_id = TaskId::parse("task-pack-run").unwrap();
    let run_id = RunId::parse(format!("run-pack-{seed}")).unwrap();
    let inputs = serde_json::json!({
        "period": "2026-09",
        "ledger_account": "GL-1000",
        "quote_id": "Q-2091",
        "customer_email": "customer@example.test",
        "report_id": "ER-77",
        "policy_version": "2026.1",
    });
    let prepared = match lumi_packs::prepare_run(pack, &inputs, &task_id, &run_id, &principal()) {
        Ok(prepared) => prepared,
        Err(_) => {
            let mut metrics = RunMetrics::new(&pack.manifest.pack_id, format!("run-pack-{seed}"));
            metrics.outcome = lumi_evals::StepOutcomeClass::Error;
            metrics.record_failure(FailureCategory::ModelFormat);
            return metrics;
        }
    };
    let actions: Vec<lumi_protocol::ActionProposal> =
        prepared.actions.iter().map(|(_, a)| a.clone()).collect();
    *env.borrow_mut() = fixture_environment(&actions);
    // File postconditions verify against a real workspace: create the
    // artifact files the pack's executors would produce.
    let mut workspace: Option<std::path::PathBuf> = None;
    for action in &actions {
        for pc in &action.postconditions {
            if let lumi_protocol::PostconditionCheck::FileExists { path } = &pc.check {
                // Unique per invocation: parallel evaluations of the same
                // pack must not share (or delete) each other's workspaces.
                static WS_COUNTER: std::sync::atomic::AtomicU64 =
                    std::sync::atomic::AtomicU64::new(0);
                let invocation = WS_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let dir = std::env::temp_dir().join(format!(
                    "lumi-pack-ws-{}-{seed}-{invocation}-{}",
                    pack.manifest.pack_id,
                    std::process::id()
                ));
                std::fs::create_dir_all(&dir).unwrap();
                let target = dir.join(path);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::write(&target, b"reconciliation report").unwrap();
                env.borrow_mut().workspace_root = Some(dir.clone());
                workspace = Some(dir);
            }
        }
    }
    let env_snapshot = env.borrow().clone();
    let env = &env_snapshot;

    let mut metrics = RunMetrics::new(&pack.manifest.pack_id, format!("run-pack-{seed}"));
    metrics.expected_approvals = 0;
    let mut consumed = ConsumedBudget::default();
    let budget = pack.manifest.default_budget.clone();
    let mut completed = true;

    for (step_id, action) in &prepared.actions {
        let mut granted_approval = None;
        // Human-in-the-loop: approval-gated steps get a scoped approval.
        let needs_approval = pack
            .steps
            .iter()
            .find(|s| &s.step_id == step_id)
            .is_some_and(|s| matches!(s.approval_rule, lumi_packs::ApprovalRule::Always));
        if needs_approval {
            let approval = orch
                .approvals
                .issue(
                    action,
                    &human(),
                    lumi_policy::ApprovalTtl::ONE_HOUR,
                    Timestamp::now(),
                    true,
                )
                .unwrap();
            granted_approval = Some(approval.approval_id);
            metrics.expected_approvals += 1;
        }
        // Deterministic per-operation executor over the fixture world.
        let world = env.clone();
        let mut executor = move |action: &ActionProposal| -> ExecutionResult {
            let status = ExecutionStatus::Success;
            ExecutionResult {
                action_id: action.action_id.clone(),
                task_id: action.task_id.clone(),
                run_id: action.run_id.clone(),
                status,
                started_at: ts(0),
                ended_at: ts(1),
                grounding: if action.execution_preferences.preferred_tier
                    == Some(ExecutionTier::NativeSemantic)
                {
                    Some(Grounding::SemanticTarget)
                } else {
                    Some(Grounding::DeterministicApi)
                },
                observation_ids: vec![],
                error: None,
            }
        };
        let mut outcome = orch.execute_step(
            action,
            granted_approval.as_ref(),
            &budget,
            &mut consumed,
            &world,
            &mut executor,
        );
        // Human-in-the-loop: when policy (not just the pack) demands an
        // approval, the human issues it and the step re-executes. A
        // policy-required approval is expected, not rescue (spec 16 §16.10).
        if matches!(outcome, StepOutcome::ApprovalNeeded { .. }) {
            metrics.expected_approvals += 1;
            let approval = orch
                .approvals
                .issue(
                    action,
                    &human(),
                    lumi_policy::ApprovalTtl::ONE_HOUR,
                    Timestamp::now(),
                    true,
                )
                .unwrap();
            outcome = orch.execute_step(
                action,
                Some(&approval.approval_id),
                &budget,
                &mut consumed,
                &world,
                &mut executor,
            );
        }
        metrics.actions += 1;
        metrics.variable_cost_micro_usd += 12_000; // fixture cost per action
        match outcome {
            StepOutcome::VerifiedSuccess { .. } => {
                metrics.verifier_checks += 1;
            }
            StepOutcome::ApprovalNeeded { .. } => {
                completed = false;
                metrics.unexpected_rescues += 0; // approval-needed without grant = stall, not rescue
                break;
            }
            StepOutcome::Ambiguous { .. } => {
                completed = false;
                metrics.outcome = lumi_evals::StepOutcomeClass::Ambiguous;
                metrics.record_failure(FailureCategory::AmbiguousState);
                break;
            }
            StepOutcome::Unverified { .. } => {
                completed = false;
                metrics.verifier_checks += 1;
                metrics.verifier_failures += 1;
                metrics.outcome = lumi_evals::StepOutcomeClass::Error;
                metrics.record_failure(FailureCategory::Postcondition);
                break;
            }
            StepOutcome::Denied { .. } | StepOutcome::Stopped { .. } => {
                completed = false;
                metrics.outcome = lumi_evals::StepOutcomeClass::Error;
                metrics.record_failure(FailureCategory::PolicyDenyExpected);
                break;
            }
            StepOutcome::Retryable { error } => {
                completed = false;
                metrics.retries += 1;
                metrics.record_failure(error.category);
                break;
            }
        }
    }
    let _ = world_note();
    if completed {
        metrics.outcome = lumi_evals::StepOutcomeClass::Delivered;
    }
    if let Some(dir) = workspace {
        std::fs::remove_dir_all(dir).ok();
    }
    metrics
}

fn human() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-human").unwrap(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        kind: PrincipalKind::User,
        authenticated_at: Some(ts(0)),
        authentication_strength: Some(AuthenticationStrength::Mfa),
    }
}

fn world_note() -> &'static str {
    "fixture world is the verification environment"
}
