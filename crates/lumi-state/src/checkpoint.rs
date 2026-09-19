//! Durable checkpoints (spec 02 §2.5–2.6).
//!
//! Checkpoints MUST NOT contain plaintext secrets: they carry only
//! references (secret refs, provider context refs). Before consequential
//! execution the runtime persists a [`PreActionCheckpoint`] so a crash at
//! any instant leaves enough state to determine what may have happened.

use lumi_protocol::{
    ActionId, ActionProposal, ApprovalId, ConsumedBudget, RunId, StepId, Timestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The plan for verifying a pre-persisted action after recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifierPlan {
    /// Postconditions to evaluate, serialized as protocol JSON.
    pub postconditions: Vec<lumi_protocol::Postcondition>,
}

/// A durable snapshot of run progress (spec 02 §2.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub run_id: RunId,
    /// Position in the plan/workflow (step index or workflow step id).
    pub step_position: u32,
    pub completed_step_ids: BTreeSet<StepId>,
    /// Actions proposed but not yet completed, serialized as protocol JSON
    /// array of `lumi.action` messages.
    pub pending_actions: Vec<String>,
    /// The last side effect whose postconditions were verified.
    pub last_verified_side_effect: Option<ActionId>,
    pub retry_counters: std::collections::BTreeMap<String, u32>,
    pub consumed_budgets: ConsumedBudget,
    /// References into model/provider context (never raw context, never
    /// secrets).
    pub model_context_refs: Vec<String>,
    /// References into workflow state.
    pub workflow_state_refs: Vec<String>,
    pub timestamp: Timestamp,
}

/// Persisted immediately before a consequential side effect (spec 02 §2.6).
///
/// If the process dies between this write and the result write, recovery
/// consults the side-effect journal + external postconditions (never
/// blindly retries).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreActionCheckpoint {
    /// The serialized `lumi.action` proposal about to execute.
    pub action: String,
    pub idempotency_key: Option<String>,
    pub approval_id: Option<ApprovalId>,
    pub verifier_plan: VerifierPlan,
    pub timestamp: Timestamp,
}

impl PreActionCheckpoint {
    /// Builds a pre-action checkpoint from a validated proposal.
    ///
    /// # Errors
    /// Returns the protocol error when the action cannot serialize.
    pub fn from_action(
        action: &ActionProposal,
        approval_id: Option<ApprovalId>,
    ) -> Result<Self, lumi_protocol::ProtocolError> {
        Ok(Self {
            action: action.to_json()?,
            idempotency_key: action.idempotency.key.clone(),
            approval_id,
            verifier_plan: VerifierPlan {
                postconditions: action.postconditions.clone(),
            },
            timestamp: Timestamp::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::{
        AuthenticationStrength, Capability, Principal, PrincipalId, PrincipalKind, ResourceRef,
        ResourceType, RiskClass, Target, TaskId, TenantId,
    };

    fn action() -> ActionProposal {
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
        .arguments(serde_json::json!({"subject": "hi"}))
        .postconditions(vec![lumi_protocol::Postcondition {
            id: lumi_protocol::PostconditionId::new("sent"),
            description: "message exists".to_owned(),
            check: lumi_protocol::PostconditionCheck::RecordExists {
                resource: ResourceRef {
                    resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                    id: "outbound/x".to_owned(),
                    sensitivity: None,
                },
            },
        }])
        .unwrap()
    }

    #[test]
    fn pre_action_checkpoint_round_trips_without_secrets() {
        let cp = PreActionCheckpoint::from_action(&action(), None).unwrap();
        let json = serde_json::to_string(&cp).unwrap();
        assert!(!json.contains("password"), "{json}");
        let back: PreActionCheckpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cp);
        // The verifier plan survived serialization.
        assert_eq!(back.verifier_plan.postconditions.len(), 1);
        // And the action itself re-parses.
        let reparsed = ActionProposal::from_json(&back.action).unwrap();
        assert_eq!(reparsed.material_digest(), action().material_digest());
    }

    #[test]
    fn checkpoint_round_trips() {
        let mut cp = Checkpoint {
            run_id: RunId::parse("run-1").unwrap(),
            step_position: 3,
            completed_step_ids: BTreeSet::from([
                StepId::parse("s1").unwrap(),
                StepId::parse("s2").unwrap(),
            ]),
            pending_actions: vec![],
            last_verified_side_effect: Some(ActionId::parse("a-0").unwrap()),
            retry_counters: std::collections::BTreeMap::from([("step-3".to_owned(), 2)]),
            consumed_budgets: ConsumedBudget::default(),
            model_context_refs: vec!["ctx-1".to_owned()],
            workflow_state_refs: vec![],
            timestamp: Timestamp::UNIX_EPOCH,
        };
        cp.consumed_budgets.record_action(false, true);
        let json = serde_json::to_string(&cp).unwrap();
        let back: Checkpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cp);
        assert_eq!(back.consumed_budgets.external_writes, 1);
    }
}
