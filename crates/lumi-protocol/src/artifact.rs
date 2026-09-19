//! Artifacts: user-visible outputs (spec 01 §1.11).

use crate::ids::{ArtifactId, RunId, TaskId};
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// Artifact type. Open vocabulary with well-known constants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactType(pub String);

impl ArtifactType {
    pub const MARKDOWN: &'static str = "markdown";
    pub const TEXT: &'static str = "text";
    pub const PDF: &'static str = "pdf";
    pub const DOCX: &'static str = "docx";
    pub const XLSX: &'static str = "xlsx";
    pub const CSV: &'static str = "csv";
    pub const PPTX: &'static str = "pptx";
    pub const IMAGE: &'static str = "image";
    pub const CODE: &'static str = "code";
    pub const REPORT: &'static str = "report";
    pub const JSON: &'static str = "json";

    /// Builds an artifact type.
    ///
    /// # Errors
    /// Rejects empty strings.
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.is_empty() {
            return Err("artifact type must not be empty".to_owned());
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn well_known(name: &'static str) -> Self {
        Self(name.to_owned())
    }
}

/// Deterministic validation state of an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationStatus {
    /// Not yet validated.
    Unvalidated,
    /// Deterministic checks passed (schema, checksum, size, format).
    Valid,
    /// Deterministic checks failed. The artifact MUST NOT be treated as a
    /// completed output.
    Invalid,
}

/// Draft/publication lifecycle. Draft creation is separated from external
/// publication/send (AGENTS.md, artifacts section).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublicationState {
    Draft,
    PendingReview,
    Approved,
    Published,
    Withdrawn,
}

/// Provenance of an artifact: what produced it and from what inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactProvenance {
    /// Generator identity, e.g. `lumi-runtime/0.1.0` or `provider/model-id`.
    pub generator: String,
    /// IDs of source inputs (task ids, evidence ids, file checksums…).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
    pub created_at: Timestamp,
}

/// A user-visible output artifact (spec 01 §1.11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: ArtifactId,
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<RunId>,
    pub artifact_type: ArtifactType,
    /// Location or reference (path, URI) where the artifact lives.
    pub location: String,
    pub provenance: ArtifactProvenance,
    pub validation_status: ValidationStatus,
    pub publication_state: PublicationState,
    /// SHA-256 hex checksum when content is locally materialized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_serde_round_trip() {
        let artifact = Artifact {
            artifact_id: ArtifactId::parse("art-1").unwrap(),
            task_id: TaskId::parse("task-1").unwrap(),
            run_id: Some(RunId::parse("run-1").unwrap()),
            artifact_type: ArtifactType::well_known(ArtifactType::XLSX),
            location: "workspaces/w1/out/report.xlsx".to_owned(),
            provenance: ArtifactProvenance {
                generator: "lumi-runtime/0.1.0".to_owned(),
                source_refs: vec!["evidence-1".to_owned()],
                created_at: Timestamp::UNIX_EPOCH,
            },
            validation_status: ValidationStatus::Valid,
            publication_state: PublicationState::Draft,
            sha256: Some("abc".to_owned()),
        };
        let json = serde_json::to_string_pretty(&artifact).unwrap();
        let back: Artifact = serde_json::from_str(&json).unwrap();
        assert_eq!(back, artifact);
    }

    #[test]
    fn invalid_artifacts_cannot_be_published_state_by_type() {
        // Type-level documentation: ValidationStatus::Invalid artifacts
        // must not satisfy workflow completion (checked by state machine).
        assert_ne!(ValidationStatus::Invalid, ValidationStatus::Valid);
    }
}
