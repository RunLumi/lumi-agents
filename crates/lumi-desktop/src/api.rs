//! Desktop backend API: typed commands the Tauri frontend calls.
//!
//! Every command delegates to the orchestrator/policy/audit/state crates.
//! No parallel authority, no UI-owned policy, no secret handling.

use lumi_handoff::{ApprovalCard, ExceptionCard, ProgressView, TrustLanguage};
use lumi_policy::{ApprovalInvalidReason, ApprovalLedger, ApprovalTtl};
use lumi_protocol::{ActionId, ActionProposal, Principal, Timestamp};
use serde::{Deserialize, Serialize};

/// Kill-switch state: one glance tells the operator whether work is
/// running or stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillSwitchState {
    Running,
    Stopped,
}

/// Whether the local runtime has supplied a view of its state.
///
/// `Unknown` is intentional: a desktop shell must not turn an absent
/// runtime connection into a fake empty queue or a fake successful run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Unknown,
}

/// Whether the local runtime is currently able to execute work. This is
/// separate from `ConnectionState`: a readable state store does not imply
/// that grants, executors, or a live runtime capability are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Enabled,
    /// A Work-mode task worker is executing (it holds the runtime lock;
    /// the shell reports this honestly instead of blocking reads).
    Executing,
    Stopped,
    Unavailable,
}

/// A normalized action waiting for a trusted human approval.
///
/// The action digest is displayed for correlation only. The UI cannot mint
/// authority from it: issuance below requires the complete action, a trusted
/// authenticated human principal, and a digest re-check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingApproval {
    pub action_digest: String,
    pub card: ApprovalCard,
    /// False until the host has attached a trusted human and an issuer that
    /// can reach the runtime's real ApprovalLedger.
    pub can_issue: bool,
}

/// Runtime economics shown by the operations console.
///
/// Each metric is optional because a desktop client must distinguish
/// "measured zero" from "the runtime has not supplied this measurement".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EconomicsSummary {
    pub verified_work_units: Option<u64>,
    pub model_cost_micro_usd: Option<u64>,
    pub human_intervention_minutes: Option<u64>,
    pub released_human_minutes: Option<u64>,
}

/// One read-only snapshot for the employee operations console.
///
/// The desktop shell owns presentation only. A runtime integration replaces
/// this snapshot with data derived from durable task, audit, evidence, and
/// economics state; the default snapshot is deliberately unknown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationsSnapshot {
    pub connection: ConnectionState,
    pub execution: ExecutionState,
    pub queue: Option<Vec<ProgressView>>,
    pub progress: Option<ProgressView>,
    pub pending_approvals: Option<Vec<PendingApproval>>,
    pub exceptions: Option<Vec<ExceptionCard>>,
    pub evidence: Option<Vec<EvidenceSummaryEntry>>,
    pub economics: Option<EconomicsSummary>,
    pub permissions: Option<crate::permissions::PermissionCheckResult>,
}

impl Default for OperationsSnapshot {
    fn default() -> Self {
        Self {
            connection: ConnectionState::Unknown,
            execution: ExecutionState::Unavailable,
            queue: None,
            progress: None,
            pending_approvals: None,
            exceptions: None,
            evidence: None,
            economics: None,
            permissions: None,
        }
    }
}

/// The desktop backend: exposes a read-only snapshot for the UI layer.
/// Runtime code supplies snapshots from the orchestrator's durable state; the
/// backend never creates policy, approval, or executor authority.
#[derive(Debug, Clone, Default)]
pub struct DesktopBackend {
    snapshot: OperationsSnapshot,
}

impl DesktopBackend {
    /// Creates a backend whose state is explicitly unknown until the local
    /// runtime supplies a snapshot.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a backend from a runtime-produced snapshot.
    #[must_use]
    pub fn from_snapshot(snapshot: OperationsSnapshot) -> Self {
        Self { snapshot }
    }

    /// Returns the current read-only view for the UI.
    #[must_use]
    pub fn snapshot(&self) -> OperationsSnapshot {
        self.snapshot.clone()
    }

    /// Replaces the view with a runtime-produced snapshot. This method does
    /// not create or mutate policy, approvals, executor state, or evidence.
    pub fn replace_snapshot(&mut self, snapshot: OperationsSnapshot) {
        self.snapshot = snapshot;
    }
}

/// Request to issue a scoped approval for a pending action.
///
/// This request carries only the digest for correlation. The runtime must
/// resolve the complete normalized action and trusted human principal before
/// calling [`DesktopBackend::issue_approval`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueApprovalRequest {
    pub action_digest: String,
}

/// Backwards-compatible name for callers migrating from the scaffold API.
/// It still represents issuance; it never consumes an approval.
pub type ApproveRequest = IssueApprovalRequest;

/// Response after an approve/reject operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub accepted: bool,
    pub reason: String,
}

/// An approval issued by the policy-owned ledger. Issuance is separate from
/// execution: the orchestrator must consume this approval only when it later
/// executes the same normalized action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssuedApproval {
    pub approval_id: String,
    pub action_digest: String,
    pub expires_at: String,
}

/// Summary of evidence for one action (§23.3 evidence summary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSummaryEntry {
    pub action_id: String,
    pub operation: String,
    pub trust_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

impl DesktopBackend {
    /// Returns the current kill-switch state.
    #[must_use]
    pub fn kill_switch_state(cancelled: bool) -> KillSwitchState {
        if cancelled {
            KillSwitchState::Stopped
        } else {
            KillSwitchState::Running
        }
    }

    /// Builds an approval card from the orchestrator's pending state.
    /// The business effect is stated in plain language (§23.4).
    #[must_use]
    pub fn build_approval_card(
        action_digest: &str,
        business_effect: &str,
        target_description: &str,
        reversible: bool,
        policy_reason: &str,
    ) -> ApprovalCard {
        ApprovalCard {
            business_effect: business_effect.to_owned(),
            target_description: target_description.to_owned(),
            reversible,
            material_value: None,
            policy_reason: policy_reason.to_owned(),
            expires_at: "in 60 minutes".to_owned(),
            evidence_summary: format!("digest: {action_digest}"),
        }
    }

    /// Builds a trust-labeled evidence entry for the evidence viewer
    /// (§23.9: planned/attempted/executed/verified/ambiguous are distinct).
    #[must_use]
    pub fn build_evidence_entry(
        action_id: &ActionId,
        operation: &str,
        trust_label: &TrustLanguage,
        target: Option<&str>,
    ) -> EvidenceSummaryEntry {
        EvidenceSummaryEntry {
            action_id: action_id.as_str().to_owned(),
            operation: operation.to_owned(),
            trust_label: trust_label.label().to_owned(),
            verification: match trust_label {
                TrustLanguage::Verified => Some("postconditions passed".to_owned()),
                TrustLanguage::Ambiguous => Some("outcome unknown; verify externally".to_owned()),
                _ => None,
            },
            target: target.map(str::to_owned),
        }
    }

    /// Issues a scoped approval through the runtime's real ledger.
    ///
    /// This is intentionally not a "consume approval" helper. Consumption
    /// belongs to `Orchestrator::execute_step`, after policy and routing have
    /// revalidated the same action. The caller must supply a trusted,
    /// authenticated human principal from the host boundary; the UI must not
    /// manufacture one from request data.
    pub fn issue_approval(
        ledger: &mut ApprovalLedger,
        action: &ActionProposal,
        expected_digest: &str,
        approver: &Principal,
        now: Timestamp,
    ) -> Result<IssuedApproval, ApprovalInvalidReason> {
        if !action.material_matches_digest(expected_digest) {
            return Err(ApprovalInvalidReason::DigestMismatch);
        }
        let approval = ledger.issue(action, approver, ApprovalTtl::ONE_HOUR, now, true)?;
        Ok(IssuedApproval {
            approval_id: approval.approval_id.as_str().to_owned(),
            action_digest: approval.action_digest,
            expires_at: approval.expires_at.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_switch_state_is_binary_and_obvious() {
        assert_eq!(
            DesktopBackend::kill_switch_state(false),
            KillSwitchState::Running
        );
        assert_eq!(
            DesktopBackend::kill_switch_state(true),
            KillSwitchState::Stopped
        );
    }

    #[test]
    fn approval_card_carries_business_effect() {
        let card = DesktopBackend::build_approval_card(
            "abc123",
            "Send quote to customer@example.com for 12,500,000 VND",
            "Email via CRM connector",
            false,
            "COMMUNICATION requires approval",
        );
        assert!(card.business_effect.contains("customer@example.com"));
        assert!(card.business_effect.contains("12,500,000 VND"));
        assert!(!card.reversible);
    }

    #[test]
    fn evidence_entry_shows_honest_trust_labels() {
        let entry = DesktopBackend::build_evidence_entry(
            &ActionId::parse("a-1").unwrap(),
            "submit_form",
            &TrustLanguage::Ambiguous,
            Some("https://portal.example.test"),
        );
        assert_eq!(entry.trust_label, "Ambiguous — outcome unknown");
        assert!(entry
            .verification
            .as_deref()
            .unwrap()
            .contains("externally"));
    }

    #[test]
    fn unknown_snapshot_does_not_invent_work_or_permissions() {
        let snapshot = OperationsSnapshot::default();
        assert_eq!(snapshot.connection, ConnectionState::Unknown);
        assert_eq!(snapshot.execution, ExecutionState::Unavailable);
        assert!(snapshot.queue.is_none());
        assert!(snapshot.progress.is_none());
        assert!(snapshot.pending_approvals.is_none());
        assert!(snapshot.evidence.is_none());
        assert!(snapshot.permissions.is_none());
        let json = serde_json::to_value(snapshot).unwrap();
        assert_eq!(json["progress"], serde_json::Value::Null);
        assert_eq!(json["pending_approvals"], serde_json::Value::Null);
    }

    #[test]
    fn issue_approval_validates_digest_without_consuming_it() {
        use lumi_protocol::{
            AuthenticationStrength, Capability, PrincipalId, PrincipalKind, ResourceRef,
            ResourceType, RiskClass, RunId, Target, TaskId, TenantId,
        };

        let action = ActionProposal::builder(
            ActionId::parse("a-approval").unwrap(),
            TaskId::parse("task-approval").unwrap(),
            RunId::parse("run-approval").unwrap(),
            Principal {
                principal_id: PrincipalId::parse("workflow-1").unwrap(),
                tenant_id: TenantId::parse("tenant-1").unwrap(),
                kind: PrincipalKind::Workflow,
                authenticated_at: Some(Timestamp::UNIX_EPOCH),
                authentication_strength: Some(AuthenticationStrength::DevicePossession),
            },
            Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "outbound/approval".to_owned(),
                sensitivity: None,
            },
            Target::canonical("mailto:finance@example.test"),
            "send_customer_email",
            RiskClass::Communication,
        )
        .arguments(serde_json::json!({"subject": "Invoice"}))
        .unwrap();
        let approver = Principal {
            principal_id: PrincipalId::parse("human-1").unwrap(),
            tenant_id: TenantId::parse("tenant-1").unwrap(),
            kind: PrincipalKind::User,
            authenticated_at: Some(Timestamp::UNIX_EPOCH),
            authentication_strength: Some(AuthenticationStrength::Mfa),
        };
        let mut ledger = ApprovalLedger::new();
        assert_eq!(
            DesktopBackend::issue_approval(
                &mut ledger,
                &action,
                "wrong-digest",
                &approver,
                Timestamp::UNIX_EPOCH,
            ),
            Err(ApprovalInvalidReason::DigestMismatch)
        );
        let issued = DesktopBackend::issue_approval(
            &mut ledger,
            &action,
            &action.material_digest(),
            &approver,
            Timestamp::UNIX_EPOCH,
        )
        .unwrap();

        let approval_id = lumi_protocol::ApprovalId::parse(issued.approval_id).unwrap();
        assert!(matches!(
            ledger.validate(&approval_id, &action, Timestamp::UNIX_EPOCH),
            lumi_policy::ApprovalValidation::Valid { .. }
        ));
        assert!(ledger.consumed_at(&approval_id).is_none());
    }
}
