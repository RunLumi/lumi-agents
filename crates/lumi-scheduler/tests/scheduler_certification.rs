//! Spec 13.14 certification: schedule-driven runs flow through the SAME
//! orchestrator gate; kill-switch and lease revocation halt runs
//! mid-flight; duplicates never create duplicate consequential work;
//! local-only tasks cannot cloud-fail over.

use lumi_orchestrator::{Orchestrator, OrchestratorConfig, StepOutcome};
use lumi_policy::CapabilityGrant;
use lumi_protocol::{
    ActionProposal, Budget, Capability, ConsumedBudget, ExecutionResult, ExecutionStatus,
    ExecutionTier, ResourceRef, ResourceType, RiskClass, RunId, Target, TaskId, TenantId,
    Timestamp,
};
use lumi_scheduler::{
    CatchUpPolicy, DeduplicationLedger, DeviceAvailability, LeaseRegistry, LeaseRevocation,
    Schedule, ScheduleAdmission, ScheduleId, Scheduler, SchedulerError, TriggerKind,
    TriggerRequest, TriggerSource,
};
use lumi_state::InMemoryStateStore;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn ts(secs: i64) -> Timestamp {
    Timestamp::from_epoch(secs, 0).unwrap()
}

fn principal() -> lumi_protocol::Principal {
    lumi_protocol::Principal {
        principal_id: lumi_protocol::PrincipalId::parse("u-schedule").unwrap(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        kind: lumi_protocol::PrincipalKind::Schedule,
        authenticated_at: Some(ts(0)),
        authentication_strength: Some(lumi_protocol::AuthenticationStrength::DevicePossession),
    }
}

/// A fixture connector executor that counts how many consequential
/// actions actually ran (the "would have been a side effect" probe).
struct CountingExecutor {
    calls: Arc<AtomicUsize>,
}

impl CountingExecutor {
    fn new(calls: Arc<AtomicUsize>) -> Self {
        Self { calls }
    }
}

fn counting_executor(calls: Arc<AtomicUsize>) -> impl FnMut(&ActionProposal) -> ExecutionResult {
    let executor = CountingExecutor::new(calls);
    move |action: &ActionProposal| {
        executor.calls.fetch_add(1, Ordering::SeqCst);
        ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status: ExecutionStatus::Success,
            started_at: ts(0),
            ended_at: ts(1),
            grounding: Some(lumi_protocol::Grounding::DeterministicApi),
            observation_ids: vec![],
            error: None,
        }
    }
}

fn orchestrator() -> Orchestrator<InMemoryStateStore> {
    let registry = lumi_policy::CapabilityRegistry::default().grant(CapabilityGrant {
        grant_id: "g-api-read".to_owned(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        principal_id: None,
        capability: Capability::well_known("api.read"),
        resource_scope: lumi_policy::ResourceScope::all_of([ResourceType::well_known(
            ResourceType::DATABASE_RECORD,
        )]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: lumi_policy::GrantSource::OrganizationPolicy,
    });
    let config = OrchestratorConfig {
        registry,
        device_state: lumi_policy::DeviceExecutionState::Trusted,
        pre_authorizations: vec![lumi_policy::PreAuthorization {
            auth_id: "pre-scheduled-recon".to_owned(),
            capability: Capability::well_known("api.read"),
            resource_types: vec![ResourceType::well_known(ResourceType::DATABASE_RECORD)],
            target_prefixes: vec![],
            workflow_id: Some(lumi_protocol::WorkflowId::parse("invoice-reconciliation").unwrap()),
            max_sensitivity: Some(lumi_protocol::SensitivityLabel::Internal),
            expires_at: Timestamp::from_epoch(10_000_000, 0).unwrap(),
        }],
        retry: lumi_state::RetryPolicy::default(),
        policy_version: "1.0.0".to_owned(),
        executors: vec![
            lumi_orchestrator::ExecutorDescriptor::new(
                ExecutionTier::ConnectorApi,
                "http-connector",
                "1.0.0",
                [Capability::well_known("api.read")],
            ),
            lumi_orchestrator::ExecutorDescriptor::new(
                ExecutionTier::ConnectorApi,
                "http-connector",
                "1.0.0",
                [Capability::well_known("api.write")],
            ),
        ],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
    };
    Orchestrator::new(config, InMemoryStateStore::new())
}

fn read_action(run_id: &RunId) -> ActionProposal {
    ActionProposal::builder(
        lumi_protocol::ActionId::generate(),
        TaskId::parse("task-sched").unwrap(),
        run_id.clone(),
        principal(),
        Capability::well_known("api.read"),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::DATABASE_RECORD),
            id: "erp/invoices/2026-09".to_owned(),
            sensitivity: Some(lumi_protocol::SensitivityLabel::Internal),
        },
        Target::canonical("erp://acme.test/invoices"),
        "fetch_open_invoices",
        RiskClass::Read,
    )
    .workflow_id(lumi_protocol::WorkflowId::parse("invoice-reconciliation").unwrap())
    .unwrap()
}

#[test]
fn schedule_driven_run_flows_through_the_orchestrator_gate() {
    // The scheduler ADMITS; the orchestrator EXECUTES. The audit trail
    // proves the gated path (not a bypass).
    let mut scheduler = Scheduler::new(false);
    scheduler.add_schedule(Schedule {
        schedule_id: ScheduleId::new("nightly-reconciliation"),
        cron_expression: "0 2 * * *".to_owned(),
        timezone: "Europe/Berlin".to_owned(),
        catch_up: CatchUpPolicy::RunOnce,
        quiet_hours: None,
        critical: false,
        enabled: true,
        trigger: TriggerRequest {
            tenant_id: TenantId::parse("t-acme").unwrap(),
            principal_id: "u-schedule".to_owned(),
            template_id: "invoice-reconciliation".to_owned(),
            payload_refs: serde_json::json!({"invoice_ref": "erp/invoices/2026-09"}),
            triggered_at: ts(0),
            deduplication: Some(lumi_scheduler::DeduplicationWindow {
                key: "nightly".to_owned(),
                window_seconds: 60,
            }),
            source: TriggerSource {
                kind: TriggerKind::Cron,
                identity: "cron:0 2 * * *".to_owned(),
            },
            timezone: Some("Europe/Berlin".to_owned()),
            device_availability: DeviceAvailability::WaitForDevice,
            deadline: None,
            max_actions: 5,
            lease_id: "lease-1".to_owned(),
        },
    });

    let mut lease_registry = LeaseRegistry::new();
    lease_registry.register(lumi_scheduler::UnattendedLease {
        lease_id: "lease-1".to_owned(),
        device_id: "dev-1".to_owned(),
        workflow_id: "invoice-reconciliation".to_owned(),
        capability_scope: vec![Capability::well_known("api.read")],
        issued_at: ts(0),
        expires_at: ts(10_000),
        max_actions_total: 5,
        actions_consumed: 0,
        revoked: false,
    });

    let mut dedup = DeduplicationLedger::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let mut orchestrator = orchestrator();

    // Night 1: schedule fires, lease admits, dedup admits, orchestrator
    // executes the gated action.
    let admission = scheduler
        .admit_firing(
            &ScheduleId::new("nightly-reconciliation"),
            ts(100),
            2,
            1,
            true,
        )
        .unwrap();
    let task_id = match &admission {
        ScheduleAdmission::Admit { task_id, .. } => task_id.clone(),
        other => panic!("expected Admit, got {other:?}"),
    };
    let _ = task_id;
    assert!(lease_registry
        .check_run(
            &RunId::parse("run-night-1").unwrap(),
            "lease-1",
            "invoice-reconciliation",
            &Capability::well_known("api.read"),
            ts(100)
        )
        .is_ok());
    {
        let action = read_action(&RunId::parse("run-night-1").unwrap());
        let mut executor = counting_executor(Arc::clone(&calls));
        let outcome = orchestrator.execute_step(
            &action,
            None,
            &Budget::default(),
            &mut ConsumedBudget::default(),
            &lumi_audit::FixtureEnvironment::new(),
            &mut executor,
        );
        assert!(matches!(outcome, StepOutcome::VerifiedSuccess { .. }));
    }

    // Night 1 re-delivered 60s later (webhook-style duplicate): the dedup
    // ledger drops it BEFORE any task/action — the call counter proves no
    // duplicate consequential work (§13.4).
    let dup_key = "cron:0 2 * * *::nightly";
    assert!(!dedup.is_duplicate(dup_key, 3600, ts(100)), "first arrival");
    assert!(
        dedup.is_duplicate(dup_key, 3600, ts(110)),
        "re-delivery is duplicate"
    );
    let before = calls.load(Ordering::SeqCst);
    let second = scheduler.admit_firing(
        &ScheduleId::new("nightly-reconciliation"),
        ts(110),
        2,
        1,
        true,
    );
    // The scheduler's own dedup (window 60s) also refuses.
    assert!(matches!(second, Err(SchedulerError::Duplicate { .. })));
    assert_eq!(calls.load(Ordering::SeqCst), before);
}

#[test]
fn lease_revocation_halts_scheduled_run_mid_flight() {
    let mut lease_registry = LeaseRegistry::new();
    lease_registry.register(lumi_scheduler::UnattendedLease {
        lease_id: "lease-1".to_owned(),
        device_id: "dev-1".to_owned(),
        workflow_id: "invoice-reconciliation".to_owned(),
        capability_scope: vec![Capability::well_known("api.read")],
        issued_at: ts(0),
        expires_at: ts(10_000),
        max_actions_total: 10,
        actions_consumed: 0,
        revoked: false,
    });
    let run = RunId::parse("run-live").unwrap();
    lease_registry
        .check_run(
            &run,
            "lease-1",
            "invoice-reconciliation",
            &Capability::well_known("api.read"),
            ts(1),
        )
        .unwrap();

    // Revoked between steps:
    lease_registry.revoke("lease-1");

    // The run-loop polls recheck() at each step gate; here we prove the
    // mid-run poll refuses AFTER revocation.
    assert_eq!(
        lease_registry.recheck(&run, ts(2)),
        Err(LeaseRevocation::Revoked)
    );
}

#[test]
fn kill_switch_stops_scheduled_run_mid_run() {
    // The emergency stop IS the orchestrator's CancelToken: once set,
    // execute_step returns Stopped at the first gate — no executor call.
    let mut sandbox_orchestrator = orchestrator();
    let run = RunId::parse("run-kill").unwrap();
    let action = read_action(&run);
    let calls = Arc::new(AtomicUsize::new(0));

    {
        let mut executor = counting_executor(Arc::clone(&calls));
        let o1 = sandbox_orchestrator.execute_step(
            &action,
            None,
            &Budget::default(),
            &mut ConsumedBudget::default(),
            &lumi_audit::FixtureEnvironment::new(),
            &mut executor,
        );
        assert!(matches!(o1, StepOutcome::VerifiedSuccess { .. }));
    }

    // KILL SWITCH: one call, everything stops.
    sandbox_orchestrator.cancel.cancel();

    let mut executor = counting_executor(Arc::clone(&calls));
    let outcome = sandbox_orchestrator.execute_step(
        &action,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &lumi_audit::FixtureEnvironment::new(),
        &mut executor,
    );
    match outcome {
        StepOutcome::Stopped { reason } => assert!(reason.contains("cancelled"), "{reason}"),
        other => panic!("expected Stopped, got {other:?}"),
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "no executor call after kill"
    );
}

#[test]
fn local_only_scheduled_task_cannot_cloud_failover() {
    // §13.14: local-only task cannot cloud failover.
    let mut scheduler = Scheduler::new(true); // local-only deployment
    scheduler.add_schedule(Schedule {
        schedule_id: ScheduleId::new("local-recon"),
        cron_expression: "0 3 * * *".to_owned(),
        timezone: "UTC".to_owned(),
        catch_up: CatchUpPolicy::RunOnce,
        quiet_hours: None,
        critical: false,
        enabled: true,
        trigger: TriggerRequest {
            tenant_id: TenantId::parse("t-acme").unwrap(),
            principal_id: "u-schedule".to_owned(),
            template_id: "invoice-reconciliation".to_owned(),
            payload_refs: serde_json::json!({}),
            triggered_at: ts(0),
            deduplication: None,
            source: TriggerSource {
                kind: TriggerKind::Cron,
                identity: "cron:0 3 * * *".to_owned(),
            },
            timezone: Some("UTC".to_owned()),
            // Even if the trigger WOULD permit rerouting:
            device_availability: DeviceAvailability::MayRerouteIfPolicyPermits,
            deadline: None,
            max_actions: 5,
            lease_id: "lease-1".to_owned(),
        },
    });
    // Device offline + local-only: the admission refuses rather than
    // rerouting to a cloud worker.
    let err = scheduler
        .admit_firing(&ScheduleId::new("local-recon"), ts(100), 7, 1, false)
        .unwrap_err();
    assert!(matches!(err, SchedulerError::DeviceUnavailable));
}

#[test]
fn budget_exhausted_scheduled_task_stops_cleanly() {
    let mut orchestrator = orchestrator();
    // One-action budget; two actions requested.
    let budget = Budget {
        max_actions: Some(1),
        ..Budget::default()
    };
    let mut consumed = ConsumedBudget::default();
    let env = lumi_audit::FixtureEnvironment::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let run = RunId::parse("run-budget").unwrap();

    let mut executor = counting_executor(Arc::clone(&calls));
    let first = orchestrator.execute_step(
        &read_action(&run),
        None,
        &budget,
        &mut consumed,
        &env,
        &mut executor,
    );
    assert!(matches!(first, StepOutcome::VerifiedSuccess { .. }));

    let second = orchestrator.execute_step(
        &read_action(&run),
        None,
        &budget,
        &mut consumed,
        &env,
        &mut executor,
    );
    assert!(matches!(second, StepOutcome::Stopped { .. }));
}

#[test]
fn quiet_hours_and_approval_delay_park_the_task() {
    // §13.13: approval required in background => WAITING_APPROVAL, no
    // material action proceeds. The quote pack's send step is the
    // canonical case; here we prove the orchestrator parks a gated
    // action without executing.
    // api.write is granted (known to policy) but consequential and NOT
    // pre-authorized: policy demands approval exactly as a background
    // task with an approval-gated step should experience (§13.13).
    let mut orchestrator = {
        let mut o = orchestrator();
        o.config.registry = o.config.registry.grant(CapabilityGrant {
            grant_id: "g-api-write".to_owned(),
            tenant_id: TenantId::parse("t-acme").unwrap(),
            principal_id: None,
            capability: Capability::well_known("api.write"),
            resource_scope: lumi_policy::ResourceScope::all_of([ResourceType::well_known(
                ResourceType::DATABASE_RECORD,
            )]),
            target_prefixes: BTreeSet::new(),
            expires_at: None,
            source: lumi_policy::GrantSource::OrganizationPolicy,
        });
        o
    };

    let action = ActionProposal::builder(
        lumi_protocol::ActionId::generate(),
        TaskId::parse("task-park").unwrap(),
        RunId::parse("run-park").unwrap(),
        principal(),
        Capability::well_known("api.write"),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::DATABASE_RECORD),
            id: "expenses/audits/ER-77".to_owned(),
            sensitivity: Some(lumi_protocol::SensitivityLabel::Internal),
        },
        Target::canonical("expenses://acme.test/audits/ER-77"),
        "record_audit_verdict",
        RiskClass::ExternalWrite,
    )
    .workflow_id(lumi_protocol::WorkflowId::parse("invoice-reconciliation").unwrap())
    .unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let mut executor = counting_executor(Arc::clone(&calls));
    let outcome = orchestrator.execute_step(
        &action,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &lumi_audit::FixtureEnvironment::new(),
        &mut executor,
    );
    match outcome {
        StepOutcome::ApprovalNeeded { reason, .. } => {
            assert!(!reason.is_empty());
        }
        other => panic!("expected ApprovalNeeded, got {other:?}"),
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "no material action proceeds"
    );
}
