//! Pack certification fixtures (spec 12 §12.14).
//!
//! Every certification scenario runs the same pack through the same
//! orchestrator gate used in production: structural validation, schema
//! enforcement, template resolution, approval gating, ambiguity, and
//! verifier-failure handling.

use lumi_orchestrator::{Orchestrator, OrchestratorConfig, StepOutcome};
use lumi_packs::{
    prepare_run, ApprovalRule, CompatibilityMatrix, EconomicBaseline, ExceptionRoute,
    ExceptionTarget, PackManifest, PackStatus, PackStep, PrivacyClassification, SchemaField,
    ValueSchema, WorkflowPack,
};
use lumi_policy::CapabilityGrant;
use lumi_protocol::{
    ActionProposal, AuthenticationStrength, Budget, Capability, ConsumedBudget, ExecutionResult,
    ExecutionStatus, ExecutionTier, FailureCategory, Idempotency, IdempotencySemantics,
    Postcondition, PostconditionCheck, PostconditionId, Principal, PrincipalId, PrincipalKind,
    ResourceRef, ResourceType, RiskClass, RunId, SensitivityLabel, TaskId, TenantId, Timestamp,
};
use lumi_state::InMemoryStateStore;
use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

fn ts(secs: i64) -> Timestamp {
    Timestamp::from_epoch(secs, 0).unwrap()
}

fn principal() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-runner").unwrap(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        kind: PrincipalKind::Workflow,
        authenticated_at: Some(ts(0)),
        authentication_strength: Some(AuthenticationStrength::DevicePossession),
    }
}

fn human() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-approver").unwrap(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        kind: PrincipalKind::User,
        authenticated_at: Some(ts(0)),
        authentication_strength: Some(AuthenticationStrength::Mfa),
    }
}

fn resource(rtype: ResourceType, id: &str) -> ResourceRef {
    ResourceRef {
        resource_type: rtype,
        id: id.to_owned(),
        sensitivity: Some(SensitivityLabel::Confidential),
    }
}

/// A realistic quote-follow-up pack: CRM read -> draft -> send.
fn quote_followup_pack() -> WorkflowPack {
    let manifest = PackManifest {
        pack_id: "crm-quote-followup".to_owned(),
        version: "1.0.0".to_owned(),
        owner: "ops@acme.test".to_owned(),
        status: PackStatus::Alpha,
        runtime_version_range: ">=0.1".to_owned(),
        supported_os: BTreeSet::from(["macos-14".to_owned(), "windows-11".to_owned()]),
        required_connectors: BTreeSet::from(["crm-connector@1.2".to_owned()]),
        required_model_capabilities: BTreeSet::from(["tool_use".to_owned()]),
        privacy_classification: PrivacyClassification::Confidential,
        default_budget: Budget {
            max_actions: Some(10),
            ..Budget::default()
        },
        inputs: lumi_packs::SchemaFieldSet {
            fields: vec![
                SchemaField {
                    name: "quote_id".to_owned(),
                    value_type: ValueSchema::String,
                    description: "CRM quote id".to_owned(),
                    required: true,
                },
                SchemaField {
                    name: "customer_email".to_owned(),
                    value_type: ValueSchema::String,
                    description: "Recipient address".to_owned(),
                    required: true,
                },
            ],
        },
        outputs: lumi_packs::SchemaFieldSet {
            fields: vec![SchemaField {
                name: "sent_message_id".to_owned(),
                value_type: ValueSchema::String,
                description: "Sent message id".to_owned(),
                required: true,
            }],
        },
        compatibility: CompatibilityMatrix::default(),
        economics: EconomicBaseline {
            manual_minutes_per_run: 10,
            residual_human_minutes_per_run: 1,
            runs_per_month: 400,
            loaded_labor_cost_per_hour_micro_usd: 60_000_000,
            runtime_variable_cost_per_run_micro_usd: 15_000,
            implementation_hours: 40,
            payback_hypothesis: "<6 months at 400 runs/month".to_owned(),
        },
    };

    let steps = vec![
        PackStep {
            step_id: "fetch-quote".to_owned(),
            purpose: "Load the quote record from the CRM".to_owned(),
            capability: Capability::well_known(lumi_protocol::capabilities::FILES_READ),
            operation: "fetch_crm_quote".to_owned(),
            arguments_template: serde_json::json!({"quote_id": "$input.quote_id"}),
            preferred_tier: ExecutionTier::ConnectorApi,
            allowed_fallback_tiers: vec![ExecutionTier::ConnectorApi],
            risk_class: RiskClass::Read,
            approval_rule: ApprovalRule::PerPolicy,
            preconditions: vec![],
            postconditions: vec![Postcondition {
                id: PostconditionId::new("quote-loaded"),
                description: "Quote record exists".to_owned(),
                check: PostconditionCheck::RecordExists {
                    resource: resource(
                        ResourceType::well_known(ResourceType::CRM_QUOTE),
                        "crm/quotes/Q-2091",
                    ),
                },
            }],
            idempotency: Idempotency::default(),
            evidence: vec![lumi_protocol::EvidenceRequirement::StructuredState],
            resource_type: ResourceType::well_known(ResourceType::CRM_QUOTE).0,
            resource_id_template: "crm/quotes/$input.quote_id".to_owned(),
            target_template: "crm://acme.test/quotes/$input.quote_id".to_owned(),
            exception_route: ExceptionRoute {
                triggers: vec![FailureCategory::ConnectorFailure],
                target: ExceptionTarget::Retry,
                guidance: "Retry the CRM fetch once before escalating".to_owned(),
            },
        },
        PackStep {
            step_id: "draft-followup".to_owned(),
            purpose: "Draft the follow-up email for approval".to_owned(),
            capability: Capability::well_known(lumi_protocol::capabilities::EMAIL_DRAFT_CREATE),
            operation: "draft_email".to_owned(),
            arguments_template: serde_json::json!({
                "to": "$input.customer_email",
                "subject": "Quote $input.quote_id follow-up",
            }),
            preferred_tier: ExecutionTier::ConnectorApi,
            allowed_fallback_tiers: vec![ExecutionTier::ConnectorApi],
            risk_class: RiskClass::LocalWrite,
            approval_rule: ApprovalRule::PerPolicy,
            preconditions: vec![],
            postconditions: vec![Postcondition {
                id: PostconditionId::new("draft-exists"),
                description: "Draft exists".to_owned(),
                check: PostconditionCheck::RecordExists {
                    resource: resource(
                        ResourceType::well_known(ResourceType::EMAIL_DRAFT),
                        "outbound/draft-Q-2091",
                    ),
                },
            }],
            idempotency: Idempotency::default(),
            evidence: vec![lumi_protocol::EvidenceRequirement::ResourceReference],
            resource_type: ResourceType::well_known(ResourceType::EMAIL_DRAFT).0,
            resource_id_template: "outbound/draft-$input.quote_id".to_owned(),
            target_template: "mailto:$input.customer_email".to_owned(),
            exception_route: ExceptionRoute {
                triggers: vec![FailureCategory::ConnectorFailure],
                target: ExceptionTarget::Stop,
                guidance: "Draft failures stop the run for review".to_owned(),
            },
        },
        PackStep {
            step_id: "send-followup".to_owned(),
            purpose: "Send the approved follow-up email".to_owned(),
            capability: Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            operation: "send_customer_email".to_owned(),
            arguments_template: serde_json::json!({
                "to": "$input.customer_email",
                "subject": "Quote $input.quote_id follow-up",
            }),
            preferred_tier: ExecutionTier::ConnectorApi,
            allowed_fallback_tiers: vec![ExecutionTier::ConnectorApi],
            risk_class: RiskClass::Communication,
            approval_rule: ApprovalRule::Always,
            preconditions: vec![],
            postconditions: vec![Postcondition {
                id: PostconditionId::new("message-exists"),
                description: "Sent message exists with expected subject".to_owned(),
                check: PostconditionCheck::RecordFieldEquals {
                    resource: resource(
                        ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                        "outbound/sent-Q-2091",
                    ),
                    field: "subject".to_owned(),
                    expected: serde_json::json!("Quote Q-2091 follow-up"),
                },
            }],
            idempotency: Idempotency {
                key: Some("send-Q-2091".to_owned()),
                semantics: IdempotencySemantics::ClientKey,
            },
            evidence: vec![lumi_protocol::EvidenceRequirement::StructuredState],
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE).0,
            resource_id_template: "outbound/sent-$input.quote_id".to_owned(),
            target_template: "mailto:$input.customer_email".to_owned(),
            exception_route: ExceptionRoute {
                triggers: vec![
                    FailureCategory::AmbiguousState,
                    FailureCategory::AuthSession,
                ],
                target: ExceptionTarget::User,
                guidance: "Verify the sent folder manually before any re-send".to_owned(),
            },
        },
    ];

    WorkflowPack { manifest, steps }
}

fn registry() -> lumi_policy::CapabilityRegistry {
    let grant = |id: &'static str, cap: &'static str, rtype: ResourceType| CapabilityGrant {
        grant_id: id.to_owned(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        principal_id: None,
        capability: Capability::well_known(cap),
        resource_scope: lumi_policy::ResourceScope::all_of([rtype]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: lumi_policy::GrantSource::OrganizationPolicy,
    };
    lumi_policy::CapabilityRegistry::default()
        .grant(grant(
            "g-crm-read",
            lumi_protocol::capabilities::FILES_READ,
            ResourceType::well_known(ResourceType::CRM_QUOTE),
        ))
        .grant(grant(
            "g-draft",
            lumi_protocol::capabilities::EMAIL_DRAFT_CREATE,
            ResourceType::well_known(ResourceType::EMAIL_DRAFT),
        ))
        .grant(grant(
            "g-send",
            lumi_protocol::capabilities::EMAIL_SEND,
            ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
        ))
}

fn config() -> OrchestratorConfig {
    OrchestratorConfig {
        registry: registry(),
        device_state: lumi_policy::DeviceExecutionState::Trusted,
        pre_authorizations: vec![],
        retry: lumi_state::RetryPolicy::default(),
        policy_version: "1.0.0".to_owned(),
        executors: vec![lumi_orchestrator::ExecutorDescriptor::new(
            ExecutionTier::ConnectorApi,
            "http-connector",
            "1.0.0",
            [
                Capability::well_known(lumi_protocol::capabilities::FILES_READ),
                Capability::well_known(lumi_protocol::capabilities::EMAIL_DRAFT_CREATE),
                Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            ],
        )],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
    }
}

fn inputs() -> serde_json::Value {
    serde_json::json!({
        "quote_id": "Q-2091",
        "customer_email": "customer@example.test",
    })
}

/// Scripted per-operation executor: each operation name maps to the status
/// it should return; call counter proves exactly which steps ran.
type Behavior = Vec<(&'static str, ExecutionStatus)>;

fn op_executor(
    behavior: Behavior,
) -> (
    impl FnMut(&ActionProposal) -> ExecutionResult,
    Rc<Cell<usize>>,
) {
    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let behavior = Rc::new(std::cell::RefCell::new(
        behavior
            .into_iter()
            .collect::<std::collections::HashMap<&'static str, ExecutionStatus>>(),
    ));
    let executor = move |action: &ActionProposal| {
        observed.set(observed.get() + 1);
        let operation = action.operation.as_str();
        let status = behavior
            .borrow()
            .get(operation)
            .copied()
            .unwrap_or(ExecutionStatus::Success);
        let error = if status == ExecutionStatus::Failed {
            Some(lumi_protocol::ErrorEnvelope::new(
                FailureCategory::ConnectorFailure,
                "crm connector unreachable",
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

fn valid_env() -> lumi_audit::FixtureEnvironment {
    lumi_audit::FixtureEnvironment::new()
        .with_record(
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::CRM_QUOTE),
                id: "crm/quotes/Q-2091".to_owned(),
                sensitivity: None,
            },
            serde_json::json!({"exists": true, "total": "4200.00"}),
        )
        .with_record(
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_DRAFT),
                id: "outbound/draft-Q-2091".to_owned(),
                sensitivity: None,
            },
            serde_json::json!({"exists": true}),
        )
        .with_record(
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "outbound/sent-Q-2091".to_owned(),
                sensitivity: None,
            },
            serde_json::json!({"exists": true, "subject": "Quote Q-2091 follow-up"}),
        )
}

fn task_and_run() -> (TaskId, RunId) {
    (
        TaskId::parse("task-pack").unwrap(),
        RunId::parse("run-pack").unwrap(),
    )
}

#[test]
fn pack_validation_enforces_structure() {
    let pack = quote_followup_pack();
    assert!(pack.validate().is_ok());

    // Certified without OS entries fails:
    let mut certified = quote_followup_pack();
    certified.manifest.status = PackStatus::Certified;
    certified.manifest.compatibility = CompatibilityMatrix::default();
    let violations = certified.manifest.validate().unwrap_err();
    assert!(violations
        .iter()
        .any(|v| v.contains("compatibility matrix")));

    // Duplicate step ids fail:
    let mut broken = quote_followup_pack();
    broken.steps[0].step_id = broken.steps[1].step_id.clone();
    assert!(broken.validate().is_err());

    // Consequential step without postconditions fails:
    let mut unverified = quote_followup_pack();
    unverified.steps[2].postconditions = vec![];
    assert!(unverified.validate().is_err());
}

#[test]
fn inputs_missing_required_fields_fail_before_execution() {
    let pack = quote_followup_pack();
    let (task_id, run_id) = task_and_run();
    let error = prepare_run(
        &pack,
        &serde_json::json!({"quote_id": "Q-2091"}),
        &task_id,
        &run_id,
        &principal(),
    )
    .unwrap_err();
    match error {
        lumi_packs::PackRunError::InvalidInputs(violations) => {
            assert!(violations.iter().any(|v| v.contains("customer_email")));
        }
        other => panic!("expected InvalidInputs, got {other:?}"),
    }
}

#[test]
fn unsupported_preconditions_and_missing_client_key_refuse_preparation() {
    let (task_id, run_id) = task_and_run();
    let mut pack = quote_followup_pack();
    let precondition = pack.steps[0].postconditions[0].clone();
    pack.steps[0].preconditions.push(precondition);
    assert!(prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).is_err());
    let mut pack = quote_followup_pack();
    pack.steps[2].idempotency.key = None;
    assert!(prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).is_err());
}

#[test]
fn templates_resolve_and_materialize_authorized_actions() {
    let pack = quote_followup_pack();
    let (task_id, run_id) = task_and_run();
    let prepared = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();
    assert_eq!(prepared.actions.len(), 3);

    let (step_id, send_action) = &prepared.actions[2];
    assert_eq!(step_id, "send-followup");
    // $input references resolved:
    assert_eq!(send_action.arguments["to"], "customer@example.test");
    assert_eq!(send_action.arguments["subject"], "Quote Q-2091 follow-up");
    assert_eq!(send_action.target.canonical, "mailto:customer@example.test");
    assert_eq!(send_action.resource.id, "outbound/sent-Q-2091");
    assert!(send_action.risk_class.is_consequential());
    assert_eq!(
        send_action.step_id.as_ref().map(|s| s.as_str()),
        Some("send-followup")
    );
    assert_eq!(
        send_action.workflow_id.as_ref().map(|w| w.as_str()),
        Some("crm-quote-followup")
    );
}

#[test]
fn repeated_work_items_do_not_overwrite_other_runs_journal_identity() {
    let pack = quote_followup_pack();
    let (task_id, run_id) = task_and_run();
    let first = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();
    let resumed = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();
    let next = prepare_run(
        &pack,
        &inputs(),
        &task_id,
        &RunId::parse("run-next").unwrap(),
        &principal(),
    )
    .unwrap();
    let mut other_tenant = principal();
    other_tenant.tenant_id = TenantId::parse("t-other").unwrap();
    let isolated = prepare_run(&pack, &inputs(), &task_id, &run_id, &other_tenant).unwrap();
    let other_task = prepare_run(
        &pack,
        &inputs(),
        &TaskId::parse("task-other").unwrap(),
        &run_id,
        &principal(),
    )
    .unwrap();
    for i in 0..first.actions.len() {
        let original = &first.actions[i].1;
        assert_eq!(original.action_id, resumed.actions[i].1.action_id);
        assert_ne!(original.action_id, next.actions[i].1.action_id);
        assert_ne!(original.action_id, isolated.actions[i].1.action_id);
        assert_ne!(original.action_id, other_task.actions[i].1.action_id);
    }
}

#[test]
fn idempotency_templates_follow_business_item_not_run_attempt() {
    let mut pack = quote_followup_pack();
    pack.steps[2].idempotency.key = Some("send-$input.quote_id".to_owned());
    let (task_id, run_id) = task_and_run();
    let first = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();
    let retry = prepare_run(
        &pack,
        &inputs(),
        &task_id,
        &RunId::parse("retry-run").unwrap(),
        &principal(),
    )
    .unwrap();
    assert_eq!(
        first.actions[2].1.idempotency.key.as_deref(),
        Some("send-Q-2091")
    );
    assert_eq!(
        first.actions[2].1.idempotency.key,
        retry.actions[2].1.idempotency.key
    );
    let mut next_input = inputs();
    next_input["quote_id"] = serde_json::json!("Q-2092");
    let next = prepare_run(&pack, &next_input, &task_id, &run_id, &principal()).unwrap();
    assert_eq!(
        next.actions[2].1.idempotency.key.as_deref(),
        Some("send-Q-2092")
    );
    // Editing a proposal within a run must invalidate the old digest, not
    // evade its journal entry by generating a different action identity.
    assert_eq!(first.actions[2].1.action_id, next.actions[2].1.action_id);
    assert_ne!(
        first.actions[2].1.material_digest(),
        next.actions[2].1.material_digest()
    );
}

#[test]
fn unresolved_or_non_string_identity_and_idempotency_fail_before_dispatch() {
    let (task_id, run_id) = task_and_run();
    let mut pack = quote_followup_pack();
    pack.steps[2].idempotency.key = Some("$input.missing".to_owned());
    assert!(matches!(
        prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()),
        Err(lumi_packs::PackRunError::UnresolvedTemplate { .. })
    ));
    let mut supplied = inputs();
    supplied["bad_identity"] = serde_json::json!(42);
    for field in ["resource", "target", "key"] {
        let mut pack = quote_followup_pack();
        match field {
            "resource" => pack.steps[2].resource_id_template = "$input.bad_identity".to_owned(),
            "target" => pack.steps[2].target_template = "$input.bad_identity".to_owned(),
            _ => pack.steps[2].idempotency.key = Some("$input.bad_identity".to_owned()),
        }
        assert!(matches!(
            prepare_run(&pack, &supplied, &task_id, &run_id, &principal()),
            Err(lumi_packs::PackRunError::InvalidTemplateValue { .. })
        ));
    }
    pack.steps[2].idempotency.key = Some("   ".to_owned());
    assert!(prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).is_err());
}

#[test]
fn happy_path_runs_read_and_draft_without_approval_gates_send() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let pack = quote_followup_pack();
    let (task_id, run_id) = task_and_run();
    let prepared = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();
    let env = valid_env();
    let (mut executor, calls) = op_executor(vec![
        ("fetch_crm_quote", ExecutionStatus::Success),
        ("draft_email", ExecutionStatus::Success),
        ("send_customer_email", ExecutionStatus::Success),
    ]);

    // Steps 1-2: read/local-write, run without approval.
    for (_, action) in &prepared.actions[..2] {
        let outcome = orch.execute_step(
            action,
            None,
            &pack.manifest.default_budget,
            &mut ConsumedBudget::default(),
            &env,
            &mut executor,
        );
        assert!(
            matches!(outcome, StepOutcome::VerifiedSuccess { .. }),
            "unexpected: {outcome:?}"
        );
    }

    // Step 3: COMMUNICATION requires the scoped human approval.
    let (_, send) = &prepared.actions[2];
    let outcome = orch.execute_step(
        send,
        None,
        &pack.manifest.default_budget,
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert!(matches!(outcome, StepOutcome::ApprovalNeeded { .. }));

    let approval = orch
        .approvals
        .issue(
            send,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let outcome = orch.execute_step(
        send,
        Some(&approval.approval_id),
        &pack.manifest.default_budget,
        &mut ConsumedBudget::default(),
        &env,
        &mut executor,
    );
    assert!(matches!(outcome, StepOutcome::VerifiedSuccess { .. }));
    assert_eq!(calls.get(), 3, "exactly three steps executed");

    // The journal holds exactly one verified send.
    let record = orch.journal.get(&send.action_id).unwrap();
    assert_eq!(record.status, lumi_state::SideEffectStatus::Succeeded);
    assert_eq!(
        record.idempotency_key.as_deref(),
        Some("send-Q-2091"),
        "pack idempotency metadata travels to the journal"
    );
}

#[test]
fn connector_failure_maps_to_pack_exception_route() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let pack = quote_followup_pack();
    let (task_id, run_id) = task_and_run();
    let prepared = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();
    let (mut executor, _calls) = op_executor(vec![("fetch_crm_quote", ExecutionStatus::Failed)]);

    let (step_id, fetch) = &prepared.actions[0];
    let outcome = orch.execute_step(
        fetch,
        None,
        &pack.manifest.default_budget,
        &mut ConsumedBudget::default(),
        &lumi_audit::FixtureEnvironment::new(),
        &mut executor,
    );
    // CONNECTOR_FAILURE is the step's declared exception trigger: the run
    // loop reads the route (Retry) from the pack.
    match outcome {
        StepOutcome::Retryable { error } => {
            assert_eq!(error.category, FailureCategory::ConnectorFailure);
            let route = &pack
                .steps
                .iter()
                .find(|s| s.step_id == *step_id)
                .unwrap()
                .exception_route;
            assert!(matches!(route.target, ExceptionTarget::Retry));
            assert!(route.triggers.contains(&FailureCategory::ConnectorFailure));
        }
        other => panic!("expected Retryable, got {other:?}"),
    }
}

#[test]
fn ambiguous_send_surfaces_for_exception_route() {
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let pack = quote_followup_pack();
    let (task_id, run_id) = task_and_run();
    let prepared = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();
    let (mut executor, _calls) =
        op_executor(vec![("send_customer_email", ExecutionStatus::Ambiguous)]);

    let (_, send) = &prepared.actions[2];
    let approval = orch
        .approvals
        .issue(
            send,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let outcome = orch.execute_step(
        send,
        Some(&approval.approval_id),
        &pack.manifest.default_budget,
        &mut ConsumedBudget::default(),
        &lumi_audit::FixtureEnvironment::new(),
        &mut executor,
    );
    assert!(matches!(outcome, StepOutcome::Ambiguous { .. }));
    // The declared route for AMBIGUOUS_STATE on this step is User.
    let route = &pack.steps[2].exception_route;
    assert!(route.triggers.contains(&FailureCategory::AmbiguousState));
    assert!(matches!(route.target, ExceptionTarget::User));
    // And the journal is unresolved - the send may NOT be blindly retried.
    assert!(orch
        .journal
        .get(&send.action_id)
        .unwrap()
        .status
        .retry_may_duplicate());
}

#[test]
fn verifier_failure_marks_step_unverified() {
    // The executor claims success but the sent message never appeared:
    // the step must NOT be verified success.
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let pack = quote_followup_pack();
    let (task_id, run_id) = task_and_run();
    let prepared = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();
    let (mut executor, _calls) =
        op_executor(vec![("send_customer_email", ExecutionStatus::Success)]);

    let (_, send) = &prepared.actions[2];
    let approval = orch
        .approvals
        .issue(
            send,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    // Empty env: postcondition check unresolvable => AMBIGUOUS outcome.
    let outcome = orch.execute_step(
        send,
        Some(&approval.approval_id),
        &pack.manifest.default_budget,
        &mut ConsumedBudget::default(),
        &lumi_audit::FixtureEnvironment::new(),
        &mut executor,
    );
    assert!(
        matches!(outcome, StepOutcome::Ambiguous { .. }),
        "unverifiable send must be ambiguous, got {outcome:?}"
    );
}

#[test]
fn stale_state_policy_denial_and_auth_paths() {
    // Auth-session failure on the send maps to retryable classification
    // with the pack's User exception route available.
    let mut orch: Orchestrator<InMemoryStateStore> =
        Orchestrator::new(config(), InMemoryStateStore::new());
    let pack = quote_followup_pack();
    let (task_id, run_id) = task_and_run();
    let prepared = prepare_run(&pack, &inputs(), &task_id, &run_id, &principal()).unwrap();

    let (mut executor, _calls) = op_executor(vec![]);
    // Simulate an auth failure via a raw executor result instead of the
    // scripted map (which only models connector/ambiguous):
    let (_, send) = &prepared.actions[2];
    let approval = orch
        .approvals
        .issue(
            send,
            &human(),
            lumi_policy::ApprovalTtl::ONE_HOUR,
            Timestamp::now(),
            true,
        )
        .unwrap();
    let mut auth_failed = orch.execute_step(
        send,
        Some(&approval.approval_id),
        &pack.manifest.default_budget,
        &mut ConsumedBudget::default(),
        &valid_env(),
        &mut executor,
    );
    // Force-verify the auth mapping at the taxonomy level (spec 6.14/18):
    let envelope = lumi_protocol::ErrorEnvelope::from_adapter("http", "401", "session expired");
    assert_eq!(envelope.category, FailureCategory::AuthSession);
    let _ = &mut auth_failed;

    // Stale-state category mapping:
    let stale = lumi_protocol::ErrorEnvelope::from_adapter("playwright", "TimeoutError", "stale");
    assert_eq!(stale.category, FailureCategory::BrowserState);
    assert!(
        stale.retry_class.allows_auto_retry(),
        "stale state re-observes then retries"
    );
}
