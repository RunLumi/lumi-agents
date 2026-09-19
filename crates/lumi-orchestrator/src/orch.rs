//! The orchestrator core.

use crate::router::{select, ExecutorDescriptor, Selection, TierPolicy};
use lumi_audit::{
    workspace_file_checksum, AuditEvent, AuditEventKind, AuditLedger, AuditScope, EvidenceKind,
    EvidenceRequest, EvidenceStore, PolicyOutcome, PostconditionVerifier, VerificationEnvironment,
    VerificationOutcome, VerificationStatus,
};
use lumi_policy::{
    evaluate, ApprovalLedger, ApprovalValidation, CapabilityRegistry, Decision,
    DeviceExecutionState, PolicyContext, PreAuthorization,
};
use lumi_protocol::{
    ActionId, ActionProposal, Budget, ConsumedBudget, ErrorEnvelope, EvidenceRequirement,
    ExecutionResult, ExecutionStatus, FailureCategory, Postcondition, PostconditionCheck,
    SensitivityLabel, Timestamp,
};
use lumi_state::{
    idempotency_scope_digest, resume, verifier_plan_digest, AmbiguousResolution, CancelToken,
    Checkpoint, PreActionCheckpoint, ResumeAction, ResumeContext, RetryController, RetryDecision,
    RetryPolicy, SideEffectJournal, SideEffectStatus, StateStore,
};

/// Static configuration for one orchestrator instance.
pub struct OrchestratorConfig {
    /// Policy layer: capability registry (deny-by-default).
    pub registry: CapabilityRegistry,
    /// Host-enrolled device trust state. This is evaluated at every action
    /// gate; it is never inferred from a model, connector, or workflow.
    /// Callers without enrollment must provide `Unregistered`.
    pub device_state: DeviceExecutionState,
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
    /// none were required for a non-consequential action). The tool's
    /// observation payload (file contents, shell output) rides along for
    /// the agent loop / UX.
    VerifiedSuccess {
        action_id: ActionId,
        verification: VerificationStatus,
        observation: Option<String>,
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

/// The tool's observation payload for feedback into agent loops.
fn executor_observation(result: &ExecutionResult) -> Option<String> {
    if result.observation_ids.is_empty() {
        None
    } else {
        Some(result.observation_ids.join("\n"))
    }
}

fn validate_execution_result(
    action: &ActionProposal,
    result: &ExecutionResult,
) -> Result<(), String> {
    if result.action_id != action.action_id {
        return Err("executor result action_id does not match proposal".to_owned());
    }
    if result.task_id != action.task_id {
        return Err("executor result task_id does not match proposal".to_owned());
    }
    if result.run_id != action.run_id {
        return Err("executor result run_id does not match proposal".to_owned());
    }
    result.validate().map_err(|error| error.to_string())
}

fn validate_evidence_requirements(action: &ActionProposal) -> Result<(), String> {
    // v1 treats the declared list as conjunctive: every requirement must
    // produce its own evidence record. Preference rank is for pack authors;
    // it never permits substituting a weaker kind for a missing requirement.
    for requirement in &action.evidence_requirements {
        let supported = match requirement {
            // These kinds require at least one declared postcondition whose
            // actual verifier observation can supply the reference/state.
            EvidenceRequirement::StructuredState | EvidenceRequirement::ResourceReference => {
                !action.postconditions.is_empty()
            }
            EvidenceRequirement::Checksum => action.postconditions.iter().any(|postcondition| {
                matches!(
                    postcondition.check,
                    PostconditionCheck::FileChecksum { .. }
                        | PostconditionCheck::FileExists { .. }
                        | PostconditionCheck::ArtifactValid { .. }
                )
            }),
            EvidenceRequirement::Diff => action.postconditions.iter().any(|postcondition| {
                matches!(
                    postcondition.check,
                    PostconditionCheck::RecordFieldEquals { .. }
                )
            }),
            // The current verifier has no controlled source for these kinds.
            // Refuse before dispatch instead of manufacturing evidence.
            EvidenceRequirement::LogExcerpt
            | EvidenceRequirement::SelectiveScreenshot
            | EvidenceRequirement::FullScreenshot => false,
        };
        if !supported {
            return Err(format!("{requirement:?} is unsupported for this action"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct DiffBeforeProof {
    postcondition_id: String,
    field: String,
    value_hash: String,
}

fn capture_diff_before(
    action: &ActionProposal,
    env: &dyn VerificationEnvironment,
) -> Result<Vec<DiffBeforeProof>, String> {
    let mut proofs = Vec::new();
    for postcondition in &action.postconditions {
        let PostconditionCheck::RecordFieldEquals {
            resource, field, ..
        } = &postcondition.check
        else {
            continue;
        };
        let state = env.resolve_record(resource).ok_or_else(|| {
            format!(
                "before-state unavailable for postcondition {}",
                postcondition.id
            )
        })?;
        let actual = state.get(field).ok_or_else(|| {
            format!(
                "before-field {field:?} unavailable for postcondition {}",
                postcondition.id
            )
        })?;
        proofs.push(DiffBeforeProof {
            postcondition_id: postcondition.id.0.clone(),
            field: field.clone(),
            value_hash: json_hash(actual),
        });
    }
    if proofs.is_empty() {
        return Err("DIFF requires a RecordFieldEquals postcondition".to_owned());
    }
    Ok(proofs)
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
    /// A restore failure is sticky.  The compatibility `restore` constructor
    /// still returns an orchestrator, but every subsequent execution stops
    /// before policy/executor work until the host handles the persistence
    /// failure.  This prevents a missing/corrupt journal from becoming an
    /// implicit permission to replay consequential work.
    persistence_error: Option<String>,
}

impl<S: StateStore> Orchestrator<S> {
    #[must_use]
    pub fn new(config: OrchestratorConfig, store: S) -> Self {
        let mut orch = Self::empty(config, store);
        if let Err(error) = orch.try_load_journal() {
            orch.persistence_error = Some(error.to_string());
        }
        orch
    }

    fn empty(config: OrchestratorConfig, store: S) -> Self {
        Self {
            config,
            approvals: ApprovalLedger::new(),
            audit: AuditLedger::new(),
            evidence: EvidenceStore::new(),
            store,
            journal: SideEffectJournal::new(),
            cancel: CancelToken::new(),
            verifier: PostconditionVerifier,
            persistence_error: None,
        }
    }

    /// Rebuilds an orchestrator after a restart: the side-effect journal is
    /// reloaded from durable storage so ambiguity recovery sees records
    /// written before the crash. The audit chain itself persists via
    /// `lumi_audit::JsonlAuditLog` at the host layer.
    #[must_use]
    pub fn restore(config: OrchestratorConfig, store: S) -> Self {
        Self::new(config, store)
    }

    /// Restores an orchestrator and surfaces durable-state failures to the
    /// caller.  [`Self::restore`] remains available for existing callers and
    /// fails closed at the first execution gate when loading fails.
    pub fn try_restore(
        config: OrchestratorConfig,
        store: S,
    ) -> Result<Self, lumi_state::StoreError> {
        let mut orch = Self::empty(config, store);
        orch.try_load_journal()?;
        Ok(orch)
    }

    fn try_load_journal(&mut self) -> Result<(), lumi_state::StoreError> {
        self.journal = self.store.load_journal()?;
        Ok(())
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
        self.execute_step_with_mode(action, approval_id, budget, consumed, env, executor, false)
    }

    /// Executes a step that requires a fresh scoped approval even when the
    /// ordinary policy decision is ALLOW.  This is for workflow/role steps
    /// declaring `approval_rule: always`; policy DENY still wins and the
    /// approval is validated/consumed exactly once by the same gate.
    pub fn execute_step_requiring_approval<E>(
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
        self.execute_step_with_mode(action, approval_id, budget, consumed, env, executor, true)
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_step_with_mode<E>(
        &mut self,
        action: &ActionProposal,
        approval_id: Option<&lumi_protocol::ApprovalId>,
        budget: &Budget,
        consumed: &mut ConsumedBudget,
        env: &dyn VerificationEnvironment,
        executor: &mut E,
        force_approval: bool,
    ) -> StepOutcome
    where
        E: FnMut(&ActionProposal) -> ExecutionResult,
    {
        let mut now = Timestamp::now();

        // A failed restore is a durable-state failure, not a reason to run
        // with an empty journal.  Stop before any action can reach policy or
        // an executor.
        if let Some(reason) = &self.persistence_error {
            return StepOutcome::Stopped {
                reason: format!("durable state unavailable: {reason}"),
            };
        }

        // 0. Cancellation before anything else (spec 02 §2.9).
        if self.cancel.is_cancelled() {
            return StepOutcome::Stopped {
                reason: "cancelled".to_owned(),
            };
        }

        // Retry admissions are authoritative in the run-scoped state store;
        // a caller-provided ConsumedBudget may only rehydrate to that value,
        // never reset or advance it silently.
        if let Err(error) = self.reconcile_retry_attempts(action, consumed) {
            self.persistence_error = Some(error.to_string());
            return StepOutcome::Stopped {
                reason: format!("retry state unavailable: {error}"),
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

        // An existing action id is checked before policy/approval or
        // execution.  This blocks direct replay, including pack step IDs
        // accidentally reused by another run, and also blocks a caller from
        // downgrading a previously consequential action to READ/LOCAL_WRITE.
        if let Some(replay) = self.replay_guard(action) {
            return replay;
        }

        // 2. Policy evaluation (deny-by-default; hard gates intact).
        // Deny decisions preempt everything; approval requirements are
        // enforced AFTER tier selection so a human is never asked to
        // approve an action no executor can perform.
        let policy_ctx = PolicyContext {
            now,
            device_state: self.config.device_state,
            registry: Some(&self.config.registry),
        };
        let mut decision = evaluate(action, &self.config.pre_authorizations, &policy_ctx);
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

        // Evidence requirements are part of the action contract. Reject
        // kinds this runtime cannot produce before selecting or invoking an
        // executor; supported kinds are checked again against actual
        // verifier observations after dispatch.
        if let Err(reason) = validate_evidence_requirements(action) {
            self.audit_action_event(
                action,
                &PolicyOutcome::Denied {
                    rule_id: "lumi.evidence/v1".to_owned(),
                    reason: reason.clone(),
                },
                None,
                None,
                VerificationStatus::NotRequired,
            );
            return StepOutcome::Stopped {
                reason: format!("evidence requirement unavailable: {reason}"),
            };
        }

        // DIFF evidence needs a real before observation. Capture it after
        // policy has allowed the action but before router/approval work can
        // delay the observation and before any executor dispatch.
        let diff_before = if action
            .evidence_requirements
            .contains(&EvidenceRequirement::Diff)
        {
            match capture_diff_before(action, env) {
                Ok(proofs) => Some(proofs),
                Err(reason) => {
                    self.audit_action_event(
                        action,
                        &PolicyOutcome::Allowed {
                            rule_id: "lumi.policy.default/v1".to_owned(),
                        },
                        None,
                        None,
                        VerificationStatus::NotRequired,
                    );
                    return StepOutcome::Stopped {
                        reason: format!("required DIFF before-state unavailable: {reason}"),
                    };
                }
            }
        } else {
            None
        };

        if diff_before.is_some() {
            // The before read may cross a deadline, grant expiry, or
            // cancellation request. Re-authorize with a fresh timestamp
            // before any approval is consumed or executor is selected.
            if self.cancel.is_cancelled() {
                self.audit_action_event(
                    action,
                    &PolicyOutcome::Allowed {
                        rule_id: "lumi.policy.default/v1".to_owned(),
                    },
                    None,
                    None,
                    VerificationStatus::NotRequired,
                );
                return StepOutcome::Stopped {
                    reason: "cancelled after DIFF before-state observation".to_owned(),
                };
            }
            now = Timestamp::now();
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
                    reason: format!("budget exhausted after DIFF observation: {dimension:?}"),
                };
            }
            let fresh_policy_ctx = PolicyContext {
                now,
                device_state: self.config.device_state,
                registry: Some(&self.config.registry),
            };
            decision = evaluate(action, &self.config.pre_authorizations, &fresh_policy_ctx);
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

        // 4. Scoped approval validation + consumption (fail closed). A
        // workflow/role may force this gate even when policy otherwise
        // allows the action; a policy DENY returned above still wins.
        let approval_requirement = match decision {
            Decision::RequireApproval {
                reason,
                action_digest,
                ..
            } => Some((action_digest, reason.as_str().to_owned())),
            Decision::Allow { .. } if force_approval => Some((
                action.material_digest(),
                "workflow step requires explicit approval".to_owned(),
            )),
            _ => None,
        };
        if let Some((action_digest, reason)) = approval_requirement {
            let Some(ap_id) = approval_id else {
                self.audit_action_event_with_selection(
                    action,
                    &PolicyOutcome::ApprovalRequired {
                        rule_id: "lumi.policy.default/v1".to_owned(),
                        reason: reason.clone(),
                    },
                    None,
                    None,
                    VerificationStatus::NotRequired,
                    Some(&selection),
                );
                return StepOutcome::ApprovalNeeded {
                    action_digest,
                    reason,
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
            let pre = match PreActionCheckpoint::from_action(action, approval_id.cloned()) {
                Ok(pre) => pre,
                Err(error) => {
                    return StepOutcome::Stopped {
                        reason: format!("cannot persist pre-action intent: {error}"),
                    };
                }
            };
            if let Err(error) = self.store.save_pre_action(&pre) {
                self.persistence_error = Some(error.to_string());
                return StepOutcome::Stopped {
                    reason: format!("cannot persist pre-action intent: {error}"),
                };
            }

            // Do not leave a locally proposed record behind when the durable
            // journal write fails.  The executor must never run without the
            // durable intent barrier.
            let previous_journal = self.journal.clone();
            self.journal.propose(action, now);
            if let Err(error) = self.store.save_journal(&self.journal) {
                self.journal = previous_journal;
                self.persistence_error = Some(error.to_string());
                return StepOutcome::Stopped {
                    reason: format!("cannot persist side-effect intent: {error}"),
                };
            }
        }

        // Cancellation can arrive while policy/approval/intent persistence
        // is in flight. Recheck at the final pre-dispatch boundary so a
        // cancelled action never reaches the executor.
        if self.cancel.is_cancelled() {
            self.audit_action_event_with_selection_and_error(
                action,
                &policy_outcome,
                approval_id,
                None,
                VerificationStatus::NotRequired,
                Some(ErrorEnvelope::new(
                    FailureCategory::UserCancel,
                    "cancelled before executor dispatch",
                )),
                Some(&selection),
            );
            return StepOutcome::Stopped {
                reason: "cancelled before executor dispatch".to_owned(),
            };
        }

        // 5. Execute.
        let result = executor(action);
        // Every dispatch consumes budget, even if a lower-trust executor
        // returns malformed data. Count the selected tier, not an unused
        // fallback listed in the proposal.
        consumed.record_action(selection.tier.is_vision(), is_consequential);
        if let Err(error) = validate_execution_result(action, &result) {
            if is_consequential {
                if let Err(persist_error) = self.journalize_ambiguous(action) {
                    return StepOutcome::Stopped {
                        reason: format!(
                            "malformed executor result and ambiguity persistence failed: {persist_error}"
                        ),
                    };
                }
                self.audit_action_event_with_selection_and_error(
                    action,
                    &PolicyOutcome::Allowed {
                        rule_id: "lumi.policy.default/v1".to_owned(),
                    },
                    None,
                    None,
                    VerificationStatus::Ambiguous,
                    Some(ErrorEnvelope::new(FailureCategory::ModelFormat, error)),
                    Some(&selection),
                );
                return StepOutcome::Ambiguous {
                    action_id: action.action_id.clone(),
                };
            }
            return StepOutcome::Stopped {
                reason: format!("malformed executor result: {error}"),
            };
        }
        // 6. Postcondition verification determines success (spec 11 §11.8).
        match result.status {
            ExecutionStatus::Success => {
                let (status, outcomes) = self.verifier.verify_all(&action.postconditions, env);
                let evidence_refs = match self.record_verification_evidence(
                    action,
                    status,
                    &outcomes,
                    env,
                    diff_before.as_deref(),
                ) {
                    Ok(refs) => refs,
                    Err(reason)
                        if matches!(
                            status,
                            VerificationStatus::Passed | VerificationStatus::NotRequired
                        ) =>
                    {
                        if is_consequential {
                            if let Err(error) = self.journalize_ambiguous(action) {
                                return StepOutcome::Stopped {
                                    reason: format!(
                                        "required evidence unavailable and ambiguity state persistence failed: {error}"
                                    ),
                                };
                            }
                        }
                        let failure = ErrorEnvelope::new(
                            FailureCategory::Postcondition,
                            format!("required evidence unavailable: {reason}"),
                        );
                        self.audit_action_event_with_selection_and_error(
                            action,
                            &policy_outcome,
                            None,
                            Some(ExecutionStatus::Success),
                            VerificationStatus::Ambiguous,
                            Some(failure),
                            Some(&selection),
                        );
                        return if is_consequential {
                            StepOutcome::Ambiguous {
                                action_id: action.action_id.clone(),
                            }
                        } else {
                            StepOutcome::Unverified {
                                action_id: action.action_id.clone(),
                                detail: reason,
                            }
                        };
                    }
                    // A failed or ambiguous verifier result is already
                    // non-successful; preserve its existing outcome while
                    // avoiding unreferenced/unsupported proof records.
                    Err(_) => Vec::new(),
                };
                match status {
                    VerificationStatus::Passed => {
                        if let Err(error) =
                            self.journalize_resolution(action, SideEffectStatus::Succeeded)
                        {
                            self.audit_action_event_with_selection_and_error_and_evidence(
                                action,
                                &policy_outcome,
                                None,
                                Some(ExecutionStatus::Success),
                                VerificationStatus::Ambiguous,
                                Some(ErrorEnvelope::new(
                                    FailureCategory::Filesystem,
                                    format!("post-effect state persistence failed: {error}"),
                                )),
                                evidence_refs,
                                Some(&selection),
                            );
                            return StepOutcome::Stopped {
                                reason: format!(
                                    "post-effect state persistence failed; refusing success: {error}"
                                ),
                            };
                        }
                        self.audit_action_event_with_selection_and_evidence(
                            action,
                            &policy_outcome,
                            None,
                            Some(ExecutionStatus::Success),
                            VerificationStatus::Passed,
                            evidence_refs,
                            Some(&selection),
                        );
                        StepOutcome::VerifiedSuccess {
                            action_id: action.action_id.clone(),
                            verification: status,
                            observation: executor_observation(&result),
                        }
                    }
                    VerificationStatus::NotRequired if !is_consequential => {
                        self.audit_action_event_with_selection_and_evidence(
                            action,
                            &policy_outcome,
                            None,
                            Some(ExecutionStatus::Success),
                            VerificationStatus::NotRequired,
                            evidence_refs,
                            Some(&selection),
                        );
                        StepOutcome::VerifiedSuccess {
                            action_id: action.action_id.clone(),
                            verification: VerificationStatus::NotRequired,
                            observation: executor_observation(&result),
                        }
                    }
                    // A consequential action with no verifiable postcondition
                    // is, by definition, of unknown effect: ambiguous.
                    VerificationStatus::NotRequired | VerificationStatus::Ambiguous => {
                        if let Err(error) = self.journalize_ambiguous(action) {
                            self.audit_action_event_with_selection_and_error_and_evidence(
                                action,
                                &policy_outcome,
                                None,
                                Some(ExecutionStatus::Success),
                                VerificationStatus::Ambiguous,
                                Some(ErrorEnvelope::new(
                                    FailureCategory::Filesystem,
                                    format!("ambiguous effect state persistence failed: {error}"),
                                )),
                                evidence_refs,
                                Some(&selection),
                            );
                            return StepOutcome::Stopped {
                                reason: format!(
                                    "ambiguous effect state persistence failed: {error}"
                                ),
                            };
                        }
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
                        // Executor said success; a failed postcondition does
                        // not prove the external effect was absent. The
                        // record remains ambiguous so retry requires an
                        // independent external-state resolution.
                        let postcondition_error = ErrorEnvelope::new(
                            FailureCategory::Postcondition,
                            "postcondition failed after executor success",
                        );
                        if let Err(error) = self.journalize_ambiguous(action) {
                            self.audit_action_event_with_selection_and_error_and_evidence(
                                action,
                                &policy_outcome,
                                None,
                                Some(ExecutionStatus::Success),
                                VerificationStatus::Ambiguous,
                                Some(ErrorEnvelope::new(
                                    FailureCategory::Filesystem,
                                    format!(
                                        "postcondition failure state persistence failed: {error}"
                                    ),
                                )),
                                evidence_refs,
                                Some(&selection),
                            );
                            return StepOutcome::Stopped {
                                reason: format!(
                                    "postcondition failure state persistence failed: {error}"
                                ),
                            };
                        }
                        self.audit_action_event_with_selection_and_error_and_evidence(
                            action,
                            &policy_outcome,
                            None,
                            Some(ExecutionStatus::Success),
                            VerificationStatus::Failed,
                            Some(postcondition_error),
                            evidence_refs,
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
                    if let Err(persist_error) = self.journalize_ambiguous(action) {
                        return StepOutcome::Stopped {
                            reason: format!(
                                "executor ambiguity state persistence failed; refusing retry: {persist_error}"
                            ),
                        };
                    }
                    self.audit_action_event_with_selection_and_error(
                        action,
                        &policy_outcome,
                        None,
                        Some(ExecutionStatus::Failed),
                        VerificationStatus::Ambiguous,
                        Some(error.clone()),
                        Some(&selection),
                    );
                    return StepOutcome::Ambiguous {
                        action_id: action.action_id.clone(),
                    };
                }
                // Classify recovery through a fresh controller, but carry
                // the durable/run-scoped retry count in `ConsumedBudget` so
                // repeated execute_step calls and restarts cannot reset the
                // retry ceiling.
                let remaining_attempts = self
                    .config
                    .retry
                    .max_attempts
                    .saturating_sub(consumed.retries);
                let mut retry_policy = self.config.retry.clone();
                retry_policy.max_attempts = remaining_attempts;
                let decision = RetryController::new(retry_policy).on_failure(&error, now);
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
                    | RetryDecision::RefreshAuthThenRetry => {
                        let next_attempts = consumed.retries.saturating_add(1);
                        if let Err(persist_error) =
                            self.persist_retry_admission(action, next_attempts)
                        {
                            return StepOutcome::Stopped {
                                reason: format!(
                                    "retry state persistence failed; refusing retry: {persist_error}"
                                ),
                            };
                        }
                        consumed.record_retry();
                        StepOutcome::Retryable { error }
                    }
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
                    if let Err(error) = self.journalize_ambiguous(action) {
                        return StepOutcome::Stopped {
                            reason: format!("ambiguous effect state persistence failed: {error}"),
                        };
                    }
                }
                self.audit_action_event_with_selection_and_error(
                    action,
                    &policy_outcome,
                    None,
                    Some(ExecutionStatus::Ambiguous),
                    VerificationStatus::Ambiguous,
                    result.error.clone(),
                    Some(&selection),
                );
                StepOutcome::Ambiguous {
                    action_id: action.action_id.clone(),
                }
            }
            ExecutionStatus::Cancelled => {
                if is_consequential {
                    if let Err(error) = self.journalize_ambiguous(action) {
                        return StepOutcome::Stopped {
                            reason: format!(
                                "cancellation ambiguity state persistence failed: {error}"
                            ),
                        };
                    }
                    self.audit_action_event_with_selection_and_error(
                        action,
                        &policy_outcome,
                        None,
                        Some(ExecutionStatus::Cancelled),
                        VerificationStatus::Ambiguous,
                        Some(ErrorEnvelope::new(
                            FailureCategory::UserCancel,
                            "executor reported cancellation during consequential action",
                        )),
                        Some(&selection),
                    );
                    return StepOutcome::Ambiguous {
                        action_id: action.action_id.clone(),
                    };
                }
                StepOutcome::Stopped {
                    reason: "executor reported cancellation".to_owned(),
                }
            }
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
        let Some(record) = self.journal.get(&action.action_id) else {
            return AmbiguousResolution::StillUnknown;
        };
        // Bind the oracle query to the exact persisted intent and verifier
        // plan. A caller cannot swap postconditions or action fields and use
        // a readback to resolve an unrelated side effect.
        if !matches!(
            record.status,
            SideEffectStatus::Proposed | SideEffectStatus::Ambiguous
        ) || record.task_id != Some(action.task_id.clone())
            || record.run_id != action.run_id
            || record.action_digest != action.material_digest()
            || record.tenant_id != Some(action.principal.tenant_id.clone())
            || record.principal_id != Some(action.principal.principal_id.clone())
            || record.idempotency_key != action.idempotency.key
            || record.risk_class != Some(action.risk_class)
            || record.resource_sensitivity != action.resource.sensitivity
            || record.idempotency_scope_digest != idempotency_scope_digest(action)
            || record.verifier_plan_digest != Some(verifier_plan_digest(action))
        {
            return AmbiguousResolution::StillUnknown;
        }
        let (status, _outcomes) = self.verifier.verify_all(&action.postconditions, env);
        let resolution = match status {
            VerificationStatus::Passed => AmbiguousResolution::ConfirmedApplied,
            // A failed equality/exists check only says the requested
            // postcondition is not currently observed. It does not prove an
            // external write was absent, so keep the action blocked.
            VerificationStatus::Failed => AmbiguousResolution::StillUnknown,
            VerificationStatus::Ambiguous | VerificationStatus::NotRequired => {
                AmbiguousResolution::StillUnknown
            }
        };
        if resolution != AmbiguousResolution::StillUnknown {
            let mut next = self.journal.clone();
            let persisted = next
                .resolve_ambiguous(
                    &action.action_id,
                    resolution,
                    Some(action.material_digest()),
                    None,
                    Timestamp::now(),
                )
                .map_err(|error| lumi_state::StoreError::Io(format!("journal invariant: {error}")))
                .and_then(|_| self.store.save_journal(&next));
            if let Err(error) = persisted {
                // The external state was observed, but the durable
                // resolution was not committed. Keep the action blocked so a
                // restart cannot mistake the observation for permission to
                // replay it.
                self.persistence_error = Some(error.to_string());
                return AmbiguousResolution::StillUnknown;
            }
            self.journal = next;
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
            Vec::new(),
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
        let checkpoint = match self.store.load_checkpoint(&run.run_id) {
            Ok(checkpoint) => Some(checkpoint),
            Err(lumi_state::StoreError::NotFound(_)) => None,
            Err(error) => {
                return vec![ResumeAction::Blocked {
                    reason: format!("durable checkpoint unavailable: {error}"),
                }];
            }
        };
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
    pub fn checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), lumi_state::StoreError> {
        let result = self.store.save_checkpoint(checkpoint);
        if let Err(error) = &result {
            self.persistence_error = Some(error.to_string());
        }
        result
    }

    fn reconcile_retry_attempts(
        &self,
        action: &ActionProposal,
        consumed: &mut ConsumedBudget,
    ) -> Result<(), lumi_state::StoreError> {
        let persisted = self.store.load_retry_attempts(
            &action.principal.tenant_id,
            &action.run_id,
            &action.action_id,
        )?;
        match persisted {
            Some(attempts) if consumed.retries > attempts => {
                Err(lumi_state::StoreError::StaleWriter {
                    expected: attempts as u64,
                    actual: consumed.retries as u64,
                })
            }
            Some(attempts) => {
                consumed.retries = attempts;
                Ok(())
            }
            None if consumed.retries != 0 => Err(lumi_state::StoreError::Io(
                "caller retry count has no durable record".to_owned(),
            )),
            None => Ok(()),
        }
    }

    fn persist_retry_admission(
        &mut self,
        action: &ActionProposal,
        attempts: u32,
    ) -> Result<(), lumi_state::StoreError> {
        if let Err(error) = self.store.save_retry_attempts(
            &action.principal.tenant_id,
            &action.run_id,
            &action.action_id,
            attempts,
        ) {
            self.persistence_error = Some(error.to_string());
            return Err(error);
        }
        Ok(())
    }

    /// Checks whether a consequential action may enter the executor.  An
    /// existing action id is a durable replay key: a different run, digest,
    /// or idempotency key is rejected, while Proposed/Ambiguous records stay
    /// blocked until explicit external verification.
    fn replay_guard(&self, action: &ActionProposal) -> Option<StepOutcome> {
        let record = self.journal.get(&action.action_id).cloned();
        if let Some(record) = &record {
            let digest = action.material_digest();
            if record.task_id != Some(action.task_id.clone())
                || record.run_id != action.run_id
                || record.action_digest != digest
                || record.idempotency_key != action.idempotency.key
                || record.risk_class != Some(action.risk_class)
                || record.resource_sensitivity != action.resource.sensitivity
                || record.idempotency_scope_digest != idempotency_scope_digest(action)
            {
                return Some(StepOutcome::Stopped {
                    reason: format!(
                        "replay blocked: action {} is already bound to another run or material intent",
                        action.action_id
                    ),
                });
            }
            // An action id already used for a consequential operation cannot
            // be reused as a lower-risk action, even when its material fields
            // happen to match. This closes the early-return downgrade bypass.
            if !action.risk_class.is_consequential() {
                return Some(StepOutcome::Stopped {
                    reason: format!(
                        "replay blocked: action {} was already journaled as consequential",
                        action.action_id
                    ),
                });
            }
        }
        if !action.risk_class.is_consequential() {
            return None;
        }
        if let Some(key) = action.idempotency.key.as_deref() {
            let requested_scope = idempotency_scope_digest(action);
            for candidate in self.journal.all() {
                if candidate.idempotency_key.as_deref() != Some(key)
                    || candidate.action_id == action.action_id
                {
                    continue;
                }
                let Some(candidate_scope) = candidate.idempotency_scope_digest.as_ref() else {
                    return Some(StepOutcome::Stopped {
                        reason: format!(
                            "replay blocked: idempotency key {key:?} has an older record with unknown business scope"
                        ),
                    });
                };
                if Some(candidate_scope) != requested_scope.as_ref() {
                    continue;
                }
                match candidate.status {
                    SideEffectStatus::Succeeded => {
                        return Some(StepOutcome::Stopped {
                            reason: format!(
                                "replay blocked: idempotency key {key:?} already succeeded in this business scope"
                            ),
                        });
                    }
                    SideEffectStatus::Proposed | SideEffectStatus::Ambiguous => {
                        return Some(StepOutcome::Ambiguous {
                            action_id: action.action_id.clone(),
                        });
                    }
                    SideEffectStatus::Failed => {}
                }
            }
        }
        match record.as_ref().map(|record| record.status) {
            None => None,
            Some(SideEffectStatus::Succeeded) => Some(StepOutcome::Stopped {
                reason: format!(
                    "replay blocked: consequential action {} already succeeded",
                    action.action_id
                ),
            }),
            Some(SideEffectStatus::Proposed | SideEffectStatus::Ambiguous) => {
                Some(StepOutcome::Ambiguous {
                    action_id: action.action_id.clone(),
                })
            }
            Some(SideEffectStatus::Failed) => None,
        }
    }

    fn journalize_resolution(
        &mut self,
        action: &ActionProposal,
        status: SideEffectStatus,
    ) -> Result<(), lumi_state::StoreError> {
        if !action.risk_class.is_consequential() {
            return Ok(());
        }
        if status != SideEffectStatus::Succeeded {
            return self.journalize_ambiguous(action);
        }
        let mut next = self.journal.clone();
        next.mark_succeeded(
            &action.action_id,
            action.material_digest(),
            Timestamp::now(),
        )
        .map_err(|error| lumi_state::StoreError::Io(format!("journal invariant: {error}")))?;
        if let Err(error) = self.store.save_journal(&next) {
            self.persistence_error = Some(error.to_string());
            return Err(error);
        }
        self.journal = next;
        Ok(())
    }

    fn journalize_ambiguous(
        &mut self,
        action: &ActionProposal,
    ) -> Result<(), lumi_state::StoreError> {
        if !action.risk_class.is_consequential() {
            return Ok(());
        }
        let mut next = self.journal.clone();
        next.mark_ambiguous(&action.action_id)
            .map_err(|error| lumi_state::StoreError::Io(format!("journal invariant: {error}")))?;
        if let Err(error) = self.store.save_journal(&next) {
            self.persistence_error = Some(error.to_string());
            return Err(error);
        }
        self.journal = next;
        Ok(())
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
        self.audit_action_event_with_selection_and_error(
            action,
            policy,
            approval_id,
            execution_status,
            verification,
            None,
            selection,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn audit_action_event_with_selection_and_evidence(
        &mut self,
        action: &ActionProposal,
        policy: &PolicyOutcome,
        approval_id: Option<&lumi_protocol::ApprovalId>,
        execution_status: Option<ExecutionStatus>,
        verification: VerificationStatus,
        evidence_refs: Vec<String>,
        selection: Option<&Selection>,
    ) {
        self.audit_action_event_with_selection_and_error_and_evidence(
            action,
            policy,
            approval_id,
            execution_status,
            verification,
            None,
            evidence_refs,
            selection,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn audit_action_event_with_selection_and_error(
        &mut self,
        action: &ActionProposal,
        policy: &PolicyOutcome,
        approval_id: Option<&lumi_protocol::ApprovalId>,
        execution_status: Option<ExecutionStatus>,
        verification: VerificationStatus,
        failure: Option<ErrorEnvelope>,
        selection: Option<&Selection>,
    ) {
        self.audit_action_event_with_selection_and_error_and_evidence(
            action,
            policy,
            approval_id,
            execution_status,
            verification,
            failure,
            Vec::new(),
            selection,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn audit_action_event_with_selection_and_error_and_evidence(
        &mut self,
        action: &ActionProposal,
        policy: &PolicyOutcome,
        approval_id: Option<&lumi_protocol::ApprovalId>,
        execution_status: Option<ExecutionStatus>,
        verification: VerificationStatus,
        failure: Option<ErrorEnvelope>,
        evidence_refs: Vec<String>,
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
            failure_category: failure.as_ref().map(|error| error.category),
            failure,
        }));
        self.audit_event_inner(
            AuditScope {
                tenant_id: action.principal.tenant_id.clone(),
                task_id: action.task_id.clone(),
                run_id: action.run_id.clone(),
                workflow: None,
            },
            kind,
            evidence_refs,
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
            Vec::new(),
            Timestamp::now(),
        );
    }

    fn audit_event_inner(
        &mut self,
        scope: AuditScope,
        kind: AuditEventKind,
        evidence_refs: Vec<String>,
        at: Timestamp,
    ) {
        let prev = self.audit.head_hash().to_owned();
        let event = AuditEvent::new(scope, kind, evidence_refs, &prev, at);
        self.audit.append(event).ok();
    }

    fn record_verification_evidence(
        &mut self,
        action: &ActionProposal,
        status: VerificationStatus,
        outcomes: &[VerificationOutcome],
        env: &dyn VerificationEnvironment,
        diff_before: Option<&[DiffBeforeProof]>,
    ) -> Result<Vec<String>, String> {
        if action.evidence_requirements.is_empty() {
            return Ok(Vec::new());
        }

        // Build every proof before appending any record. A missing required
        // proof therefore cannot leave unreferenced partial evidence behind.
        let payloads = action
            .evidence_requirements
            .iter()
            .map(|requirement| {
                let kind = EvidenceKind::from_requirement(*requirement);
                let payload = evidence_payload(action, kind, status, outcomes, env, diff_before)
                    .ok_or_else(|| format!("{requirement:?} proof unavailable"))?;
                Ok((kind, payload))
            })
            .collect::<Result<Vec<_>, String>>()?;

        let sensitivity = evidence_sensitivity(action);
        let mut evidence_refs = Vec::with_capacity(payloads.len());
        for (kind, payload) in payloads {
            let evidence_id = self
                .evidence
                .record(EvidenceRequest {
                    tenant_id: action.principal.tenant_id.clone(),
                    kind,
                    sensitivity,
                    payload,
                    redaction_rules: vec![],
                    created_at: Timestamp::now(),
                    retention_until: None,
                    action_requirements: action.evidence_requirements.clone(),
                })
                .map_err(|error| format!("{kind:?}: {error:?}"))?;
            evidence_refs.push(evidence_id.as_str().to_owned());
        }
        Ok(evidence_refs)
    }
}

fn evidence_payload(
    action: &ActionProposal,
    kind: EvidenceKind,
    status: VerificationStatus,
    outcomes: &[VerificationOutcome],
    env: &dyn VerificationEnvironment,
    diff_before: Option<&[DiffBeforeProof]>,
) -> Option<serde_json::Value> {
    let passed = action
        .postconditions
        .iter()
        .zip(outcomes)
        .filter(|(_, outcome)| outcome.status == VerificationStatus::Passed);

    let proofs: Vec<serde_json::Value> = match kind {
        EvidenceKind::StructuredState => passed
            .filter_map(|(postcondition, _)| structured_state_proof(postcondition, env))
            .collect(),
        EvidenceKind::ResourceReference => passed
            .map(|(postcondition, _)| resource_reference_proof(postcondition))
            .collect(),
        EvidenceKind::Checksum => passed
            .filter_map(|(postcondition, _)| checksum_proof(postcondition, env))
            .collect(),
        EvidenceKind::Diff => passed
            .filter_map(|(postcondition, _)| diff_proof(postcondition, env, diff_before?))
            .collect(),
        // These kinds are rejected by the pre-dispatch validation above.
        EvidenceKind::LogExcerpt
        | EvidenceKind::SelectiveScreenshot
        | EvidenceKind::FullScreenshot => Vec::new(),
    };

    (!proofs.is_empty()).then(|| {
        serde_json::json!({
            "task_id": action.task_id.as_str(),
            "action_id": action.action_id.as_str(),
            "verification": format!("{status:?}"),
            "proofs": proofs,
        })
    })
}

fn evidence_sensitivity(action: &ActionProposal) -> SensitivityLabel {
    let mut sensitivity = action
        .resource
        .sensitivity
        .unwrap_or(SensitivityLabel::Internal);
    for postcondition in &action.postconditions {
        let candidate = match &postcondition.check {
            PostconditionCheck::RecordExists { resource }
            | PostconditionCheck::RecordFieldEquals { resource, .. } => resource.sensitivity,
            _ => None,
        };
        if let Some(candidate) = candidate {
            if sensitivity_rank(candidate) > sensitivity_rank(sensitivity) {
                sensitivity = candidate;
            }
        }
    }
    sensitivity
}

fn sensitivity_rank(label: SensitivityLabel) -> u8 {
    match label {
        SensitivityLabel::Public => 0,
        SensitivityLabel::Internal => 1,
        SensitivityLabel::Confidential => 2,
        SensitivityLabel::Restricted => 3,
        SensitivityLabel::PersonalData => 4,
    }
}

fn resource_reference_proof(postcondition: &Postcondition) -> serde_json::Value {
    let reference = match &postcondition.check {
        PostconditionCheck::RecordExists { resource }
        | PostconditionCheck::RecordFieldEquals { resource, .. } => serde_json::json!({
            "resource_type": resource.resource_type.0,
            "resource_id": resource.id,
        }),
        PostconditionCheck::FileChecksum { path, .. } | PostconditionCheck::FileExists { path } => {
            serde_json::json!({
                "workspace_path": path,
            })
        }
        PostconditionCheck::ArtifactValid { artifact_id } => serde_json::json!({
            "artifact_id": artifact_id.as_str(),
        }),
        PostconditionCheck::RemoteStateMatches { probe_id, .. } => serde_json::json!({
            "probe_id": probe_id,
        }),
        PostconditionCheck::Custom { verifier_id, .. } => serde_json::json!({
            "verifier_id": verifier_id,
        }),
    };
    serde_json::json!({
        "postcondition_id": postcondition.id.0,
        "reference": reference,
    })
}

fn structured_state_proof(
    postcondition: &Postcondition,
    env: &dyn VerificationEnvironment,
) -> Option<serde_json::Value> {
    let proof = match &postcondition.check {
        PostconditionCheck::RecordExists { resource } => {
            let state = env.resolve_record(resource)?;
            if state.get("exists").and_then(serde_json::Value::as_bool) != Some(true) {
                return None;
            }
            serde_json::json!({
                "check": "record_exists",
                "resource_type": resource.resource_type.0,
                "resource_id": resource.id,
                "observed_exists": state.get("exists").and_then(serde_json::Value::as_bool),
            })
        }
        PostconditionCheck::RecordFieldEquals {
            resource,
            field,
            expected,
        } => {
            let state = env.resolve_record(resource)?;
            let actual = state.get(field)?;
            if lumi_protocol::canonical::canonical_json(actual)
                != lumi_protocol::canonical::canonical_json(expected)
            {
                return None;
            }
            serde_json::json!({
                "check": "record_field_equals",
                "resource_type": resource.resource_type.0,
                "resource_id": resource.id,
                "field": field,
                "actual_hash": json_hash(actual),
                "expected_hash": json_hash(expected),
            })
        }
        PostconditionCheck::FileChecksum { path, sha256 } => {
            let actual = file_checksum(env, path)?;
            if actual != *sha256 {
                return None;
            }
            serde_json::json!({
                "check": "file_checksum",
                "path": path,
                "sha256": actual,
                "expected_sha256": sha256,
            })
        }
        PostconditionCheck::FileExists { path } => {
            let root = env.workspace_root()?;
            if !lumi_audit::safe_workspace_path(root, path).is_some_and(|path| path.is_file()) {
                return None;
            }
            serde_json::json!({
                "check": "file_exists",
                "path": path,
                "exists": true,
            })
        }
        PostconditionCheck::ArtifactValid { artifact_id } => {
            let artifact = env.artifact(artifact_id)?;
            if artifact.validation_status != lumi_protocol::ValidationStatus::Valid {
                return None;
            }
            serde_json::json!({
                "check": "artifact_valid",
                "artifact_id": artifact_id.as_str(),
                "validation_status": format!("{:?}", artifact.validation_status),
                "sha256": artifact.sha256,
            })
        }
        PostconditionCheck::RemoteStateMatches { probe_id, expected } => {
            let actual = env.run_probe(probe_id)?;
            if lumi_protocol::canonical::canonical_json(&actual)
                != lumi_protocol::canonical::canonical_json(expected)
            {
                return None;
            }
            serde_json::json!({
                "check": "remote_state_matches",
                "probe_id": probe_id,
                "actual_hash": json_hash(&actual),
                "expected_hash": json_hash(expected),
            })
        }
        PostconditionCheck::Custom {
            verifier_id,
            params,
        } => {
            let verification = env.run_custom(verifier_id, params)?;
            if verification != VerificationStatus::Passed {
                return None;
            }
            serde_json::json!({
                "check": "custom",
                "verifier_id": verifier_id,
                "result": format!("{verification:?}"),
            })
        }
    };
    Some(serde_json::json!({
        "postcondition_id": postcondition.id.0,
        "proof": proof,
    }))
}

fn checksum_proof(
    postcondition: &Postcondition,
    env: &dyn VerificationEnvironment,
) -> Option<serde_json::Value> {
    match &postcondition.check {
        PostconditionCheck::FileChecksum { path, sha256 } => {
            let actual = file_checksum(env, path)?;
            if actual != *sha256 {
                return None;
            }
            Some(serde_json::json!({
                "postcondition_id": postcondition.id.0,
                "path": path,
                "sha256": actual,
                "expected_sha256": sha256,
            }))
        }
        PostconditionCheck::FileExists { path } => {
            let sha256 = file_checksum(env, path)?;
            Some(serde_json::json!({
                "postcondition_id": postcondition.id.0,
                "path": path,
                "sha256": sha256,
            }))
        }
        PostconditionCheck::ArtifactValid { artifact_id } => {
            let artifact = env.artifact(artifact_id)?;
            if artifact.validation_status != lumi_protocol::ValidationStatus::Valid {
                return None;
            }
            let sha256 = artifact.sha256?;
            Some(serde_json::json!({
                "postcondition_id": postcondition.id.0,
                "artifact_id": artifact_id.as_str(),
                "sha256": sha256,
            }))
        }
        _ => None,
    }
}

fn diff_proof(
    postcondition: &Postcondition,
    env: &dyn VerificationEnvironment,
    before: &[DiffBeforeProof],
) -> Option<serde_json::Value> {
    let PostconditionCheck::RecordFieldEquals {
        resource,
        field,
        expected,
    } = &postcondition.check
    else {
        return None;
    };
    let before = before
        .iter()
        .find(|proof| proof.postcondition_id == postcondition.id.0 && proof.field == *field)?;
    let state = env.resolve_record(resource)?;
    let actual = state.get(field)?;
    if lumi_protocol::canonical::canonical_json(actual)
        != lumi_protocol::canonical::canonical_json(expected)
    {
        return None;
    }
    let after_hash = json_hash(actual);
    let changed = before.value_hash != after_hash;
    Some(serde_json::json!({
        "postcondition_id": postcondition.id.0,
        "resource_type": resource.resource_type.0,
        "resource_id": resource.id,
        "field": field,
        "before_hash": before.value_hash,
        "after_hash": after_hash,
        "changed": changed,
        "expected_hash": json_hash(expected),
        "comparison": "observed_before_vs_observed_after",
    }))
}

fn file_checksum(env: &dyn VerificationEnvironment, path: &str) -> Option<String> {
    let root = env.workspace_root()?;
    workspace_file_checksum(root, path)
}

fn json_hash(value: &serde_json::Value) -> String {
    lumi_protocol::canonical::sha256_hex(lumi_protocol::canonical::canonical_json(value).as_bytes())
}
