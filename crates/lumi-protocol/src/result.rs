//! Execution results (spec 03 §3.7).
//!
//! SUCCESS means the executor operation returned successfully — it does
//! NOT mean workflow postconditions passed. Verification is a separate,
//! explicit step (spec 11 §11.8).

use crate::error::ErrorEnvelope;
use crate::ids::{ActionId, RunId, TaskId};
use crate::schema::{schema_names, Envelope, ProtocolError};
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// Executor result status (spec 03 §3.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionStatus {
    /// The executor operation completed and reported success.
    Success,
    /// The executor operation failed with a normalized error.
    Failed,
    /// The executor cannot determine whether the operation took effect
    /// (e.g. timeout after submit). MUST route through ambiguity handling,
    /// never blind retry.
    Ambiguous,
    /// Cancelled before or during execution.
    Cancelled,
}

/// How the executor grounded its operation (spec 05 §5.10). Recorded for
/// determinism telemetry: high vision usage should trigger semantic-adapter
/// review (spec 05 §5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Grounding {
    /// Deterministic API/connector call.
    DeterministicApi,
    /// DOM/accessibility selector.
    SemanticLocator,
    /// AX/UIA semantic element target.
    SemanticTarget,
    /// App-specific deterministic adapter logic.
    AppAdapterLogic,
    /// Vision/coordinate grounding.
    VisionCoordinate,
}

impl Grounding {
    /// True for non-deterministic grounding modes.
    #[must_use]
    pub const fn is_vision_based(self) -> bool {
        matches!(self, Self::VisionCoordinate)
    }
}

/// Raw wire shape of an [`ExecutionResult`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionResultRaw {
    schema_name: String,
    schema_version: String,
    action_id: ActionId,
    task_id: TaskId,
    run_id: RunId,
    status: ExecutionStatus,
    started_at: Timestamp,
    ended_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grounding: Option<Grounding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    observation_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<ErrorEnvelope>,
}

/// The outcome of one executor attempt for one action.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutionResult {
    pub action_id: ActionId,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub status: ExecutionStatus,
    pub started_at: Timestamp,
    pub ended_at: Timestamp,
    pub grounding: Option<Grounding>,
    /// Observations produced by/for this execution (ids into the
    /// observation store).
    pub observation_ids: Vec<String>,
    /// Present iff status is FAILED (and MAY be present for AMBIGUOUS).
    pub error: Option<ErrorEnvelope>,
}

impl ExecutionResult {
    #[must_use]
    pub fn envelope() -> Envelope {
        Envelope::new(schema_names::EXECUTION_RESULT)
    }

    /// Validates the success/error invariants.
    ///
    /// # Errors
    /// [`ProtocolError::Malformed`] when FAILED lacks an error envelope,
    /// or SUCCESS carries one.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self.status {
            ExecutionStatus::Failed => {
                if self.error.is_none() {
                    return Err(ProtocolError::Malformed(
                        "FAILED result must carry an error envelope".to_owned(),
                    ));
                }
            }
            ExecutionStatus::Success => {
                if self.error.is_some() {
                    return Err(ProtocolError::Malformed(
                        "SUCCESS result must not carry an error envelope".to_owned(),
                    ));
                }
            }
            ExecutionStatus::Ambiguous | ExecutionStatus::Cancelled => {}
        }
        Ok(())
    }

    /// Serializes with the protocol envelope.
    ///
    /// # Errors
    /// Propagates [`Self::validate`].
    pub fn to_json(&self) -> Result<String, ProtocolError> {
        self.validate()?;
        let raw = ExecutionResultRaw {
            schema_name: schema_names::EXECUTION_RESULT.to_owned(),
            schema_version: crate::schema::PROTOCOL_VERSION.to_owned(),
            action_id: self.action_id.clone(),
            task_id: self.task_id.clone(),
            run_id: self.run_id.clone(),
            status: self.status,
            started_at: self.started_at,
            ended_at: self.ended_at,
            grounding: self.grounding,
            observation_ids: if self.observation_ids.is_empty() {
                None
            } else {
                Some(self.observation_ids.clone())
            },
            error: self.error.clone(),
        };
        serde_json::to_string(&raw).map_err(|e| ProtocolError::Malformed(e.to_string()))
    }

    /// Deserializes with strict envelope and invariant checks.
    ///
    /// # Errors
    /// Fails closed on unknown schema versions/enum values/invariants.
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        let raw: ExecutionResultRaw =
            serde_json::from_str(json).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
        Envelope {
            schema_name: raw.schema_name.clone(),
            schema_version: raw.schema_version.clone(),
        }
        .ensure(schema_names::EXECUTION_RESULT)?;
        let result = Self {
            action_id: raw.action_id,
            task_id: raw.task_id,
            run_id: raw.run_id,
            status: raw.status,
            started_at: raw.started_at,
            ended_at: raw.ended_at,
            grounding: raw.grounding,
            observation_ids: raw.observation_ids.unwrap_or_default(),
            error: raw.error,
        };
        result.validate()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FailureCategory;

    fn result(status: ExecutionStatus) -> ExecutionResult {
        ExecutionResult {
            action_id: ActionId::parse("a-1").unwrap(),
            task_id: TaskId::parse("task-1").unwrap(),
            run_id: RunId::parse("run-1").unwrap(),
            status,
            started_at: Timestamp::UNIX_EPOCH,
            ended_at: Timestamp::from_epoch(1, 0).unwrap(),
            grounding: Some(Grounding::DeterministicApi),
            observation_ids: vec!["obs-1".to_owned()],
            error: None,
        }
    }

    #[test]
    fn success_requires_no_error() {
        assert!(result(ExecutionStatus::Success).validate().is_ok());
        let bad = ExecutionResult {
            error: Some(ErrorEnvelope::new(FailureCategory::Network, "nope")),
            ..result(ExecutionStatus::Success)
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn failed_requires_error_envelope() {
        assert!(result(ExecutionStatus::Failed).validate().is_err());
        let ok = ExecutionResult {
            error: Some(ErrorEnvelope::new(FailureCategory::Network, "timeout")),
            ..result(ExecutionStatus::Failed)
        };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn json_round_trip_carries_envelope() {
        let r = result(ExecutionStatus::Success);
        let json = r.to_json().unwrap();
        assert!(
            json.contains("\"schema_name\":\"lumi.execution-result\""),
            "{json}"
        );
        let back = ExecutionResult::from_json(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn ambiguous_is_distinct_from_success() {
        // Spec 03 §3.7: the executor must be able to express ambiguity.
        assert!(result(ExecutionStatus::Ambiguous).validate().is_ok());
        assert_ne!(ExecutionStatus::Ambiguous, ExecutionStatus::Success);
    }
}
