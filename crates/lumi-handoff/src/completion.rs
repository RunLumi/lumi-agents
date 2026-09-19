//! Completion summary: what the user sees when a task ends (§23.8).

use serde::{Deserialize, Serialize};

/// An artifact produced by the task. Draft vs published visually
/// distinct (§23.13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSummary {
    pub name: String,
    pub artifact_type: String,
    /// "draft" or "published" — the UI renders these differently.
    pub publication_state: String,
    pub path_or_url: String,
}

/// A verified action taken during the task (§23.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionTaken {
    pub operation: String,
    pub trust_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Approval used during the task (§23.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalUsed {
    pub business_effect: String,
    pub approved_by: String,
}

/// The final result shown to the user (§23.8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionSummary {
    /// Concise outcome statement.
    pub outcome: String,
    /// Whether the task completed successfully (verified, not attempted).
    pub succeeded: bool,
    /// Artifacts produced (links/paths).
    pub artifacts: Vec<ArtifactSummary>,
    /// Unresolved items / exceptions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<String>,
    /// Evidence/provenance refs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    /// Actions taken (with trust labels — §23.9: honest about what was
    /// verified vs attempted).
    pub actions_taken: Vec<ActionTaken>,
    /// Approvals used during the task.
    pub approvals_used: Vec<ApprovalUsed>,
    /// Total wall-clock time in minutes.
    pub duration_minutes: u32,
    /// Total variable cost in micro-USD.
    pub total_cost_micro_usd: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_summary_shows_honest_trust_labels() {
        let summary = CompletionSummary {
            outcome: "Reconciled 42 invoices; 1 mismatch flagged for review".to_owned(),
            succeeded: true,
            artifacts: vec![ArtifactSummary {
                name: "reconciliation-report".to_owned(),
                artifact_type: "csv".to_owned(),
                publication_state: "draft".to_owned(),
                path_or_url: "workspaces/w1/report.csv".to_owned(),
            }],
            unresolved: vec!["INV-2091: amount mismatch needs manual review".to_owned()],
            evidence_refs: vec!["ev-1".to_owned(), "ev-2".to_owned()],
            actions_taken: vec![
                ActionTaken {
                    operation: "fetch_open_invoices".to_owned(),
                    trust_label: "Verified".to_owned(),
                    target: Some("erp://acme.test/invoices".to_owned()),
                },
                ActionTaken {
                    operation: "submit_adjustment".to_owned(),
                    trust_label: "Attempted".to_owned(), // NOT "Verified"
                    target: None,
                },
            ],
            approvals_used: vec![ApprovalUsed {
                business_effect: "Send reconciliation email to finance@acme.test".to_owned(),
                approved_by: "u-finance".to_owned(),
            }],
            duration_minutes: 3,
            total_cost_micro_usd: 36_000,
        };
        // §23.9: honest trust labels.
        assert!(summary
            .actions_taken
            .iter()
            .any(|a| a.trust_label == "Attempted"));
        assert!(summary
            .actions_taken
            .iter()
            .any(|a| a.trust_label == "Verified"));
        // §23.13: draft artifact is not published.
        assert_eq!(summary.artifacts[0].publication_state, "draft");
    }
}
