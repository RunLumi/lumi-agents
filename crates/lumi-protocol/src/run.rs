//! Run lifecycle vocabulary (spec 01 §1.7, spec 02 §2.3).

use crate::budget::ConsumedBudget;
use crate::error::ErrorEnvelope;
use crate::ids::{RunId, TaskId};
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// Finer-grained execution states; a run's state always rolls up to a
/// task-visible [`crate::task::TaskStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunState {
    Initializing,
    Planning,
    Executing,
    Verifying,
    Checkpointing,
    Recovering,
}

/// One execution attempt of a task. A task MAY have multiple runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub run_id: RunId,
    pub task_id: TaskId,
    /// Lumi runtime version executing this run.
    pub runtime_version: String,
    /// Workflow version for workflow-mode runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version: Option<crate::ids::WorkflowVersion>,
    /// Providers selected for this run, in routing order. Recording the
    /// actual selection (not just preference) is required for provider
    /// neutrality evidence.
    pub selected_providers: Vec<String>,
    pub started_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Timestamp>,
    pub state: RunState,
    pub budgets_consumed: ConsumedBudget,
    /// Terminal failure envelope; present iff the run failed terminally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<ErrorEnvelope>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FailureCategory;

    #[test]
    fn run_serde_round_trip() {
        let run = Run {
            run_id: RunId::parse("run-1").unwrap(),
            task_id: TaskId::parse("task-1").unwrap(),
            runtime_version: "0.1.0".to_owned(),
            workflow_version: None,
            selected_providers: vec!["anthropic".to_owned()],
            started_at: Timestamp::UNIX_EPOCH,
            ended_at: None,
            state: RunState::Executing,
            budgets_consumed: ConsumedBudget::default(),
            failure: None,
        };
        let json = serde_json::to_string(&run).unwrap();
        let back: Run = serde_json::from_str(&json).unwrap();
        assert_eq!(back, run);
        assert!(back.failure.is_none());
    }

    #[test]
    fn failure_envelope_round_trips() {
        let run = Run {
            run_id: RunId::parse("run-1").unwrap(),
            task_id: TaskId::parse("task-1").unwrap(),
            runtime_version: "0.1.0".to_owned(),
            workflow_version: None,
            selected_providers: vec![],
            started_at: Timestamp::UNIX_EPOCH,
            ended_at: Some(Timestamp::from_epoch(5, 0).unwrap()),
            state: RunState::Recovering,
            budgets_consumed: ConsumedBudget::default(),
            failure: Some(ErrorEnvelope::new(
                FailureCategory::ProviderRateLimit,
                "429 from provider",
            )),
        };
        let json = serde_json::to_string(&run).unwrap();
        assert!(json.contains("PROVIDER_RATE_LIMIT"));
        let back: Run = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.failure.unwrap().category,
            FailureCategory::ProviderRateLimit
        );
    }
}
