//! Task lifecycle vocabulary (spec 01 §1.6, spec 02 §2.2).

use crate::budget::{Budget, ConsumedBudget};
use crate::ids::{TaskId, TenantId};
use crate::principal::Principal;
use crate::project::ProjectTaskBinding;
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// Product mode of a task (ADR 0006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskMode {
    /// General-purpose knowledge work.
    Work,
    /// Hardened repeatable workflow-pack execution.
    Workflow,
}

/// Task-visible lifecycle states (spec 02 §2.2). Run states (finer) roll up
/// to these; transition validation lives with the durable state store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Created,
    Queued,
    Running,
    WaitingApproval,
    WaitingUser,
    WaitingExternal,
    Paused,
    Completed,
    Failed,
    Ambiguous,
    Cancelled,
}

impl TaskStatus {
    /// True for states from which no further work is possible.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// True while the task is waiting for something outside the runtime.
    #[must_use]
    pub const fn is_waiting(self) -> bool {
        matches!(
            self,
            Self::WaitingApproval | Self::WaitingUser | Self::WaitingExternal
        )
    }
}

/// Privacy constraints bound onto a task by its principal or tenant policy.
/// Content (web pages, emails, documents) cannot create or relax these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PrivacyConstraint {
    /// Force local-only model execution for this task.
    #[serde(default)]
    pub local_only: bool,
    /// Restrict to these providers even if tenant allowlist is wider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_allowlist: Option<Vec<String>>,
    /// Providers explicitly denied for this task.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_denylist: Vec<String>,
    /// Disable selective screenshots for this task.
    #[serde(default)]
    pub no_screenshots: bool,
    /// Data may not leave the device (e.g. evidence stays local).
    #[serde(default)]
    pub no_egress: bool,
}

/// A user/business goal (spec 01 §1.6).
///
/// Tasks never contain raw provider credentials; secrets are referenced via
/// the runtime's secret broker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub task_id: TaskId,
    pub tenant_id: TenantId,
    pub principal: Principal,
    pub mode: TaskMode,
    /// The goal in natural language (work mode) or the workflow reference
    /// plus inputs (workflow mode).
    pub goal: String,
    pub created_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<Timestamp>,
    pub budget: Budget,
    pub privacy_constraints: PrivacyConstraint,
    pub status: TaskStatus,
    /// Requested output descriptions (artifact kinds/paths).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_outputs: Vec<String>,
    /// Durable project/workspace binding when the task works on local
    /// project resources (spec 26 §26.6–26.7). `None` for tasks without
    /// project scope. Resume restores the same binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_binding: Option<ProjectTaskBinding>,
}

/// Runtime bookkeeping attached to a task's current/last run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskBudgetState {
    pub consumed: ConsumedBudget,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::principal::PrincipalKind;

    #[test]
    fn status_terminality() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(!TaskStatus::Ambiguous.is_terminal());
        assert!(TaskStatus::WaitingApproval.is_waiting());
        assert!(!TaskStatus::Running.is_waiting());
    }

    #[test]
    fn task_serde_round_trip() {
        let json = serde_json::json!({
            "task_id": "task-1",
            "tenant_id": "t-1",
            "principal": {
                "principal_id": "u-1",
                "tenant_id": "t-1",
                "kind": "USER"
            },
            "mode": "WORK",
            "goal": "Reconcile Q3 invoices",
            "created_at": "2026-09-18T10:00:00Z",
            "budget": {},
            "privacy_constraints": {"local_only": true},
            "status": "CREATED"
        });
        let task: Task = serde_json::from_value(json).unwrap();
        assert_eq!(task.status, TaskStatus::Created);
        assert_eq!(task.mode, TaskMode::Work);
        assert!(task.privacy_constraints.local_only);
        assert_eq!(task.principal.kind, PrincipalKind::User);
    }

    #[test]
    fn unknown_status_fails_explicitly() {
        let err = serde_json::from_str::<TaskStatus>("\"TRANSCENDENT\"").unwrap_err();
        assert!(err.to_string().contains("unknown variant"));
    }
}
