//! Local execution boundary.
//!
//! Executor adapters live behind this crate so upstream automation engines
//! can be replaced without changing Lumi's workflow or policy contracts.
//!
//! Invariant (AGENTS.md): models may propose, policy authorizes, executors
//! act. [`dispatch`] is the choke point that enforces the order — an
//! executor never sees an action that policy did not permit (or that a
//! human did not approve when policy required it).

use lumi_policy::{ApprovalLedger, ApprovalValidation, Decision};
use lumi_protocol::{ActionId, ActionProposal, ExecutionResult, Timestamp};

/// An executor receives already-authorized actions only (spec 05 §5.8).
pub trait Executor {
    /// Executes one authorized action and reports its outcome. Implementors
    /// MUST NOT self-authorize and MUST treat the action as immutable.
    fn execute(&mut self, action: &ActionProposal) -> ExecutionResult;
}

/// Outcome of a gated dispatch.
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchOutcome {
    /// Policy allowed (or a validated approval covered) the action and the
    /// executor ran it. The result is boxed to keep the denied/approval
    /// variants cheap.
    Executed { result: Box<ExecutionResult> },
    /// Policy demands a scoped human approval bound to `action_digest`.
    ApprovalRequired {
        action_id: ActionId,
        action_digest: String,
        reason: String,
    },
    /// Policy denied the action. Executors are never invoked.
    Denied { action_id: ActionId, reason: String },
}

impl DispatchOutcome {
    /// True only when the executor ran the action (executor-level status
    /// inside `result` may still be FAILED/AMBIGUOUS).
    #[must_use]
    pub const fn executed(&self) -> bool {
        matches!(self, Self::Executed { .. })
    }
}

/// Evaluates policy and, only on success, hands the action to the
/// executor.
///
/// When `approval_id` is provided it is validated against the ledger and
/// consumed (if single-use) before execution; an invalid approval fails
/// closed — the executor is not invoked.
pub fn dispatch<E: Executor>(
    policy: impl Fn(&ActionProposal) -> Decision,
    registry: &mut ApprovalLedger,
    executor: &mut E,
    action: &ActionProposal,
    approval_id: Option<&lumi_protocol::ApprovalId>,
    now: Timestamp,
) -> DispatchOutcome {
    match policy(action) {
        Decision::Allow { .. } => DispatchOutcome::Executed {
            result: Box::new(executor.execute(action)),
        },
        Decision::Deny { reason, .. } => DispatchOutcome::Denied {
            action_id: action.action_id.clone(),
            reason: reason.as_str().to_owned(),
        },
        Decision::RequireApproval {
            reason,
            action_digest,
            ..
        } => {
            let Some(approval_id) = approval_id else {
                return DispatchOutcome::ApprovalRequired {
                    action_id: action.action_id.clone(),
                    action_digest,
                    reason: reason.as_str().to_owned(),
                };
            };
            let validation = registry.validate_and_consume(approval_id, action, now);
            match validation {
                ApprovalValidation::Valid { .. } => DispatchOutcome::Executed {
                    result: Box::new(executor.execute(action)),
                },
                ApprovalValidation::Invalid { reason: invalid } => {
                    // Fail closed: an invalid approval is no approval.
                    DispatchOutcome::ApprovalRequired {
                        action_id: action.action_id.clone(),
                        action_digest,
                        reason: invalid.as_str().to_owned(),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_policy::{
        ApprovalTtl, CapabilityGrant, CapabilityRegistry, DeviceExecutionState, GrantSource,
        PolicyContext, ResourceScope,
    };
    use lumi_protocol::{
        AuthenticationStrength, Capability, ExecutionStatus, Principal, PrincipalId, ResourceRef,
        ResourceType, RiskClass, RunId, Target, TaskId, TenantId,
    };
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingExecutor {
        calls: AtomicUsize,
        status: ExecutionStatus,
    }

    impl CountingExecutor {
        fn new(status: ExecutionStatus) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                status,
            }
        }
    }

    impl Executor for CountingExecutor {
        fn execute(&mut self, _action: &ActionProposal) -> ExecutionResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ExecutionResult {
                action_id: _action.action_id.clone(),
                task_id: _action.task_id.clone(),
                run_id: _action.run_id.clone(),
                status: self.status,
                started_at: Timestamp::UNIX_EPOCH,
                ended_at: Timestamp::from_epoch(1, 0).unwrap(),
                grounding: None,
                observation_ids: vec![],
                error: None,
            }
        }
    }

    fn action(risk: RiskClass) -> ActionProposal {
        ActionProposal::builder(
            ActionId::parse("a-1").unwrap(),
            TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(Timestamp::UNIX_EPOCH),
                authentication_strength: Some(AuthenticationStrength::Mfa),
            },
            Capability::well_known(lumi_protocol::capabilities::FILES_READ),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::FILE),
                id: "docs/x".to_owned(),
                sensitivity: None,
            },
            Target::canonical("file://docs/x"),
            "read_file",
            risk,
        )
        .unwrap()
    }

    use lumi_protocol::PrincipalKind;

    fn registry() -> CapabilityRegistry {
        let grant =
            |grant_id: &'static str, cap: &'static str, rtype: ResourceType| CapabilityGrant {
                grant_id: grant_id.to_owned(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                principal_id: None,
                capability: Capability::well_known(cap),
                resource_scope: ResourceScope::all_of([rtype]),
                target_prefixes: BTreeSet::new(),
                expires_at: None,
                source: GrantSource::DeviceDefault,
            };
        CapabilityRegistry::default()
            .grant(grant(
                "g-files-read",
                lumi_protocol::capabilities::FILES_READ,
                ResourceType::well_known(ResourceType::FILE),
            ))
            .grant(grant(
                "g-email-send",
                lumi_protocol::capabilities::EMAIL_SEND,
                ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            ))
    }

    fn policy_ctx<'a>(
        registry: &'a CapabilityRegistry,
    ) -> impl Fn(&ActionProposal) -> Decision + 'a {
        move |action| {
            lumi_policy::evaluate(
                action,
                &[],
                &PolicyContext {
                    now: Timestamp::UNIX_EPOCH,
                    device_state: DeviceExecutionState::Trusted,
                    registry: Some(registry),
                },
            )
        }
    }

    #[test]
    fn allowed_action_reaches_executor() {
        let registry = registry();
        let mut executor = CountingExecutor::new(ExecutionStatus::Success);
        let mut ledger = ApprovalLedger::new();
        let outcome = dispatch(
            policy_ctx(&registry),
            &mut ledger,
            &mut executor,
            &action(RiskClass::Read),
            None,
            Timestamp::UNIX_EPOCH,
        );
        assert!(outcome.executed());
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn approval_required_action_never_reaches_executor_without_approval() {
        let registry = registry();
        let mut executor = CountingExecutor::new(ExecutionStatus::Success);
        let mut ledger = ApprovalLedger::new();
        let email = email_action();
        let outcome = dispatch(
            policy_ctx(&registry),
            &mut ledger,
            &mut executor,
            &email,
            None,
            Timestamp::UNIX_EPOCH,
        );
        match &outcome {
            DispatchOutcome::ApprovalRequired {
                action_digest,
                reason,
                ..
            } => {
                assert!(!action_digest.is_empty());
                assert!(reason.contains("approval"));
            }
            other => panic!("expected ApprovalRequired, got {other:?}"),
        }
        assert_eq!(executor.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn valid_approval_unblocks_execution_and_consumes_single_use() {
        let registry = registry();
        let mut executor = CountingExecutor::new(ExecutionStatus::Success);
        let mut ledger = ApprovalLedger::new();
        let email = email_action();
        let human = Principal {
            principal_id: PrincipalId::parse("u-human").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            kind: PrincipalKind::User,
            authenticated_at: Some(Timestamp::UNIX_EPOCH),
            authentication_strength: Some(AuthenticationStrength::Mfa),
        };
        let approval = ledger
            .issue(
                &email,
                &human,
                ApprovalTtl::ONE_HOUR,
                Timestamp::UNIX_EPOCH,
                true,
            )
            .unwrap();
        let outcome = dispatch(
            policy_ctx(&registry),
            &mut ledger,
            &mut executor,
            &email,
            Some(&approval.approval_id),
            Timestamp::UNIX_EPOCH,
        );
        assert!(outcome.executed());
        // Single-use: second dispatch with the same approval fails closed.
        let second = dispatch(
            policy_ctx(&registry),
            &mut ledger,
            &mut executor,
            &email,
            Some(&approval.approval_id),
            Timestamp::from_epoch(5, 0).unwrap(),
        );
        match second {
            DispatchOutcome::ApprovalRequired { reason, .. } => {
                assert!(reason.contains("consumed"));
            }
            other => panic!("expected fail-closed ApprovalRequired, got {other:?}"),
        }
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
    }

    fn email_action() -> ActionProposal {
        ActionProposal::builder(
            ActionId::parse("a-email").unwrap(),
            TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(Timestamp::UNIX_EPOCH),
                authentication_strength: Some(AuthenticationStrength::Mfa),
            },
            Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "outbound/x".to_owned(),
                sensitivity: None,
            },
            Target::canonical("mailto:c@example.test"),
            "send_customer_email",
            RiskClass::Communication,
        )
        .unwrap()
    }

    #[test]
    fn denied_action_never_reaches_executor() {
        // Unregistered device: hard-safety deny.
        let registry = registry();
        let mut executor = CountingExecutor::new(ExecutionStatus::Success);
        let mut ledger = ApprovalLedger::new();
        let outcome = dispatch(
            move |action: &ActionProposal| {
                lumi_policy::evaluate(
                    action,
                    &[],
                    &PolicyContext {
                        now: Timestamp::UNIX_EPOCH,
                        device_state: DeviceExecutionState::Revoked,
                        registry: Some(&registry),
                    },
                )
            },
            &mut ledger,
            &mut executor,
            &action(RiskClass::Read),
            None,
            Timestamp::UNIX_EPOCH,
        );
        match outcome {
            DispatchOutcome::Denied { reason, .. } => assert!(reason.contains("revoked")),
            other => panic!("expected Denied, got {other:?}"),
        }
        assert_eq!(executor.calls.load(Ordering::SeqCst), 0);
    }
}
