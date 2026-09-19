//! The orchestrator core.

use crate::router::{select, ExecutorDescriptor, Selection, TierPolicy};
use lumi_audit::{
    AuditEvent, AuditEventKind, AuditLedger, AuditScope, EvidenceKind, EvidenceRequest,
    EvidenceStore, PolicyOutcome, PostconditionVerifier, VerificationEnvironment,
    VerificationOutcome, VerificationStatus,
};
use lumi_policy::{
    evaluate, ApprovalLedger, ApprovalValidation, CapabilityRegistry, Decision,
    DeviceExecutionState, PolicyContext, PreAuthorization,
};
use lumi_protocol::{
    ActionId, ActionProposal, Budget, ConsumedBudget, ErrorEnvelope, ExecutionResult,
    ExecutionStatus, FailureCategory, SensitivityLabel, Timestamp,
};
use lumi_state::{
    resume, AmbiguousResolution, CancelToken, Checkpoint, PreActionCheckpoint, ResumeAction,
    ResumeContext, RetryController, RetryDecision, RetryPolicy, SideEffectJournal,
    SideEffectStatus, StateStore,
};

/// Static configuration for one orchestrator instance.
pub struct OrchestratorConfig {
    /// Policy layer: capability registry (deny-by-default).
    pub registry: CapabilityRegistry,
    /// Narrow pre-authorizations that may downgrade soft approval
    /// requirements (never hard gates).
    pub pre_authorizations: Vec<PreAuthorization>,
    /// Retry policy for failed (non-ambiguous) steps.
    pub retry: RetryPolicy,
    /// Policy version in force (recorded for resume compatibility checks).
    pub policy_version: String,
    /// Registered executors available for tier selection (spec 05).
    pub executors: Vec<ExecutorDescriptor>,
    /// Tier restrictions from tenant/deployment policy.
    pub tier_policy: TierPolicy,
}

/// What one step produced.
#[derive(Debug, Clone, PartialEq)]
pub enum StepOutcome {
    /// Verified success: executor succeeded AND postconditions passed (or
    /// none were required for a non-consequential action).
    VerifiedSuccess {
        action_id: ActionId,
        verification: VerificationStatus,
    },
    /// Policy demands a scoped approval bound to this digest.
    ApprovalNeeded {
        action_digest: String,
        reason: String,
    },
    /// Policy denied; nothing executed.
    Denied { reason: String },
    /// Executor failed within a retryable classification; the run loop
    /// should retry per policy.
    Retryable { error: ErrorEnvelope },
    /// The effect's outcome is unknown; blocked pending external
    /// verification via [`Orchestrator::resolve_ambiguity`].
    Ambiguous { action_id: ActionId },
    /// Not verified successful: postconditions failed or retries
    /// exhausted; routes to the human exception queue.
    Unverified { action_id: ActionId, detail: String },
    /// Budget exhausted or cancelled.
    Stopped { reason: String },
}

/// The local execution orchestrator.
///
/// Every material outcome appends a hash-chained audit event; every
/// consequential action passes the side-effect journal. This is the single
/// choke point through which all side effects flow.
pub struct Orchestrator<S: StateStore> {
    pub config: OrchestratorConfig,
    pub approvals: ApprovalLedger,
    pub audit: AuditLedger,
    pub evidence: EvidenceStore,
    pub store: S,
    pub journal: SideEffectJournal,
    pub cancel: CancelToken,
    verifier: PostconditionVerifier,
}

impl<S: StateStore> Orchestrator<S> {
    #[must_use]
    pub fn new(config: OrchestratorConfig, store: S) -> Self {
        Self {
            config,
            approvals: ApprovalLedger::new(),
            audit: AuditLedger::new(),
            evidence: EvidenceStore::new(),
            store,
            journal: SideEffectJournal::new(),
            cancel: CancelToken::new(),
            verifier: PostconditionVerifier,
        }
    }

    /// Rebuilds an orchestrator after a restart: the side-effect journal is
    /// reloaded from durable storage so ambiguity recovery sees records
    /// written before the crash. The audit chain itself persists via
    /// `lumi_audit::JsonlAuditLog` at the host layer.
    #[must_use]
    pub fn restore(config: OrchestratorConfig, store: S) -> Self {
        let mut orch = Self::new(config, store);
        if let Ok(journal) = orch.store.load_journal() {
            orch.journal = journal;
        }
        orch
    }

    /// Executes one step for a run through the full gated pipeline.
    ///
    /// `executor` is any callable receiving the *authorized* action and
    /// returning a normalized [`ExecutionResult`] — real executors
    /// (connector, browser, native, shell) implement this contract.
    #[allow(clippy::too_many_lines)]
    pub fn execute_step<E>(
        &mut self,
        action: &ActionProposal,
        approval_id: Option<&lumi_protocol::ApprovalId>,
        budget: &Budget,
        consumed: &mut ConsumedBudget,
        env: &dyn VerificationEnvironment,
        executor: &mut E,
    ) -> StepOutcome
    where
        E: FnMut(&ActionProposal) -> ExecutionResult,
    {
        let now = Timestamp::now();

        // 0. Cancellation before anything else (spec 02 §2.9).
        if self.cancel.is_cancelled() {
            return StepOutcome::Stopped {
                reason: "cancelled".to_owned(),
            };
        }

        // 1. Budget exhaustion produces a controlled stop (spec 01 §1.13).
        if let Some(dimension) = budget.check(consumed, now) {
            self.audit_action_event(
                action,
                &PolicyOutcome::Denied {
                    rule_id: "lumi.budget/v1".to_owned(),
                    reason: format!("{dimension:?}"),
                },
                None,
                Some(ExecutionStatus::Cancelled),
                VerificationStatus::NotRequired,
            );
            return StepOutcome::Stopped {
                reason: format!("budget exhausted: {dimension:?}"),
            };
        }

        // 2. Policy evaluation (deny-by-default; hard gates intact).
        // Deny decisions preempt everything; approval requirements are
        // enforced AFTER tier selection so a human is never asked to
        // approve an action no executor can perform.
        let policy_ctx = PolicyContext {
            now,
            device_state: DeviceExecutionState::Trusted,
            registry: Some(&self.config.registry),
        };
        let decision = evaluate(action, &self.config.pre_authorizations, &policy_ctx);
        if let Decision::Deny { reason, .. } = &decision {
            let text = reason.as_str().to_owned();
            self.audit_action_event(
                action,
                &PolicyOutcome::Denied {
                    rule_id: "lumi.policy.default/v1".to_owned(),
                    reason: text.clone(),
                },
                None,
                None,
                VerificationStatus::NotRequired,
            );
            return StepOutcome::Denied { reason: text };
        }

        // 3. Execution-tier selection (spec 05): highest-semantic
        // eligible executor; denied tiers fail explicitly. Selection
        // errors stop the step before anything is persisted or executed.
        let selection = match select(action, &self.config.executors, &self.config.tier_policy) {
            Ok(selection) => selection,
            Err(err) => {
                let reason = err.to_string();
                self.audit_action_event(
                    action,
                    &PolicyOutcome::Denied {
                        rule_id: "lumi.router/v1".to_owned(),
                        reason: reason.clone(),
                    },
                    None,
                    None,
                    VerificationStatus::NotRequired,
                );
                return StepOutcome::Stopped {
                    reason: format!("router: {reason}"),
                };
            }
        };

        // 4. Scoped approval validation + consumption (fail closed).
        if let Decision::RequireApproval {
            reason,
            action_digest,
            ..
        } = decision
        {
            let Some(ap_id) = approval_id else {
                self.audit_action_event_with_selection(
                    action,
                    &PolicyOutcome::ApprovalRequired {
                        rule_id: "lumi.policy.default/v1".to_owned(),
                        reason: reason.as_str().to_owned(),
                    },
                    None,
                    None,
                    VerificationStatus::NotRequired,
                    Some(&selection),
                );
                return StepOutcome::ApprovalNeeded {
                    action_digest,
                    reason: reason.as_str().to_owned(),
                };
            };
            match self.approvals.validate_and_consume(ap_id, action, now) {
                ApprovalValidation::Valid { .. } => {
                    self.audit_approval_consumed(action, ap_id);
                }
                ApprovalValidation::Invalid { reason: invalid } => {
                    // Invalid approval == no approval: fail closed.
                    return StepOutcome::ApprovalNeeded {
                        action_digest,
                        reason: invalid.as_str().to_owned(),
                    };
                }
            }
        }
        let policy_outcome = PolicyOutcome::Allowed {
            rule_id: "lumi.policy.default/v1".to_owned(),
        };

        // 5. Consequential actions: persist intent BEFORE executing
        // (spec 02 §2.6).
        let is_consequential = action.risk_class.is_consequential();
        if is_consequential {
            if let Ok(pre) = PreActionCheckpoint::from_action(action, None) {
                self.store.save_pre_action(&pre).ok();
            }
            self.journal.propose(action, now);
            self.store.save_journal(&self.journal).ok();
        }

        // 5. Execute.
        let result = executor(action);
        consumed.record_action(
            action
                .execution_preferences
                .allowed_tiers
                .iter()
                .any(|t| t.is_vision()),
            is_consequential,
        );

        // 6. Postcondition verification determines success (spec 11 §11.8).
        match result.status {
            ExecutionStatus::Success => {
                let (status, outcomes) = self.verifier.verify_all(&action.postconditions, env);
                self.record_verification_evidence(action, status, &outcomes, env);
                match status {
                    VerificationStatus::Passed => {
                        self.journalize_resolution(action, SideEffectStatus::Succeeded);
                        self.audit_action_event_with_selection(
                            action,
                            &policy_outcome,
                            None,
                            Some(ExecutionStatus::Success),
                            VerificationStatus::Passed,
                            Some(&selection),
                        );
                        StepOutcome::VerifiedSuccess {
                            action_id: action.action_id.clone(),
                            verification: status,
                        }
                    }
                    VerificationStatus::NotRequired if !is_consequential => {
                        self.audit_action_event_with_selection(
                            action,
                            &policy_outcome,
                            None,
                            Some(ExecutionStatus::Success),
                            VerificationStatus::NotRequired,
                            Some(&selection),
                        );
                        StepOutcome::VerifiedSuccess {
                            action_id: action.action_id.clone(),
                            verification: VerificationStatus::NotRequired,
                        }
                    }
                    // A consequential action with no verifiable postcondition
                    // is, by definition, of unknown effect: ambiguous.
                    VerificationStatus::NotRequired | VerificationStatus::Ambiguous => {
                        self.journalize_ambiguous(action);
                        self.audit_action_event_with_selection(
                            action,
                            &policy_outcome,
                            None,
                            Some(ExecutionStatus::Success),
                            VerificationStatus::Ambiguous,
                            Some(&selection),
                        );
                        StepOutcome::Ambiguous {
                            action_id: action.action_id.clone(),
                        }
                    }
                    VerificationStatus::Failed => {
                        // Executor said success; the world disagrees. The
                        // step is NOT verified-successful (spec 11 §11.8).
                        self.journalize_failure(
                            action,
                            ErrorEnvelope::new(
                                FailureCategory::Postcondition,
                                "postcondition failed after executor success",
                            ),
                        );
                        self.audit_action_event_with_selection(
                            action,
                            &policy_outcome,
                            None,
                            Some(ExecutionStatus::Success),
                            VerificationStatus::Failed,
                            Some(&selection),
                        );
                        StepOutcome::Unverified {
                            action_id: action.action_id.clone(),
                            detail: outcomes
                                .iter()
                                .map(|o| format!("{}: {:?}", o.postcondition_id, o.status))
                                .collect::<Vec<_>>()
                                .join("; "),
                        }
                    }
                }
            }
            ExecutionStatus::Failed => {
                let error = result.error.clone().unwrap_or_else(|| {
                    ErrorEnvelope::new(
                        FailureCategory::ConnectorFailure,
                        "executor failed without envelope",
                    )
                });
                if is_consequential {
                    self.journalize_failure(action, error.clone());
                }
                // Classify recovery through the bounded retry controller.
                let decision = RetryController::new(self.config.retry.clone())
                    .on_failure(&error, Timestamp::now());
                self.audit_action_event_with_selection(
                    action,
                    &policy_outcome,
                    None,
                    Some(ExecutionStatus::Failed),
                    VerificationStatus::NotRequired,
                    Some(&selection),
                );
                match decision {
                    RetryDecision::RetryAfter(_)
                    | RetryDecision::ReobserveThenRetry(_)
                    | RetryDecision::RefreshAuthThenRetry => StepOutcome::Retryable { error },
                    RetryDecision::VerifyBeforeRetry => StepOutcome::Ambiguous {
                        action_id: action.action_id.clone(),
                    },
                    RetryDecision::EscalateToHuman | RetryDecision::RequireApproval => {
                        StepOutcome::Unverified {
                            action_id: action.action_id.clone(),
                            detail: format!("escalated: {}", error.message),
                        }
                    }
                    RetryDecision::Terminal => StepOutcome::Stopped {
                        reason: format!("terminal: {}", error.message),
                    },
                }
            }
            ExecutionStatus::Ambiguous => {
                if is_consequential {
                    self.journalize_ambiguous(action);
                }
                self.audit_action_event_with_selection(
                    action,
                    &policy_outcome,
                    None,
                    Some(ExecutionStatus::Ambiguous),
                    VerificationStatus::Ambiguous,
                    Some(&selection),
                );
                StepOutcome::Ambiguous {
                    action_id: action.action_id.clone(),
                }
            }
            ExecutionStatus::Cancelled => StepOutcome::Stopped {
                reason: "executor reported cancellation".to_owned(),
            },
        }
    }

    /// Resolves an ambiguous side effect by verifying its external
    /// postconditions and updating journal + audit (spec 02 §2.7,
    /// spec 18 §18.11). Never re-executes on `ConfirmedApplied`.
    pub fn resolve_ambiguity(
        &mut self,
        action: &ActionProposal,
        env: &dyn VerificationEnvironment,
    ) -> AmbiguousResolution {
        let (status, _outcomes) = self.verifier.verify_all(&action.postconditions, env);
        let resolution = match status {
            VerificationStatus::Passed => AmbiguousResolution::ConfirmedApplied,
            VerificationStatus::Failed => AmbiguousResolution::ConfirmedNotApplied,
            VerificationStatus::Ambiguous | VerificationStatus::NotRequired => {
                AmbiguousResolution::StillUnknown
            }
        };
        if resolution != AmbiguousResolution::StillUnknown {
            self.journal
                .resolve_ambiguous(
                    &action.action_id,
                    resolution,
                    Some(action.material_digest()),
                    None,
                    Timestamp::now(),
                )
                .ok();
            self.store.save_journal(&self.journal).ok();
        }
        self.audit_event_inner(
            AuditScope {
                tenant_id: action.principal.tenant_id.clone(),
                task_id: action.task_id.clone(),
                run_id: action.run_id.clone(),
                workflow: None,
            },
            AuditEventKind::Verified {
                action_id: Some(action.action_id.clone()),
                postcondition_id: "ambiguity-resolution".to_owned(),
                verification: match resolution {
                    AmbiguousResolution::ConfirmedApplied => VerificationStatus::Passed,
                    AmbiguousResolution::ConfirmedNotApplied => VerificationStatus::Failed,
                    AmbiguousResolution::StillUnknown => VerificationStatus::Ambiguous,
                },
                detail: format!("ambiguity resolved to {resolution:?}"),
            },
            Timestamp::now(),
        );
        resolution
    }

    /// Runs the crash-recovery procedure (spec 02 §2.8) for a run.
    #[must_use]
    pub fn plan_resume(
        &self,
        task: &lumi_protocol::Task,
        run: &lumi_protocol::Run,
        pending_approval: Option<ApprovalValidation>,
        verified_resolutions: std::collections::HashMap<ActionId, AmbiguousResolution>,
        policy_version_at_start: &str,
    ) -> Vec<ResumeAction> {
        let checkpoint = self.store.load_checkpoint(&run.run_id).ok();
        resume(
            task,
            run,
            checkpoint.as_ref(),
            &self.journal,
            &ResumeContext {
                now: Timestamp::now(),
                runtime_version: env!("CARGO_PKG_VERSION"),
                policy_version: &self.config.policy_version.clone(),
                policy_version_at_start,
                device_ready: true,
                pending_approval,
                verified_resolutions,
            },
        )
    }

    /// Persists a checkpoint for the run.
    pub fn checkpoint(&mut self, checkpoint: &Checkpoint) {
        self.store.save_checkpoint(checkpoint).ok();
    }

    fn journalize_resolution(&mut self, action: &ActionProposal, status: SideEffectStatus) {
        if !action.risk_class.is_consequential() {
            return;
        }
        match status {
            SideEffectStatus::Succeeded => {
                self.journal
                    .mark_succeeded(
                        &action.action_id,
                        action.material_digest(),
                        Timestamp::now(),
                    )
                    .ok();
            }
            _ => self.journalize_ambiguous(action),
        }
        self.store.save_journal(&self.journal).ok();
    }

    fn journalize_ambiguous(&mut self, action: &ActionProposal) {
        if action.risk_class.is_consequential() {
            self.journal.mark_ambiguous(&action.action_id).ok();
            self.store.save_journal(&self.journal).ok();
        }
    }

    fn journalize_failure(&mut self, action: &ActionProposal, error: ErrorEnvelope) {
        if action.risk_class.is_consequential() {
            self.journal
                .mark_failed(&action.action_id, error, Timestamp::now())
                .ok();
            self.store.save_journal(&self.journal).ok();
        }
    }

    fn audit_action_event(
        &mut self,
        action: &ActionProposal,
        policy: &PolicyOutcome,
        approval_id: Option<&lumi_protocol::ApprovalId>,
        execution_status: Option<ExecutionStatus>,
        verification: VerificationStatus,
    ) {
        self.audit_action_event_with_selection(
            action,
            policy,
            approval_id,
            execution_status,
            verification,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn audit_action_event_with_selection(
        &mut self,
        action: &ActionProposal,
        policy: &PolicyOutcome,
        approval_id: Option<&lumi_protocol::ApprovalId>,
        execution_status: Option<ExecutionStatus>,
        verification: VerificationStatus,
        selection: Option<&Selection>,
    ) {
        let kind = AuditEventKind::Action(Box::new(lumi_audit::ActionEventDetails {
            action_id: action.action_id.clone(),
            step_id: action.step_id.clone(),
            principal: action.principal.clone(),
            capability: action.capability.0.clone(),
            operation: action.operation.clone(),
            resource: action.resource.clone(),
            target: action.target.clone(),
            risk_class: action.risk_class,
            execution_tier: selection.map(|s| s.tier),
            adapter: selection.map(|s| s.adapter.clone()),
            adapter_version: selection.map(|s| s.version.clone()),
            policy: policy.clone(),
            approval_id: approval_id.cloned(),
            action_digest: action.material_digest(),
            execution_status,
            verification,
            failure: None,
            failure_category: None,
        }));
        self.audit_event_inner(
            AuditScope {
                tenant_id: action.principal.tenant_id.clone(),
                task_id: action.task_id.clone(),
                run_id: action.run_id.clone(),
                workflow: None,
            },
            kind,
            Timestamp::now(),
        );
    }

    fn audit_approval_consumed(
        &mut self,
        action: &ActionProposal,
        approval_id: &lumi_protocol::ApprovalId,
    ) {
        // Approval consumption linkage (spec 11 §11.11): digest + approver
        // travel with the approval event; consumption is recorded here.
        let kind = AuditEventKind::Verified {
            action_id: Some(action.action_id.clone()),
            postcondition_id: "approval-consumed".to_owned(),
            verification: VerificationStatus::NotRequired,
            detail: format!("approval {approval_id} validated against material digest"),
        };
        self.audit_event_inner(
            AuditScope {
                tenant_id: action.principal.tenant_id.clone(),
                task_id: action.task_id.clone(),
                run_id: action.run_id.clone(),
                workflow: None,
            },
            kind,
            Timestamp::now(),
        );
    }

    fn audit_event_inner(&mut self, scope: AuditScope, kind: AuditEventKind, at: Timestamp) {
        let prev = self.audit.head_hash().to_owned();
        let event = AuditEvent::new(scope, kind, Vec::new(), &prev, at);
        self.audit.append(event).ok();
    }

    fn record_verification_evidence(
        &mut self,
        action: &ActionProposal,
        status: VerificationStatus,
        outcomes: &[VerificationOutcome],
        env: &dyn VerificationEnvironment,
    ) {
        if outcomes.is_empty() {
            return;
        }
        if let Some(state) = env.resolve_record(&action.resource) {
            let sensitivity = action
                .resource
                .sensitivity
                .unwrap_or(SensitivityLabel::Internal);
            self.evidence
                .record(EvidenceRequest {
                    tenant_id: action.principal.tenant_id.clone(),
                    kind: EvidenceKind::StructuredState,
                    sensitivity,
                    payload: serde_json::json!({
                        "task_id": action.task_id.as_str(),
                        "action_id": action.action_id.as_str(),
                        "resource": action.resource.id,
                        "verification": format!("{status:?}"),
                        "state": state,
                    }),
                    redaction_rules: vec![],
                    created_at: Timestamp::now(),
                    retention_until: None,
                    action_requirements: action.evidence_requirements.clone(),
                })
                .ok();
        }
    }
}
