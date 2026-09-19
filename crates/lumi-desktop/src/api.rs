//! Desktop backend API: typed commands the Tauri frontend calls.
//!
//! Every command delegates to the orchestrator/policy/audit/state crates.
//! No parallel authority, no UI-owned policy, no secret handling.

use lumi_handoff::{ApprovalCard, TrustLanguage};
use lumi_protocol::ActionId;
use serde::{Deserialize, Serialize};

/// Kill-switch state: one glance tells the operator whether work is
/// running or stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillSwitchState {
    Running,
    Stopped,
}

/// The desktop backend: exposes typed commands for the UI layer.
/// Constructed from references to the orchestrator's state; never
/// creates authority.
pub struct DesktopBackend;

/// Request to approve a pending action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveRequest {
    pub action_digest: String,
    pub approval_id: String,
}

/// Response after an approve/reject operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub accepted: bool,
    pub reason: String,
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
}
