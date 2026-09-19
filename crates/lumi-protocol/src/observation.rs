//! Observations: normalized read-backs of world state (spec 03 §3.5).
//!
//! Trust rule (spec 03 §3.6): observation confidence MUST NOT grant
//! authority. Model-inferred and visual observations SHOULD be verified
//! through structured state before consequential actions.

use crate::ids::{RunId, TaskId};
use crate::schema::{schema_names, Envelope, ProtocolError};
use crate::tier::ObservationSurface;
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// What kind of observation this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObservationKind {
    /// Observed state of a resource/system.
    State,
    /// Observed content (page text, file content…).
    Content,
    /// Result of an executed operation.
    Result,
    /// Error observation.
    Error,
    /// Captured evidence artifact reference/payload.
    Evidence,
}

/// How the observation was obtained. Higher-trust kinds are produced by
/// deterministic queries; lower-trust kinds are model/visual inferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConfidenceKind {
    /// Direct deterministic API/DB/file read.
    Deterministic,
    /// Structured locator read (DOM/AX/UIA query).
    Structured,
    /// Model inference over unstructured input.
    ModelInferred,
    /// Visual heuristic over pixels.
    VisualHeuristic,
}

impl ConfidenceKind {
    /// A rough trust rank for verification policy; lower is more trusted.
    /// Trust never grants authority — this only orders *verification
    /// requirements*.
    #[must_use]
    pub const fn trust_rank(self) -> u8 {
        match self {
            Self::Deterministic => 0,
            Self::Structured => 1,
            Self::ModelInferred => 2,
            Self::VisualHeuristic => 3,
        }
    }
}

/// Confidence of an observation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Confidence {
    pub kind: ConfidenceKind,
    /// Optional model/heuristic score in `[0, 1]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}

impl Confidence {
    #[must_use]
    pub const fn deterministic() -> Self {
        Self {
            kind: ConfidenceKind::Deterministic,
            score: None,
        }
    }
}

/// Where the observation came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationSource {
    pub surface: ObservationSurface,
    /// Adapter identity, e.g. `playwright/1.49`.
    pub adapter: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
}

/// Raw wire shape of an [`Observation`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObservationRaw {
    schema_name: String,
    schema_version: String,
    observation_id: String,
    task_id: TaskId,
    run_id: RunId,
    source: ObservationSource,
    timestamp: Timestamp,
    kind: ObservationKind,
    payload: serde_json::Value,
    confidence: Confidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sensitivity: Option<crate::resource::SensitivityLabel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provenance: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    step_id: Option<String>,
}

/// A normalized observation (spec 03 §3.5).
///
/// Every observation is traceable to its task/run and, when applicable,
/// the action/step that produced it (spec 03 §3.11).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Observation {
    pub observation_id: String,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub source: ObservationSource,
    pub timestamp: Timestamp,
    pub kind: ObservationKind,
    /// JSON object payload.
    pub payload: serde_json::Value,
    pub confidence: Confidence,
    pub sensitivity: Option<crate::resource::SensitivityLabel>,
    pub provenance: Option<serde_json::Value>,
    pub action_id: Option<String>,
    pub step_id: Option<String>,
}

impl Observation {
    #[must_use]
    pub fn envelope() -> Envelope {
        Envelope::new(schema_names::OBSERVATION)
    }

    /// Validates structural invariants.
    ///
    /// # Errors
    /// [`ProtocolError::Malformed`] for non-object payloads or empty ids.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if !self.payload.is_object() {
            return Err(ProtocolError::Malformed(
                "payload must be a JSON object".to_owned(),
            ));
        }
        if self.observation_id.is_empty() {
            return Err(ProtocolError::Malformed(
                "observation_id must not be empty".to_owned(),
            ));
        }
        if self.source.adapter.trim().is_empty() {
            return Err(ProtocolError::Malformed(
                "source.adapter must not be empty".to_owned(),
            ));
        }
        Ok(())
    }

    /// Serializes with the protocol envelope.
    ///
    /// # Errors
    /// Propagates [`Self::validate`].
    pub fn to_json(&self) -> Result<String, ProtocolError> {
        self.validate()?;
        let raw = ObservationRaw {
            schema_name: schema_names::OBSERVATION.to_owned(),
            schema_version: crate::schema::PROTOCOL_VERSION.to_owned(),
            observation_id: self.observation_id.clone(),
            task_id: self.task_id.clone(),
            run_id: self.run_id.clone(),
            source: self.source.clone(),
            timestamp: self.timestamp,
            kind: self.kind,
            payload: self.payload.clone(),
            confidence: self.confidence,
            sensitivity: self.sensitivity,
            provenance: self.provenance.clone(),
            action_id: self.action_id.clone(),
            step_id: self.step_id.clone(),
        };
        serde_json::to_string(&raw).map_err(|e| ProtocolError::Malformed(e.to_string()))
    }

    /// Deserializes with strict envelope and invariant checks.
    ///
    /// # Errors
    /// Fails closed on unknown schema versions/enum values/invariants.
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        let raw: ObservationRaw =
            serde_json::from_str(json).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
        Envelope {
            schema_name: raw.schema_name.clone(),
            schema_version: raw.schema_version.clone(),
        }
        .ensure(schema_names::OBSERVATION)?;
        let observation = Self {
            observation_id: raw.observation_id,
            task_id: raw.task_id,
            run_id: raw.run_id,
            source: raw.source,
            timestamp: raw.timestamp,
            kind: raw.kind,
            payload: raw.payload,
            confidence: raw.confidence,
            sensitivity: raw.sensitivity,
            provenance: raw.provenance,
            action_id: raw.action_id,
            step_id: raw.step_id,
        };
        observation.validate()?;
        Ok(observation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> Observation {
        Observation {
            observation_id: "obs-1".to_owned(),
            task_id: TaskId::parse("task-1").unwrap(),
            run_id: RunId::parse("run-1").unwrap(),
            source: ObservationSource {
                surface: ObservationSurface::Browser,
                adapter: "playwright/1.49".to_owned(),
                resource: Some("https://example.test".to_owned()),
            },
            timestamp: Timestamp::UNIX_EPOCH,
            kind: ObservationKind::State,
            payload: serde_json::json!({"selector": "#status", "value": "APPROVED"}),
            confidence: Confidence {
                kind: ConfidenceKind::Structured,
                score: None,
            },
            sensitivity: None,
            provenance: None,
            action_id: Some("a-1".to_owned()),
            step_id: None,
        }
    }

    #[test]
    fn json_round_trip_carries_envelope() {
        let obs = observation();
        let json = obs.to_json().unwrap();
        assert!(
            json.contains("\"schema_name\":\"lumi.observation\""),
            "{json}"
        );
        let back = Observation::from_json(&json).unwrap();
        assert_eq!(back, obs);
    }

    #[test]
    fn unknown_confidence_kind_fails_closed() {
        let json = observation().to_json().unwrap();
        let mutated = json.replace("STRUCTURED", "TELEPATHY");
        assert!(Observation::from_json(&mutated).is_err());
    }

    #[test]
    fn non_object_payload_is_malformed() {
        let obs = Observation {
            payload: serde_json::json!([1, 2]),
            ..observation()
        };
        assert!(obs.validate().is_err());
    }

    #[test]
    fn trust_rank_orders_verification_requirements() {
        assert!(
            ConfidenceKind::Deterministic.trust_rank()
                < ConfidenceKind::VisualHeuristic.trust_rank()
        );
    }
}
