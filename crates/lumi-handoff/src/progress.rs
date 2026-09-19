//! Progress view: live status during task execution (§23.3).

use crate::approval::ApprovalCard;
use crate::completion::ArtifactSummary;
use crate::exception::ExceptionCard;
use crate::task_view::TaskPhase;
use crate::trust::TrustLanguage;
use serde::{Deserialize, Serialize};

/// A completed or in-progress step in the task view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressStep {
    pub label: String,
    /// Trust language: planned/attempted/executed/verified/ambiguous/
    /// failed/cancelled. NEVER collapse attempted into done.
    pub trust: TrustLanguage,
    /// The system/tier used (e.g. "CRM connector", "Chrome").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// The live progress view (§23.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressView {
    pub task_id: String,
    pub phase: TaskPhase,
    pub goal: String,
    /// Steps with their trust states.
    pub steps: Vec<ProgressStep>,
    /// Currently active system/app (§23.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_system: Option<String>,
    /// Pending approvals (§23.4).
    pub pending_approvals: Vec<ApprovalCard>,
    /// Active exceptions (§23.5).
    pub exceptions: Vec<ExceptionCard>,
    /// Artifacts produced so far (§23.13: draft vs published distinct).
    pub artifacts: Vec<ArtifactSummary>,
    /// Elapsed minutes.
    pub elapsed_minutes: u32,
    /// Budget used / total (actions).
    pub budget_used: u32,
    pub budget_total: u32,
    /// Evidence summary (e.g. "3 screenshots, 2 records, 1 checksum").
    pub evidence_summary: String,
}

impl ProgressView {
    /// §23.9: the view must not show "done" for anything except verified
    /// steps. This helper computes the honest display state.
    #[must_use]
    pub fn verified_step_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.trust == TrustLanguage::Verified)
            .count()
    }

    #[must_use]
    pub fn attempted_step_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.trust == TrustLanguage::Attempted)
            .count()
    }
}

impl ProgressView {
    #[cfg(test)]
    fn milestones_or_steps_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_view_holds_all_23_3_fields() {
        let view = ProgressView {
            task_id: "task-1".to_owned(),
            phase: TaskPhase::Executing,
            goal: "Reconcile Q3 invoices".to_owned(),
            steps: vec![
                ProgressStep {
                    label: "Fetch invoices".to_owned(),
                    trust: TrustLanguage::Verified,
                    system: Some("ERP connector".to_owned()),
                },
                ProgressStep {
                    label: "Submit adjustment".to_owned(),
                    trust: TrustLanguage::Attempted,
                    system: Some("ERP connector".to_owned()),
                },
            ],
            active_system: Some("Chrome — CRM portal".to_owned()),
            pending_approvals: vec![],
            exceptions: vec![],
            artifacts: vec![],
            elapsed_minutes: 3,
            budget_used: 4,
            budget_total: 20,
            evidence_summary: "2 records, 1 screenshot".to_owned(),
        };
        // §23.3 fields all present.
        assert_eq!(view.phase, TaskPhase::Executing);
        assert!(!view.milestones_or_steps_empty());
        assert!(view.active_system.is_some());
        // §23.9: attempted ≠ verified.
        assert_eq!(view.verified_step_count(), 1);
        assert_eq!(view.attempted_step_count(), 1);
    }

    #[test]
    fn phases_cover_lifecycle() {
        // The phase enum covers the full task lifecycle for the UX.
        for phase in [
            TaskPhase::Queued,
            TaskPhase::Planning,
            TaskPhase::Executing,
            TaskPhase::WaitingApproval,
            TaskPhase::WaitingUser,
            TaskPhase::WaitingExternal,
            TaskPhase::Paused,
            TaskPhase::Verifying,
            TaskPhase::Completed,
            TaskPhase::Failed,
            TaskPhase::Ambiguous,
            TaskPhase::Cancelled,
        ] {
            // Each phase serializes deterministically.
            let json = serde_json::to_string(&phase).unwrap();
            assert!(!json.is_empty());
        }
    }
}
