//! End-to-end vertical slice: intent -> policy -> approval -> executor ->
//! verification -> audit -> durable state -> crash recovery.
//!
//! This suite is the executable proof of the AGENTS.md core invariant with
//! deterministic fixture executors (no network, no model):

use lumi_audit::{AuditEventKind, ChainVerification, FixtureEnvironment, VerificationStatus};
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
use lumi_state::{
    AmbiguousResolution, Checkpoint, InMemoryStateStore, JsonStateStore, PreActionCheckpoint,
    SideEffectJournal, SideEffectRecord, SideEffectStatus, StateStore, StoreError,
};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

/// Error-injected durable store used to prove that the orchestrator never
/// dispatches after a failed intent write, and never reports success after a
/// failed post-effect journal write.
struct FailingStore {
    inner: InMemoryStateStore,
    fail_pre_action: bool,
    fail_load_journal: bool,
    fail_journal_call: Option<usize>,
    journal_calls: usize,
}

impl FailingStore {
    fn normal() -> Self {
        Self {
            inner: InMemoryStateStore::new(),
            fail_pre_action: false,
            fail_load_journal: false,
            fail_journal_call: None,
            journal_calls: 0,
        }
    }

    fn fail_pre_action() -> Self {
        Self {
            fail_pre_action: true,
            ..Self::normal()
        }
    }

    fn fail_journal_call(call: usize) -> Self {
        Self {
            fail_journal_call: Some(call),
            ..Self::normal()
        }
    }

    fn fail_load_journal() -> Self {
        Self {
            fail_load_journal: true,
            ..Self::normal()
        }
    }
}

impl StateStore for FailingStore {
    fn save_task(&mut self, task: &lumi_protocol::Task) -> Result<(), StoreError> {
        self.inner.save_task(task)
    }

    fn load_task(&self, task_id: &TaskId) -> Result<lumi_protocol::Task, StoreError> {
        self.inner.load_task(task_id)
    }

    fn save_run(&mut self, run: &lumi_protocol::Run) -> Result<(), StoreError> {
        self.inner.save_run(run)
    }

    fn load_run(&self, run_id: &RunId) -> Result<lumi_protocol::Run, StoreError> {
        self.inner.load_run(run_id)
    }

    fn save_checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), StoreError> {
        self.inner.save_checkpoint(checkpoint)
    }

    fn load_checkpoint(&self, run_id: &RunId) -> Result<Checkpoint, StoreError> {
        self.inner.load_checkpoint(run_id)
    }

    fn save_pre_action(&mut self, pre: &PreActionCheckpoint) -> Result<(), StoreError> {
        if self.fail_pre_action {
            return Err(StoreError::Io(
                "injected pre-action write failure".to_owned(),
            ));
        }
        self.inner.save_pre_action(pre)
    }

    fn load_pre_action(&self, run_id: &RunId) -> Result<PreActionCheckpoint, StoreError> {
        self.inner.load_pre_action(run_id)
    }

    fn save_journal(&mut self, journal: &SideEffectJournal) -> Result<(), StoreError> {
        self.journal_calls += 1;
        if self.fail_journal_call == Some(self.journal_calls) {
            return Err(StoreError::Io(format!(
                "injected journal write failure at call {}",
                self.journal_calls
            )));
        }
        self.inner.save_journal(journal)
    }

    fn load_journal(&self) -> Result<SideEffectJournal, StoreError> {
        if self.fail_load_journal {
            return Err(StoreError::Io("injected journal load failure".to_owned()));
        }
        self.inner.load_journal()
    }

    fn load_retry_attempts(
        &self,
        tenant_id: &TenantId,
        run_id: &RunId,
        action_id: &ActionId,
    ) -> Result<Option<u32>, StoreError> {
        self.inner.load_retry_attempts(tenant_id, run_id, action_id)
    }

    fn save_retry_attempts(
        &mut self,
        tenant_id: &TenantId,
        run_id: &RunId,
        action_id: &ActionId,
        attempts: u32,
    ) -> Result<(), StoreError> {
        self.inner
            .save_retry_attempts(tenant_id, run_id, action_id, attempts)
    }
}

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
        executors: vec![
            lumi_orchestrator::ExecutorDescriptor::new(
                lumi_protocol::ExecutionTier::ConnectorApi,
                "http-connector",
                "1.0.0",
                [
                    Capability::well_known(lumi_protocol::capabilities::FILES_READ),
                    Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
                    Capability::well_known(lumi_protocol::capabilities::CRM_QUOTE_UPDATE),
                ],
            ),
            lumi_orchestrator::ExecutorDescriptor::new(
                lumi_protocol::ExecutionTier::Vision,
                "vision-grounding",
                "1.0.0",
                [Capability::well_known(
                    lumi_protocol::capabilities::EMAIL_SEND,
                )],
            ),
        ],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
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
            observation: None,
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
            observation: None,
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
fn failed_pre_intent_write_never_reaches_executor() {
    let mut orch = Orchestrator::new(config(), FailingStore::fail_pre_action());
    let email = email_action("a-pre-write-failure");
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
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );

    assert_eq!(calls.get(), 0, "intent must be durable before dispatch");
    match outcome {
        StepOutcome::Stopped { reason } => {
            assert!(reason.contains("pre-action intent"), "{reason}")
        }
        other => panic!("expected fail-closed stop, got {other:?}"),
    }
}

#[test]
fn failed_post_effect_write_never_reports_verified_success() {
    // Call 1 persists the pre-dispatch journal. Call 2 is the post-effect
    // journal update, which is intentionally failed after the executor ran.
    let mut orch = Orchestrator::new(config(), FailingStore::fail_journal_call(2));
    let email = email_action("a-post-write-failure");
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
    let env = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Quote follow-up"}),
    );
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);

    let outcome = orch.execute_step(
        &email,
        Some(&approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );

    assert_eq!(calls.get(), 1, "the executor was called exactly once");
    match outcome {
        StepOutcome::Stopped { reason } => {
            assert!(reason.contains("post-effect state persistence"), "{reason}")
        }
        other => panic!("expected persistence stop, got {other:?}"),
    }
    // The in-memory journal remains Proposed and persistence is sticky, so a
    // retry is blocked rather than silently submitting the same external effect.
    let (mut second_executor, second_calls) = scripted(vec![ExecutionStatus::Success]);
    let second = orch.execute_step(
        &email,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut second_executor,
    );
    assert!(matches!(second, StepOutcome::Stopped { .. }));
    assert_eq!(second_calls.get(), 0);
}

#[test]
fn restore_load_failure_stops_before_any_executor_call() {
    let mut orch = Orchestrator::restore(config(), FailingStore::fail_load_journal());
    let read = ActionProposal::builder(
        ActionId::parse("a-restore-failure").unwrap(),
        TaskId::parse("task-reconcile").unwrap(),
        RunId::parse("run-restore-failure").unwrap(),
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

    let outcome = orch.execute_step(
        &read,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );

    assert_eq!(calls.get(), 0);
    match outcome {
        StepOutcome::Stopped { reason } => {
            assert!(reason.contains("durable state unavailable"), "{reason}")
        }
        other => panic!("expected restore failure stop, got {other:?}"),
    }
}

#[test]
fn direct_replay_of_succeeded_consequential_action_is_blocked() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let email = email_action("a-direct-replay");
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
    let env = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Quote follow-up"}),
    );
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let first = orch.execute_step(
        &email,
        Some(&approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert!(matches!(first, StepOutcome::VerifiedSuccess { .. }));

    let (mut replay_executor, replay_calls) = scripted(vec![ExecutionStatus::Success]);
    let replay = orch.execute_step(
        &email,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut replay_executor,
    );
    assert!(matches!(replay, StepOutcome::Stopped { .. }));
    assert_eq!(calls.get(), 1);
    assert_eq!(replay_calls.get(), 0, "direct replay must not dispatch");

    // Downgrading the same action id to READ must not bypass the journal
    // guard and execute a second time.
    let downgraded = ActionProposal {
        risk_class: RiskClass::Read,
        ..email.clone()
    };
    let (mut downgraded_executor, downgraded_calls) = scripted(vec![ExecutionStatus::Success]);
    let downgraded_result = orch.execute_step(
        &downgraded,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut downgraded_executor,
    );
    assert!(matches!(downgraded_result, StepOutcome::Stopped { .. }));
    assert_eq!(downgraded_calls.get(), 0);
}

#[test]
fn explicit_approval_mode_gates_policy_allow_and_consumes_once() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let read = ActionProposal::builder(
        ActionId::parse("a-always-approval").unwrap(),
        TaskId::parse("task-reconcile").unwrap(),
        RunId::parse("run-always-approval").unwrap(),
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
    let budget = Budget::default();
    let env = FixtureEnvironment::new();

    // Ordinary policy ALLOW is not enough for an Always-approval workflow
    // step, and an unknown approval id must not reach the executor.
    let missing = lumi_protocol::ApprovalId::generate();
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let missing_outcome = orch.execute_step_requiring_approval(
        &read,
        Some(&missing),
        &budget,
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert!(matches!(
        missing_outcome,
        StepOutcome::ApprovalNeeded { .. }
    ));
    assert_eq!(calls.get(), 0);

    let approval = orch
        .approvals
        .issue(
            &read,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let first = orch.execute_step_requiring_approval(
        &read,
        Some(&approval.approval_id),
        &budget,
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert!(matches!(first, StepOutcome::VerifiedSuccess { .. }));
    assert_eq!(calls.get(), 1);

    // Single-use consumption is enforced by the same approval ledger on a
    // second attempt; the executor is not called again.
    let second = orch.execute_step_requiring_approval(
        &read,
        Some(&approval.approval_id),
        &budget,
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert!(matches!(second, StepOutcome::ApprovalNeeded { .. }));
    assert_eq!(calls.get(), 1);

    // A role-level Always flag cannot override policy DENY.
    let denied = ActionProposal::builder(
        ActionId::parse("a-always-denied").unwrap(),
        TaskId::parse("task-reconcile").unwrap(),
        RunId::parse("run-always-denied").unwrap(),
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
    let denied_outcome = orch.execute_step_requiring_approval(
        &denied,
        Some(&missing),
        &budget,
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert!(matches!(denied_outcome, StepOutcome::Denied { .. }));
    assert_eq!(calls.get(), 1);
}

#[test]
fn failed_postcondition_keeps_effect_ambiguous_and_blocks_retry() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let email = email_action("a-wrong-readback");
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
    // The record exists, but its field differs. This failed equality check
    // does not prove the external write was absent.
    let env = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "different message"}),
    );
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let first = orch.execute_step(
        &email,
        Some(&approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert!(matches!(first, StepOutcome::Unverified { .. }));
    assert_eq!(
        orch.journal.get(&email.action_id).unwrap().status,
        SideEffectStatus::Ambiguous
    );

    let (mut replay_executor, replay_calls) = scripted(vec![ExecutionStatus::Success]);
    let replay = orch.execute_step(
        &email,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut replay_executor,
    );
    assert!(matches!(replay, StepOutcome::Ambiguous { .. }));
    assert_eq!(calls.get(), 1);
    assert_eq!(replay_calls.get(), 0);
}

#[test]
fn failed_consequential_executor_result_requires_external_resolution() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let email = email_action("a-failed-submit");
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
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Failed, ExecutionStatus::Success]);
    let first = orch.execute_step(
        &email,
        Some(&approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );
    assert!(matches!(first, StepOutcome::Ambiguous { .. }));
    assert_eq!(
        orch.journal.get(&email.action_id).unwrap().status,
        SideEffectStatus::Ambiguous
    );
    let action_event = orch
        .audit
        .events_for(&TenantId::parse("t-acme").unwrap())
        .into_iter()
        .find_map(|event| match &event.kind {
            AuditEventKind::Action(details) if details.action_id == email.action_id => {
                Some(details)
            }
            _ => None,
        })
        .expect("ambiguous failure must remain auditable");
    assert_eq!(
        action_event.failure_category,
        Some(FailureCategory::ProviderRateLimit)
    );

    let replay = orch.execute_step(
        &email,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );
    assert!(matches!(replay, StepOutcome::Ambiguous { .. }));
    assert_eq!(
        calls.get(),
        1,
        "failed consequential dispatch must not auto-retry"
    );
}

#[test]
fn mismatched_consequential_executor_result_is_ambiguous() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let email = email_action("a-mismatched-result");
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
    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let mut executor = move |action: &ActionProposal| {
        observed.set(observed.get() + 1);
        ExecutionResult {
            action_id: ActionId::parse("a-other").unwrap(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status: ExecutionStatus::Success,
            started_at: ts(0),
            ended_at: ts(1),
            grounding: Some(lumi_protocol::Grounding::DeterministicApi),
            observation_ids: vec![],
            error: None,
        }
    };
    let first = orch.execute_step(
        &email,
        Some(&approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );
    assert!(matches!(first, StepOutcome::Ambiguous { .. }));
    assert_eq!(calls.get(), 1);
}

#[test]
fn altered_verifier_plan_cannot_resolve_unrelated_action() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let email = email_action("a-verifier-binding");
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
    let (mut executor, _calls) = scripted(vec![ExecutionStatus::Ambiguous]);
    let _ = orch.execute_step(
        &email,
        Some(&approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );
    assert_eq!(
        orch.journal.get(&email.action_id).unwrap().status,
        SideEffectStatus::Ambiguous
    );

    // The altered plan would pass against this state if the verifier were
    // called, but it is not the verifier plan persisted with the intent.
    let altered = ActionProposal {
        postconditions: vec![lumi_protocol::Postcondition {
            id: lumi_protocol::PostconditionId::new("message-exists"),
            description: "different expected subject".to_owned(),
            check: lumi_protocol::PostconditionCheck::RecordFieldEquals {
                resource: ResourceRef {
                    resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                    id: "outbound/customer-42".to_owned(),
                    sensitivity: None,
                },
                field: "subject".to_owned(),
                expected: serde_json::json!("attacker-controlled subject"),
            },
        }],
        ..email.clone()
    };
    let env = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "attacker-controlled subject"}),
    );
    assert_eq!(
        orch.resolve_ambiguity(&altered, &env),
        AmbiguousResolution::StillUnknown
    );
    assert_eq!(
        orch.journal.get(&email.action_id).unwrap().status,
        SideEffectStatus::Ambiguous
    );
}

#[test]
fn terminal_journal_resolution_is_rejected() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let email = email_action("a-terminal-resolution");
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
    let env = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Quote follow-up"}),
    );
    let (mut executor, _calls) = scripted(vec![ExecutionStatus::Success]);
    assert!(matches!(
        orch.execute_step(
            &email,
            Some(&approval.approval_id),
            &Budget::default(),
            &mut ConsumedBudget::default(),
            &env,
            &mut executor,
        ),
        StepOutcome::VerifiedSuccess { .. }
    ));
    assert_eq!(
        orch.resolve_ambiguity(&email, &env),
        AmbiguousResolution::StillUnknown
    );
    assert_eq!(
        orch.journal.get(&email.action_id).unwrap().status,
        SideEffectStatus::Succeeded
    );
}

#[test]
fn idempotency_scope_blocks_same_business_work_but_allows_distinct_scope() {
    let mut cfg = config();
    cfg.registry = cfg.registry.grant(CapabilityGrant {
        grant_id: "g-other-tenant-email".to_owned(),
        tenant_id: TenantId::parse("t-other").unwrap(),
        principal_id: None,
        capability: Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
        resource_scope: ResourceScope::all_of([ResourceType::well_known(
            ResourceType::EMAIL_MESSAGE,
        )]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: GrantSource::OrganizationPolicy,
    });
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(cfg, InMemoryStateStore::new());
    let first = email_action("a-idempotency-first");
    let approval = orch
        .approvals
        .issue(
            &first,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let env = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Quote follow-up"}),
    );
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    assert!(matches!(
        orch.execute_step(
            &first,
            Some(&approval.approval_id),
            &Budget::default(),
            &mut ConsumedBudget::default(),
            &env,
            &mut executor,
        ),
        StepOutcome::VerifiedSuccess { .. }
    ));

    // New action/run IDs with the same business scope and key are blocked.
    let same_scope = ActionProposal {
        action_id: ActionId::parse("a-idempotency-second").unwrap(),
        run_id: RunId::parse("run-2").unwrap(),
        ..first.clone()
    };
    let (mut duplicate_executor, duplicate_calls) = scripted(vec![ExecutionStatus::Success]);
    let duplicate = orch.execute_step(
        &same_scope,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut duplicate_executor,
    );
    assert!(matches!(duplicate, StepOutcome::Stopped { .. }));
    assert_eq!(duplicate_calls.get(), 0);

    // A different tenant is a different explicit business scope and is
    // allowed to execute the same client key.
    let mut other_principal = principal();
    other_principal.tenant_id = TenantId::parse("t-other").unwrap();
    other_principal.principal_id = PrincipalId::parse("u-other").unwrap();
    let other_tenant = ActionProposal {
        action_id: ActionId::parse("a-idempotency-other-tenant").unwrap(),
        run_id: RunId::parse("run-other").unwrap(),
        principal: other_principal,
        ..first.clone()
    };
    let other_approval = orch
        .approvals
        .issue(
            &other_tenant,
            &Principal {
                principal_id: PrincipalId::parse("u-other-human").unwrap(),
                tenant_id: TenantId::parse("t-other").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(ts(0)),
                authentication_strength: Some(AuthenticationStrength::Mfa),
            },
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let (mut other_executor, other_calls) = scripted(vec![ExecutionStatus::Success]);
    let other = orch.execute_step(
        &other_tenant,
        Some(&other_approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut other_executor,
    );
    assert!(matches!(other, StepOutcome::VerifiedSuccess { .. }));
    assert_eq!(calls.get(), 1);
    assert_eq!(other_calls.get(), 1);

    // A different acting principal is still the same business scope because
    // no explicit connector/account identity is present in this action.
    let mut other_actor = principal();
    other_actor.principal_id = PrincipalId::parse("u-other-account").unwrap();
    let other_actor = ActionProposal {
        action_id: ActionId::parse("a-idempotency-other-actor").unwrap(),
        run_id: RunId::parse("run-other-actor").unwrap(),
        principal: other_actor,
        ..first.clone()
    };
    let actor_approval = orch
        .approvals
        .issue(
            &other_actor,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let (mut actor_executor, actor_calls) = scripted(vec![ExecutionStatus::Success]);
    let actor = orch.execute_step(
        &other_actor,
        Some(&actor_approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut actor_executor,
    );
    assert!(matches!(actor, StepOutcome::Stopped { .. }));
    assert_eq!(actor_calls.get(), 0);

    // A genuinely different resource/target is a different conservative
    // scope and can execute the same client key.
    let other_resource = ActionProposal {
        action_id: ActionId::parse("a-idempotency-other-resource").unwrap(),
        run_id: RunId::parse("run-other-resource").unwrap(),
        resource: ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-43".to_owned(),
            sensitivity: Some(SensitivityLabel::Confidential),
        },
        target: Target::canonical("mailto:other@example.test"),
        postconditions: vec![],
        ..first
    };
    let resource_approval = orch
        .approvals
        .issue(
            &other_resource,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let (mut resource_executor, resource_calls) = scripted(vec![ExecutionStatus::Success]);
    let resource = orch.execute_step(
        &other_resource,
        Some(&resource_approval.approval_id),
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut resource_executor,
    );
    assert!(matches!(resource, StepOutcome::Ambiguous { .. }));
    assert_eq!(resource_calls.get(), 1);
}

#[test]
fn legacy_idempotency_scope_without_metadata_fails_closed() {
    let mut store = InMemoryStateStore::new();
    let mut journal = SideEffectJournal::new();
    journal
        .restore(vec![SideEffectRecord {
            action_id: ActionId::parse("a-legacy-key").unwrap(),
            task_id: None,
            run_id: RunId::parse("run-legacy").unwrap(),
            tenant_id: None,
            principal_id: None,
            idempotency_key: Some("customer-42-followup".to_owned()),
            action_digest: "legacy-digest".to_owned(),
            risk_class: None,
            resource_sensitivity: None,
            idempotency_scope_digest: None,
            verifier_plan_digest: None,
            status: SideEffectStatus::Succeeded,
            proposed_at: ts(0),
            resolved_at: Some(ts(1)),
            result_digest: Some("legacy-result".to_owned()),
            failure: None,
        }])
        .unwrap();
    store.save_journal(&journal).unwrap();
    let mut orch = Orchestrator::new(config(), store);
    let action = email_action("a-new-key-id");
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let outcome = orch.execute_step(
        &action,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );
    assert!(matches!(outcome, StepOutcome::Stopped { .. }));
    assert_eq!(calls.get(), 0);
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
fn retry_accounting_survives_restart_and_exhausts_policy_budget() {
    let store = InMemoryStateStore::new();
    let mut cfg = config();
    cfg.retry.max_attempts = 2;
    let mut orch = Orchestrator::new(cfg, store.clone());
    let action = ActionProposal::builder(
        ActionId::parse("a-retry-budget").unwrap(),
        TaskId::parse("task-retry-budget").unwrap(),
        RunId::parse("run-retry-budget").unwrap(),
        principal(),
        Capability::well_known(lumi_protocol::capabilities::FILES_READ),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::FILE),
            id: "invoices/retry.csv".to_owned(),
            sensitivity: Some(SensitivityLabel::Internal),
        },
        Target::canonical("file://workspaces/retry/invoices.csv"),
        "read_invoices",
        RiskClass::Read,
    )
    .unwrap();
    let budget = Budget {
        max_retries: Some(2),
        ..Budget::default()
    };
    let mut consumed = ConsumedBudget::default();
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Failed, ExecutionStatus::Failed]);
    assert!(matches!(
        orch.execute_step(
            &action,
            None,
            &budget,
            &mut consumed,
            &FixtureEnvironment::new(),
            &mut executor,
        ),
        StepOutcome::Retryable { .. }
    ));
    assert_eq!(consumed.retries, 1);
    assert!(matches!(
        orch.execute_step(
            &action,
            None,
            &budget,
            &mut consumed,
            &FixtureEnvironment::new(),
            &mut executor,
        ),
        StepOutcome::Retryable { .. }
    ));
    assert_eq!(consumed.retries, 2);
    assert_eq!(calls.get(), 2);

    assert_eq!(consumed.retries, 2);
    drop(orch);

    // A restart rehydrates the consumed retry count from the gate-owned
    // durable retry record. The next call stops at the budget gate and never
    // reaches the executor, rather than resetting the RetryController.
    let mut restart_cfg = config();
    restart_cfg.retry.max_attempts = 2;
    let mut restarted = Orchestrator::new(restart_cfg, store);
    let mut restored = ConsumedBudget::default();
    let (mut resumed_executor, resumed_calls) = scripted(vec![ExecutionStatus::Failed]);
    let outcome = restarted.execute_step(
        &action,
        None,
        &budget,
        &mut restored,
        &FixtureEnvironment::new(),
        &mut resumed_executor,
    );
    assert!(matches!(outcome, StepOutcome::Stopped { .. }));
    assert_eq!(restored.retries, 2);
    assert_eq!(resumed_calls.get(), 0);
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

    // A record with a different subject does not prove that the attempted
    // effect was absent; resolution must remain blocked.
    let env_other = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Unrelated older thread"}),
    );
    let resolution = restarted.resolve_ambiguity(&email, &env_other);
    assert_eq!(resolution, AmbiguousResolution::StillUnknown);
    let record = restarted.journal.get(&email.action_id).unwrap();
    assert_eq!(record.status, SideEffectStatus::Ambiguous);
}

#[test]
fn new_loads_existing_json_journal_before_dispatch() {
    let path = std::env::temp_dir().join(format!(
        "lumi-orchestrator-new-journal-{}-{}.json",
        std::process::id(),
        Timestamp::now().epoch_seconds()
    ));
    let _ = std::fs::remove_file(&path);
    let email = email_action("a-new-loads-journal");
    let approval;
    {
        let mut first = Orchestrator::new(config(), JsonStateStore::new(path.clone()));
        approval = first
            .approvals
            .issue(
                &email,
                &human(),
                lumi_policy::ApprovalTtl::ONE_HOUR,
                Timestamp::now(),
                true,
            )
            .unwrap();
        let (mut executor, calls) = scripted(vec![ExecutionStatus::Ambiguous]);
        let outcome = first.execute_step(
            &email,
            Some(&approval.approval_id),
            &Budget::default(),
            &mut ConsumedBudget::default(),
            &FixtureEnvironment::new(),
            &mut executor,
        );
        assert!(matches!(outcome, StepOutcome::Ambiguous { .. }));
        assert_eq!(calls.get(), 1);
    }

    let mut restarted = Orchestrator::new(config(), JsonStateStore::new(path.clone()));
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let outcome = restarted.execute_step(
        &email,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );
    assert!(matches!(outcome, StepOutcome::Ambiguous { .. }));
    assert_eq!(calls.get(), 0, "new must load the existing journal");
    let _ = std::fs::remove_file(path);
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

#[test]
fn vision_tier_selection_cannot_bypass_approval() {
    // The action allows ONLY the vision tier for a consequential operation:
    // policy demands approval, and the vision executor never runs without
    // it (spec 05 §5.12, spec 04 §4.8).
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let mut email = email_action("a-vision");
    email.execution_preferences.allowed_tiers = vec![lumi_protocol::ExecutionTier::Vision];
    email.execution_preferences.preferred_tier = Some(lumi_protocol::ExecutionTier::Vision);

    let env = FixtureEnvironment::new().with_record(
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/customer-42".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true, "subject": "Quote follow-up"}),
    );
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let outcome = orch.execute_step(
        &email,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert_eq!(calls.get(), 0, "vision fallback must not bypass approval");
    assert!(matches!(outcome, StepOutcome::ApprovalNeeded { .. }));

    // With approval, the vision-tier executor runs (conservatively allowed
    // only behind explicit human authorization).
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
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert_eq!(calls.get(), 1);
    assert!(matches!(outcome, StepOutcome::VerifiedSuccess { .. }));
}

#[test]
fn router_selection_failure_stops_the_step() {
    // No executor serves the capability at any allowed tier: the step
    // stops explicitly instead of degrading silently.
    let mut config = config();
    config.executors.clear();
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config, InMemoryStateStore::new());
    let email = email_action("a-norouter");
    let (mut executor, calls) = scripted(vec![ExecutionStatus::Success]);
    let outcome = orch.execute_step(
        &email,
        None,
        &Budget::default(),
        &mut ConsumedBudget::default(),
        &FixtureEnvironment::new(),
        &mut executor,
    );
    assert_eq!(calls.get(), 0);
    match outcome {
        StepOutcome::Stopped { reason } => assert!(reason.contains("router"), "{reason}"),
        other => panic!("expected Stopped, got {other:?}"),
    }
}
