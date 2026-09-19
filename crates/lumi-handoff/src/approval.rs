//! Approval card: business effect, not raw gesture (§23.4).

use serde::{Deserialize, Serialize};

/// What the approval UI shows. The business effect is stated in plain
/// language; the raw gesture (selector, coordinates) is NEVER the
/// headline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalCard {
    /// Human-readable summary of the business effect.
    /// Good: "Send quote to customer@example.com for 12,500,000 VND."
    /// Bad:  "Click button at (812, 477)."
    pub business_effect: String,
    /// What system/resource is affected.
    pub target_description: String,
    /// Reversibility statement.
    pub reversible: bool,
    /// Material value if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_value: Option<String>,
    /// Why approval is required (policy reason).
    pub policy_reason: String,
    /// When the approval expires.
    pub expires_at: String,
    /// Evidence summary (what the agent saw/verified so far).
    pub evidence_summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_card_shows_business_effect_not_gesture() {
        let card = ApprovalCard {
            business_effect: "Send quote to customer@example.com for 12,500,000 VND".to_owned(),
            target_description: "Email via CRM connector".to_owned(),
            reversible: false,
            material_value: Some("12,500,000 VND".to_owned()),
            policy_reason: "COMMUNICATION requires approval".to_owned(),
            expires_at: "in 60 minutes".to_owned(),
            evidence_summary: "Quote Q-2091 loaded, draft verified".to_owned(),
        };
        // §23.4: business effect, not raw gesture.
        assert!(card.business_effect.contains("customer@example.com"));
        assert!(card.business_effect.contains("12,500,000 VND"));
        assert!(!card.business_effect.contains("click"));
        assert!(!card.business_effect.contains("("));
        assert!(!card.reversible, "send email is irreversible");
    }
}
