//! Task view and creation request (spec 23 §23.2–§23.3, §23.6).

use serde::{Deserialize, Serialize};

/// What the user expresses when creating a task (§23.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskCreationRequest {
    /// Business goal in natural language.
    pub goal: String,
    /// What the user wants produced.
    pub desired_output: String,
    /// Constraints (e.g. "only eu region", "no external emails").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<chrono_stub::DeadlineStub>,
    /// Allowed systems/resources (empty = policy decides).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_systems: Vec<String>,
    /// Approval preference where policy permits narrowing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_preference: Option<String>,
}

/// Stub for cross-platform deadline expression (v1: minutes from now).
pub mod chrono_stub {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct DeadlineStub {
        pub minutes_from_now: u32,
    }
}

/// Current phase of the task lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskPhase {
    Queued,
    Planning,
    Executing,
    WaitingApproval,
    WaitingUser,
    WaitingExternal,
    Paused,
    Verifying,
    Completed,
    Failed,
    Ambiguous,
    Cancelled,
}

/// A milestone completed during the task (§23.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Milestone {
    pub label: String,
    pub trust: crate::trust::TrustLanguage,
}

/// The task progress view shown in the UX (§23.3). Everything here is
/// read-only: the UI does not own policy or state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressView {
    pub task_id: String,
    pub phase: TaskPhase,
    pub goal: String,
    pub milestones: Vec<Milestone>,
    /// Active system/app, e.g. `Chrome — CRM portal`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_system: Option<String>,
    /// Pending approvals (§23.4: shown with business effect).
    pub pending_approvals: Vec<crate::approval::ApprovalCard>,
    /// Active exceptions (§23.5).
    pub exceptions: Vec<crate::exception::ExceptionCard>,
    /// Artifacts produced so far (§23.13: draft vs published distinct).
    pub artifacts: Vec<crate::completion::ArtifactSummary>,
    /// Elapsed wall-clock.
    pub elapsed_minutes: u32,
    /// Budget used / total.
    pub budget_used: u32,
    pub budget_total: u32,
    /// Evidence summary (count by type, not raw payloads).
    pub evidence_summary: String,
}

/// Operations the user can perform on a task (§23.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffOperation {
    Pause,
    TakeOver,
    EditGoal,
    Approve,
    Reject,
    SupplyInfo,
    Resume,
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_request_round_trips() {
        let req = TaskCreationRequest {
            goal: "Reconcile Q3 invoices".to_owned(),
            desired_output: "Reconciliation report CSV".to_owned(),
            constraints: vec!["only eu region".to_owned()],
            deadline: Some(chrono_stub::DeadlineStub {
                minutes_from_now: 120,
            }),
            allowed_systems: vec!["erp-connector".to_owned()],
            approval_preference: Some("always_for_sends".to_owned()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: TaskCreationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
    }
}
