//! Scoped human approvals bound to the normalized action (spec 04 §4.6–4.7).
//!
//! Approval is scoped to a normalized action, not a session-wide boolean.
//! The approval binds to the action's *material digest*: any material
//! change after issuance invalidates the approval (spec 03 §3.3).

use lumi_protocol::{
    ActionId, ActionProposal, ApprovalId, Principal, PrincipalKind, TenantId, Timestamp,
};
use std::collections::HashMap;

/// How long an approval stays valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalTtl {
    pub seconds: u64,
}

impl ApprovalTtl {
    pub const FIFTEEN_MINUTES: Self = Self { seconds: 900 };
    pub const ONE_HOUR: Self = Self { seconds: 3600 };
    pub const TWENTY_FOUR_HOURS: Self = Self { seconds: 86_400 };
}

/// Optional bounded constraints carried by an approval (spec 04 §4.6).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApprovalConstraints {
    /// Restrict use to a specific workflow.
    pub workflow_id: Option<lumi_protocol::WorkflowId>,
    /// Restrict use to a specific run.
    pub run_id: Option<lumi_protocol::RunId>,
}

/// A scoped human approval for one normalized action digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    pub approval_id: ApprovalId,
    pub tenant_id: TenantId,
    /// The human who approved (USER or ADMIN, authenticated).
    pub approver: Principal,
    /// SHA-256 over the material projection of the approved action.
    pub action_digest: String,
    /// The approved action id, for UX/evidence correlation. Validation is
    /// by digest — a different action id with the same material digest is
    /// the same business action.
    pub action_id: ActionId,
    pub issued_at: Timestamp,
    pub expires_at: Timestamp,
    pub constraints: ApprovalConstraints,
    /// Single-use approvals are consumed on first validation.
    pub single_use: bool,
}

/// Why an approval failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalInvalidReason {
    UnknownApproval,
    Expired,
    DigestMismatch,
    CrossTenant,
    SingleUseConsumed,
    ApproverNotHuman,
    ApproverUnauthenticated,
    WorkflowMismatch,
    RunMismatch,
}

impl ApprovalInvalidReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownApproval => "unknown approval id",
            Self::Expired => "approval expired",
            Self::DigestMismatch => "action changed after approval (material digest mismatch)",
            Self::CrossTenant => "approval belongs to another tenant",
            Self::SingleUseConsumed => "single-use approval already consumed",
            Self::ApproverNotHuman => "approver is not a human principal",
            Self::ApproverUnauthenticated => "approver was not authenticated",
            Self::WorkflowMismatch => "approval bound to a different workflow",
            Self::RunMismatch => "approval bound to a different run",
        }
    }
}

/// Result of validating an approval against a concrete action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalValidation {
    Valid { approval_id: ApprovalId },
    Invalid { reason: ApprovalInvalidReason },
}

/// Stores issued approvals and their consumption state.
///
/// The ledger is the only place approvals can be minted or consumed, so a
/// runtime cannot validate an approval it fabricated.
#[derive(Debug, Default)]
pub struct ApprovalLedger {
    approvals: HashMap<ApprovalId, Approval>,
    consumed: HashMap<ApprovalId, Timestamp>,
}

impl ApprovalLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Issues an approval binding `action`'s material digest to `approver`.
    ///
    /// # Errors
    /// Returns the reason when the approver is not an authenticated human
    /// in the same tenant as the action's principal — approvals minted by
    /// non-humans would break the invariant that models may propose but
    /// never authorize.
    pub fn issue(
        &mut self,
        action: &ActionProposal,
        approver: &Principal,
        ttl: ApprovalTtl,
        now: Timestamp,
        single_use: bool,
    ) -> Result<Approval, ApprovalInvalidReason> {
        if approver.tenant_id != action.principal.tenant_id {
            return Err(ApprovalInvalidReason::CrossTenant);
        }
        if !matches!(approver.kind, PrincipalKind::User | PrincipalKind::Admin) {
            return Err(ApprovalInvalidReason::ApproverNotHuman);
        }
        if !approver.is_authenticated() {
            return Err(ApprovalInvalidReason::ApproverUnauthenticated);
        }
        let expires_at = now
            .checked_add(std::time::Duration::from_secs(ttl.seconds))
            .ok_or(ApprovalInvalidReason::Expired)?;
        let approval = Approval {
            approval_id: ApprovalId::generate(),
            tenant_id: approver.tenant_id.clone(),
            approver: approver.clone(),
            action_digest: action.material_digest(),
            action_id: action.action_id.clone(),
            issued_at: now,
            expires_at,
            constraints: ApprovalConstraints::default(),
            single_use,
        };
        self.approvals
            .insert(approval.approval_id.clone(), approval.clone());
        Ok(approval)
    }

    /// Binds an issued approval to workflow/run constraints.
    pub fn bind(&mut self, approval_id: &ApprovalId, constraints: ApprovalConstraints) {
        if let Some(approval) = self.approvals.get_mut(approval_id) {
            approval.constraints = constraints;
        }
    }

    /// Validates an approval against `action` without consuming it.
    #[must_use]
    pub fn validate(
        &self,
        approval_id: &ApprovalId,
        action: &ActionProposal,
        now: Timestamp,
    ) -> ApprovalValidation {
        let Some(approval) = self.approvals.get(approval_id) else {
            return ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::UnknownApproval,
            };
        };
        if approval.tenant_id != action.principal.tenant_id {
            return ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::CrossTenant,
            };
        }
        if now > approval.expires_at {
            return ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::Expired,
            };
        }
        if approval.action_digest != action.material_digest() {
            return ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::DigestMismatch,
            };
        }
        if let Some(workflow) = &approval.constraints.workflow_id {
            if action.workflow_id.as_ref() != Some(workflow) {
                return ApprovalValidation::Invalid {
                    reason: ApprovalInvalidReason::WorkflowMismatch,
                };
            }
        }
        if let Some(run) = &approval.constraints.run_id {
            if &action.run_id != run {
                return ApprovalValidation::Invalid {
                    reason: ApprovalInvalidReason::RunMismatch,
                };
            }
        }
        if approval.single_use && self.consumed.contains_key(approval_id) {
            return ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::SingleUseConsumed,
            };
        }
        ApprovalValidation::Valid {
            approval_id: approval_id.clone(),
        }
    }

    /// Validates and consumes a single-use approval.
    pub fn validate_and_consume(
        &mut self,
        approval_id: &ApprovalId,
        action: &ActionProposal,
        now: Timestamp,
    ) -> ApprovalValidation {
        match self.validate(approval_id, action, now) {
            ApprovalValidation::Valid { approval_id } => {
                if let Some(approval) = self.approvals.get(&approval_id) {
                    if approval.single_use {
                        self.consumed.insert(approval_id.clone(), now);
                    }
                }
                ApprovalValidation::Valid { approval_id }
            }
            invalid => invalid,
        }
    }

    /// Whether a single-use approval was consumed (for audit evidence).
    #[must_use]
    pub fn consumed_at(&self, approval_id: &ApprovalId) -> Option<Timestamp> {
        self.consumed.get(approval_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::{
        AuthenticationStrength, Capability, PrincipalId, ResourceRef, ResourceType, RiskClass,
        RunId, Target, TaskId,
    };

    fn action(tenant: &str, target: &str) -> ActionProposal {
        ActionProposal::builder(
            ActionId::parse("a-1").unwrap(),
            TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            Principal {
                principal_id: PrincipalId::parse("u-agent").unwrap(),
                tenant_id: TenantId::parse(tenant).unwrap(),
                kind: PrincipalKind::Workflow,
                authenticated_at: Some(Timestamp::UNIX_EPOCH),
                authentication_strength: Some(AuthenticationStrength::DevicePossession),
            },
            Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "outbound/x".to_owned(),
                sensitivity: None,
            },
            Target::canonical(target),
            "send_customer_email",
            RiskClass::Communication,
        )
        .arguments(serde_json::json!({"subject": "hi"}))
        .unwrap()
    }

    fn human(tenant: &str) -> Principal {
        Principal {
            principal_id: PrincipalId::parse("u-human").unwrap(),
            tenant_id: TenantId::parse(tenant).unwrap(),
            kind: PrincipalKind::User,
            authenticated_at: Some(Timestamp::UNIX_EPOCH),
            authentication_strength: Some(AuthenticationStrength::Mfa),
        }
    }

    #[test]
    fn approval_rejects_changed_sensitivity_risk_principal_and_idempotency() {
        let mut ledger = ApprovalLedger::new();
        let original = action("t-1", "mailto:c@example.test");
        let approval = ledger
            .issue(
                &original,
                &human("t-1"),
                ApprovalTtl::ONE_HOUR,
                Timestamp::UNIX_EPOCH,
                false,
            )
            .unwrap();
        let mut sensitivity = original.clone();
        sensitivity.resource.sensitivity = Some(lumi_protocol::SensitivityLabel::Restricted);
        let mut risk = original.clone();
        risk.risk_class = RiskClass::Read;
        let mut principal = original.clone();
        principal.principal.principal_id = PrincipalId::parse("another-actor").unwrap();
        let mut retry_key = original.clone();
        retry_key.idempotency = lumi_protocol::Idempotency {
            key: Some("different-effect-key".to_owned()),
            semantics: lumi_protocol::IdempotencySemantics::ClientKey,
        };
        for changed in [sensitivity, risk, principal, retry_key] {
            assert_eq!(
                ledger.validate(&approval.approval_id, &changed, Timestamp::UNIX_EPOCH),
                ApprovalValidation::Invalid {
                    reason: ApprovalInvalidReason::DigestMismatch
                }
            );
        }
    }

    #[test]
    fn approval_ignores_argument_object_insertion_order() {
        let mut ledger = ApprovalLedger::new();
        let mut original = action("t-1", "mailto:c@example.test");
        original.arguments = serde_json::json!({"z": 1, "nested": {"z": 2, "a": 3}});
        let mut reordered = original.clone();
        reordered.arguments = serde_json::json!({"nested": {"a": 3, "z": 2}, "z": 1});
        let approval = ledger
            .issue(
                &original,
                &human("t-1"),
                ApprovalTtl::ONE_HOUR,
                Timestamp::UNIX_EPOCH,
                false,
            )
            .unwrap();
        assert!(matches!(
            ledger.validate(&approval.approval_id, &reordered, Timestamp::UNIX_EPOCH),
            ApprovalValidation::Valid { .. }
        ));
    }

    #[test]
    fn approval_binds_to_action_digest() {
        let mut ledger = ApprovalLedger::new();
        let a = action("t-1", "mailto:c@example.test");
        let approval = ledger
            .issue(
                &a,
                &human("t-1"),
                ApprovalTtl::ONE_HOUR,
                Timestamp::UNIX_EPOCH,
                false,
            )
            .unwrap();
        assert_eq!(
            ledger.validate(
                &approval.approval_id,
                &a,
                Timestamp::from_epoch(10, 0).unwrap()
            ),
            ApprovalValidation::Valid {
                approval_id: approval.approval_id.clone()
            }
        );
        // Material change invalidates.
        let mutated = ActionProposal {
            target: Target::canonical("mailto:other@example.test"),
            ..a
        };
        assert_eq!(
            ledger.validate(
                &approval.approval_id,
                &mutated,
                Timestamp::from_epoch(10, 0).unwrap()
            ),
            ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::DigestMismatch
            }
        );
    }

    #[test]
    fn expired_approval_is_rejected() {
        let mut ledger = ApprovalLedger::new();
        let a = action("t-1", "mailto:c@example.test");
        let approval = ledger
            .issue(
                &a,
                &human("t-1"),
                ApprovalTtl::FIFTEEN_MINUTES,
                Timestamp::UNIX_EPOCH,
                false,
            )
            .unwrap();
        let late =
            Timestamp::from_epoch((ApprovalTtl::FIFTEEN_MINUTES.seconds + 1) as i64, 0).unwrap();
        assert_eq!(
            ledger.validate(&approval.approval_id, &a, late),
            ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::Expired
            }
        );
    }

    #[test]
    fn cross_tenant_approval_is_rejected() {
        let mut ledger = ApprovalLedger::new();
        let a = action("t-1", "mailto:c@example.test");
        let approval = ledger.issue(
            &a,
            &human("t-2"),
            ApprovalTtl::ONE_HOUR,
            Timestamp::UNIX_EPOCH,
            false,
        );
        // Issuing across tenants must already fail.
        assert_eq!(approval.unwrap_err(), ApprovalInvalidReason::CrossTenant);
    }

    #[test]
    fn single_use_approval_is_consumed_once() {
        let mut ledger = ApprovalLedger::new();
        let a = action("t-1", "mailto:c@example.test");
        let approval = ledger
            .issue(
                &a,
                &human("t-1"),
                ApprovalTtl::ONE_HOUR,
                Timestamp::UNIX_EPOCH,
                true,
            )
            .unwrap();
        let first = ledger.validate_and_consume(
            &approval.approval_id,
            &a,
            Timestamp::from_epoch(10, 0).unwrap(),
        );
        assert!(matches!(first, ApprovalValidation::Valid { .. }));
        let second = ledger.validate_and_consume(
            &approval.approval_id,
            &a,
            Timestamp::from_epoch(11, 0).unwrap(),
        );
        assert_eq!(
            second,
            ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::SingleUseConsumed
            }
        );
        assert_eq!(
            ledger.consumed_at(&approval.approval_id),
            Some(Timestamp::from_epoch(10, 0).unwrap())
        );
    }

    #[test]
    fn non_human_or_unauthenticated_approvers_are_rejected() {
        let mut ledger = ApprovalLedger::new();
        let a = action("t-1", "mailto:c@example.test");
        let mut agent = human("t-1");
        agent.kind = PrincipalKind::Workflow;
        assert_eq!(
            ledger
                .issue(
                    &a,
                    &agent,
                    ApprovalTtl::ONE_HOUR,
                    Timestamp::UNIX_EPOCH,
                    false
                )
                .unwrap_err(),
            ApprovalInvalidReason::ApproverNotHuman
        );
        let mut unauth = human("t-1");
        unauth.authentication_strength = Some(AuthenticationStrength::Unauthenticated);
        assert_eq!(
            ledger
                .issue(
                    &a,
                    &unauth,
                    ApprovalTtl::ONE_HOUR,
                    Timestamp::UNIX_EPOCH,
                    false
                )
                .unwrap_err(),
            ApprovalInvalidReason::ApproverUnauthenticated
        );
    }

    #[test]
    fn unknown_approval_fails_closed() {
        let ledger = ApprovalLedger::new();
        let a = action("t-1", "mailto:c@example.test");
        assert_eq!(
            ledger.validate(
                &ApprovalId::parse("nope").unwrap(),
                &a,
                Timestamp::UNIX_EPOCH
            ),
            ApprovalValidation::Invalid {
                reason: ApprovalInvalidReason::UnknownApproval
            }
        );
    }
}
