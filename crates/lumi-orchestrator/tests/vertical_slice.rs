//! End-to-end vertical slice: intent -> policy -> approval -> executor ->
//! verification -> audit -> durable state -> crash recovery.
//!
//! This suite is the executable proof of the AGENTS.md core invariant with
//! deterministic fixture executors (no network, no model):

use lumi_audit::{ChainVerification, FixtureEnvironment, VerificationStatus};
use lumi_orchestrator::{Orchestrator, OrchestratorConfig, StepOutcome};
use lumi_policy::{
    evaluate, CapabilityGrant, DeviceExecutionState, GrantSource, PolicyContext, ResourceScope,
};
use lumi_protocol::{
    ActionId, ActionProposal, AuthenticationStrength, Budget, Capability, ConsumedBudget,
    ExecutionResult, ExecutionStatus, FailureCategory, Principal, PrincipalId, PrincipalKind,
    ResourceRef, ResourceType, RiskClass, RunId, SensitivityLabel, Target, TaskId, TenantId,
    Timestamp,
};
use lumi_state::{AmbiguousResolution, InMemoryStateStore, SideEffectStatus};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

fn ts(secs: i64) -> Timestamp {
    Timestamp::from_epoch(secs, 0).unwrap()
}

fn principal() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-worker").unwrap(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        kind: PrincipalKind::Workflow,
        authenticated_at: Some(ts(0)),
        authentication_strength: Some(AuthenticationStrength::DevicePossession),
    }
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

fn registry() -> lumi_policy::CapabilityRegistry {
    let grant = |id: &'static str, cap: &'static str, rtype: ResourceType| CapabilityGrant {
        grant_id: id.to_owned(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        principal_id: None,
        capability: Capability::well_known(cap),
        resource_scope: ResourceScope::all_of([rtype]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: GrantSource::OrganizationPolicy,
    };
    lumi_policy::CapabilityRegistry::default()
        .grant(grant(
            "g-api-read",
            lumi_protocol::capabilities::FILES_READ,
            ResourceType::well_known(ResourceType::FILE),
        ))
        .grant(grant(
            "g-email-send",
            lumi_protocol::capabilities::EMAIL_SEND,
            ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
        ))
        .grant(grant(
            "g-crm-update",
            lumi_protocol::capabilities::CRM_QUOTE_UPDATE,
            ResourceType::well_known(ResourceType::CRM_QUOTE),
        ))
}

fn config() -> OrchestratorConfig {
    OrchestratorConfig {
        registry: registry(),
        pre_authorizations: vec![],
        retry: lumi_state::RetryPolicy::default(),
        policy_version: "1.0.0".to_owned(),
    }
}

fn email_action(id: &str) -> ActionProposal {
    ActionProposal::builder(
        ActionId::parse(id).unwrap(),
        TaskId::parse("task-reconcile").unwrap(),
        RunId::parse("run-1").unwrap(),
        principal(),
        Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: Some(SensitivityLabel::Confidential),
        },
        Target::canonical("mailto:customer@example.test"),
        "send_customer_email",
        RiskClass::Communication,
    )
    .arguments(serde_json::json!({"subject": "Quote follow-up", "body": "Hi!"}))
    .postconditions(vec![lumi_protocol::Postcondition {
        id: lumi_protocol::PostconditionId::new("message-exists"),
        description: "sent message exists with expected subject".to_owned(),
        check: lumi_protocol::PostconditionCheck::RecordFieldEquals {
            resource: ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "outbound/customer-42".to_owned(),
                sensitivity: None,
            },
            field: "subject".to_owned(),
            expected: serde_json::json!("Quote follow-up"),
        },
    }])
    .evidence_requirements(vec![lumi_protocol::EvidenceRequirement::StructuredState])
    .idempotency(lumi_protocol::Idempotency {
        key: Some("customer-42-followup".to_owned()),
        semantics: lumi_protocol::IdempotencySemantics::ClientKey,
    })
    .unwrap()
}

/// Builds a scripted executor closure: each call pops the next scripted
/// status; the shared counter records how often the executor actually ran
/// (proving the executor never runs without authorization).
fn scripted(
    script: Vec<ExecutionStatus>,
) -> (
    impl FnMut(&ActionProposal) -> ExecutionResult,
    Rc<Cell<usize>>,
) {
    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let script = Rc::new(std::cell::RefCell::new(std::collections::VecDeque::from(
        script,
    )));
    let move_script = Rc::clone(&script);
    let executor = move |action: &ActionProposal| {
        observed.set(observed.get() + 1);
        let status = move_script
            .borrow_mut()
            .pop_front()
            .unwrap_or(ExecutionStatus::Failed);
        let error = if status == ExecutionStatus::Failed {
            Some(lumi_protocol::ErrorEnvelope::from_adapter(
                "http",
                "429",
                "rate limited",
            ))
        } else if status == ExecutionStatus::Ambiguous {
            Some(lumi_protocol::ErrorEnvelope::new(
                FailureCategory::AmbiguousState,
                "submit timed out; state unknown",
            ))
        } else {
            None
        };
        ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status,
            started_at: ts(0),
            ended_at: ts(1),
            grounding: Some(lumi_protocol::Grounding::DeterministicApi),
            observation_ids: vec![],
            error,
        }
    };
    (executor, calls)
}

#[test]
fn full_pipeline_read_execute_approve_execute_verify() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let env = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Quote follow-up"}),
    );
    let budget = Budget::default();
    let mut consumed = ConsumedBudget::default();

    // 1. A read action inside its grant runs without approval.
    let read = ActionProposal::builder(
        ActionId::parse("a-read").unwrap(),
        TaskId::parse("task-reconcile").unwrap(),
        RunId::parse("run-1").unwrap(),
        principal(),
        Capability::well_known(lumi_protocol::capabilities::FILES_READ),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::FILE),
            id: "invoices/q3.csv".to_owned(),
            sensitivity: Some(SensitivityLabel::Internal),
        },
        Target::canonical("file://workspaces/w1/invoices/q3.csv"),
        "read_invoices",
        RiskClass::Read,
    )
    .unwrap();
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let outcome = orch.execute_step(&read, None, &budget, &mut consumed, &env, &mut executor);
    assert_eq!(calls.get(), 1);
    assert_eq!(
        outcome,
        StepOutcome::VerifiedSuccess {
            action_id: read.action_id,
            verification: VerificationStatus::NotRequired,
        }
    );

    // 2. The consequential email action is stopped at the approval gate;
    // the executor is never invoked.
    let email = email_action("a-email");
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let outcome = orch.execute_step(&email, None, &budget, &mut consumed, &env, &mut executor);
    assert_eq!(calls.get(), 0, "executor must not run before approval");
    let digest = match outcome {
        StepOutcome::ApprovalNeeded { action_digest, .. } => action_digest,
        other => panic!("expected ApprovalNeeded, got {other:?}"),
    };
    assert_eq!(digest, email.material_digest());

    // 3. A human approves the exact normalized action.
    let approval = orch
        .approvals
        .issue(
            &email,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let outcome = orch.execute_step(
        &email,
        Some(&approval.approval_id),
        &budget,
        &mut consumed,
        &env,
        &mut executor,
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(
        outcome,
        StepOutcome::VerifiedSuccess {
            action_id: email.action_id,
            verification: VerificationStatus::Passed,
        }
    );

    // 4. The journal shows exactly one verified-applied side effect.
    let record = orch
        .journal
        .get(&ActionId::parse("a-email").unwrap())
        .unwrap();
    assert_eq!(record.status, SideEffectStatus::Succeeded);

    // 5. Audit chain is intact: read executed, approval required, approval
    // consumed, email executed+verified. Approval linkage travels with the
    // consumed event; evidence was captured for the verified step.
    assert_eq!(
        orch.audit.verify_chain(),
        ChainVerification::Intact { events: 4 }
    );
    assert!(!orch.evidence.is_empty());
}

#[test]
fn ambiguous_submit_recovers_without_double_send() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let email = email_action("a-submit");
    let approval = orch
        .approvals
        .issue(
            &email,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();

    // Executor times out after submit: status unknown.
    let env_absent = FixtureEnvironment::new();
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Ambiguous]);
    let outcome = orch.execute_step(
        &email,
        Some(&approval.approval_id.clone()),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env_absent,
        &mut executor,
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(
        outcome,
        StepOutcome::Ambiguous {
            action_id: email.action_id.clone()
        }
    );

    // The journal record is unresolved and blocks continuation.
    let unresolved = orch
        .journal
        .unresolved_for_run(&RunId::parse("run-1").unwrap());
    assert_eq!(unresolved.len(), 1);

    // External state now shows the message exists: resolution confirms
    // applied — the executor is NOT re-run.
    let env_present = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Quote follow-up"}),
    );
    let resolution = orch.resolve_ambiguity(&email, &env_present);
    assert_eq!(resolution, AmbiguousResolution::ConfirmedApplied);
    let record = orch.journal.get(&email.action_id).unwrap();
    assert_eq!(record.status, SideEffectStatus::Succeeded);
    assert_eq!(calls.get(), 1, "no re-execution after confirmed applied");
}

#[test]
fn policy_denial_never_reaches_executor_and_audits() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    // Financial action: hard gate, no grant exists either.
    let payment = ActionProposal::builder(
        ActionId::parse("a-pay").unwrap(),
        TaskId::parse("task-reconcile").unwrap(),
        RunId::parse("run-1").unwrap(),
        principal(),
        Capability::well_known("payments.transfer"),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::INVOICE),
            id: "inv-1".to_owned(),
            sensitivity: None,
        },
        Target::canonical("bank://acct/1"),
        "pay_invoice",
        RiskClass::Financial,
    )
    .unwrap();
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let outcome = orch.execute_step(
        &payment,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );
    assert_eq!(calls.get(), 0);
    assert!(matches!(outcome, StepOutcome::Denied { .. }));
}

#[test]
fn budget_exhaustion_stops_cleanly_and_cancellation_is_prompt() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let env = FixtureEnvironment::new();
    let read = ActionProposal::builder(
        ActionId::parse("a-read-2").unwrap(),
        TaskId::parse("task-reconcile").unwrap(),
        RunId::parse("run-1").unwrap(),
        principal(),
        Capability::well_known(lumi_protocol::capabilities::FILES_READ),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::FILE),
            id: "x".to_owned(),
            sensitivity: None,
        },
        Target::canonical("file://x"),
        "read_file",
        RiskClass::Read,
    )
    .unwrap();

    // Budget: 1 external write max; this read is not external so use
    // action count instead.
    let budget = Budget {
        max_actions: Some(1),
        ..Budget::default()
    };
    let mut consumed = ConsumedBudget::default();
    let (mut executor, _calls) = scripted(vec![ExecutionStatus::Success]);
    let first = orch.execute_step(&read, None, &budget, &mut consumed, &env, &mut executor);
    assert!(matches!(first, StepOutcome::VerifiedSuccess { .. }));
    // Budget consumed: next step stops in a controlled way.
    let second = orch.execute_step(&read, None, &budget, &mut consumed, &env, &mut executor);
    assert!(matches!(second, StepOutcome::Stopped { .. }));

    // Cancellation is checked before policy.
    orch.cancel.cancel();
    let third = orch.execute_step(&read, None, &budget, &mut consumed, &env, &mut executor);
    assert!(matches!(third, StepOutcome::Stopped { .. }));
}

#[test]
fn crash_after_propose_resumes_with_reverify_plan() {
    let store = InMemoryStateStore::new();
    let mut orch = Orchestrator::new(config(), store.clone());
    let email = email_action("a-crash");
    let approval = orch
        .approvals
        .issue(
            &email,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    // Execute with an ambiguous outcome, then "crash": the journal was
    // persisted into the shared store.
    let env = FixtureEnvironment::new();
    let (mut executor, _calls) = scripted(vec![ExecutionStatus::Ambiguous]);
    let _ = orch.execute_step(
        &email,
        Some(&approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );

    // Restart: rebuild the orchestrator from the same durable store.
    let mut restarted = Orchestrator::restore(config(), store);
    let unresolved = restarted
        .journal
        .unresolved_for_run(&RunId::parse("run-1").unwrap());
    assert_eq!(unresolved.len(), 1, "journal must survive the restart");

    let task = lumi_protocol::Task {
        task_id: TaskId::parse("task-reconcile").unwrap(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        principal: principal(),
        mode: lumi_protocol::TaskMode::Workflow,
        goal: "reconcile invoices".to_owned(),
        created_at: ts(0),
        deadline: None,
        budget: Budget::default(),
        privacy_constraints: lumi_protocol::PrivacyConstraint::default(),
        status: lumi_protocol::TaskStatus::Running,
        requested_outputs: vec![],
    };
    let run = lumi_protocol::Run {
        run_id: RunId::parse("run-1").unwrap(),
        task_id: TaskId::parse("task-reconcile").unwrap(),
        runtime_version: env!("CARGO_PKG_VERSION").to_owned(),
        workflow_version: None,
        selected_providers: vec![],
        started_at: ts(0),
        ended_at: None,
        state: lumi_protocol::RunState::Recovering,
        budgets_consumed: ConsumedBudget::default(),
        failure: None,
    };
    let plan = restarted.plan_resume(&task, &run, None, Default::default(), "1.0.0");
    assert_eq!(
        plan,
        vec![lumi_state::ResumeAction::ReverifySideEffect {
            action_id: email.action_id.clone()
        }]
    );

    // Resolve through external verification: the record exists but with a
    // different subject - the message we sent never landed.
    let env_other = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Unrelated older thread"}),
    );
    let resolution = restarted.resolve_ambiguity(&email, &env_other);
    assert_eq!(resolution, AmbiguousResolution::ConfirmedNotApplied);
    let record = restarted.journal.get(&email.action_id).unwrap();
    assert_eq!(record.status, SideEffectStatus::Failed);
}

#[test]
fn policy_still_gates_when_evaluated_directly() {
    // Sanity: direct policy evaluation sees identical normalized actions
    // across tiers (spec 03 §3.12 policy-identity fixture, now through the
    // real engine).
    let reg = registry();
    let ctx = PolicyContext {
        now: ts(0),
        device_state: DeviceExecutionState::Trusted,
        registry: Some(&reg),
    };
    let mut connector = email_action("a-t1");
    connector.execution_preferences.allowed_tiers =
        vec![lumi_protocol::ExecutionTier::ConnectorApi];
    let mut native = email_action("a-t2");
    native.execution_preferences.allowed_tiers = vec![lumi_protocol::ExecutionTier::NativeSemantic];
    let d1 = evaluate(&connector, &[], &ctx);
    let d2 = evaluate(&native, &[], &ctx);
    assert_eq!(d1, d2);
    assert!(matches!(d1, lumi_policy::Decision::RequireApproval { .. }));
}
