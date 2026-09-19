//! Artifact pipeline: provenance, validation, lifecycle (spec 08
//! §8.10–8.13).
//!
//! Publication is strictly separated from generation: an artifact moves
//! DRAFT → VALIDATING → READY_FOR_REVIEW → APPROVED → PUBLISHED (or
//! REJECTED), and only APPROVED artifacts can be published. Every record
//! carries producing task/run, source refs, generator, checksum, and
//! recorded validation outcomes (§8.12).

use lumi_protocol::Timestamp;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Lifecycle states (§8.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactLifecycle {
    Draft,
    Validating,
    ReadyForReview,
    Approved,
    Published,
    Rejected,
}

/// Artifact types with their built-in deterministic validations (§8.10,
/// §8.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltInValidator {
    /// Content is valid UTF-8.
    Utf8,
    /// Starts with the PDF magic (`%PDF-`).
    PdfMagic,
    /// ZIP container magic (`PK\x03\x04`) — DOCX/XLSX/PPTX are ZIPs.
    ZipMagic,
    /// PNG magic bytes.
    PngMagic,
    /// CSV parses: consistent column count across rows, non-empty.
    CsvShape,
    /// Content parses as a JSON object/array.
    JsonParse,
}

impl BuiltInValidator {
    /// Runs the deterministic check.
    ///
    /// # Errors
    /// Human-readable detail when the content fails the check.
    pub fn check(&self, content: &[u8]) -> Result<(), String> {
        match self {
            Self::Utf8 => std::str::from_utf8(content)
                .map(|_| ())
                .map_err(|e| format!("not valid UTF-8: {e}")),
            Self::PdfMagic => {
                if content.starts_with(b"%PDF-") {
                    Ok(())
                } else {
                    Err("missing %PDF- magic".to_owned())
                }
            }
            Self::ZipMagic => {
                if content.starts_with(b"PK\x03\x04") {
                    Ok(())
                } else {
                    Err("missing ZIP container magic (PK)".to_owned())
                }
            }
            Self::PngMagic => {
                if content.starts_with(&[0x89, b'P', b'N', b'G']) {
                    Ok(())
                } else {
                    Err("missing PNG magic".to_owned())
                }
            }
            Self::CsvShape => {
                let text =
                    std::str::from_utf8(content).map_err(|_| "CSV is not UTF-8".to_owned())?;
                let mut rows = 0usize;
                let mut columns: Option<usize> = None;
                for line in text.lines().filter(|l| !l.is_empty()) {
                    rows += 1;
                    let count = line.split(',').count();
                    match columns {
                        None => columns = Some(count),
                        Some(expected) if expected != count => {
                            return Err(format!(
                                "row {rows} has {count} columns, expected {expected}"
                            ));
                        }
                        Some(_) => {}
                    }
                }
                if rows == 0 {
                    return Err("CSV is empty".to_owned());
                }
                Ok(())
            }
            Self::JsonParse => {
                let text =
                    std::str::from_utf8(content).map_err(|_| "not valid UTF-8".to_owned())?;
                let value: serde_json::Value =
                    serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?;
                if value.is_object() || value.is_array() {
                    Ok(())
                } else {
                    Err("JSON must be an object or array".to_owned())
                }
            }
        }
    }

    /// The default validators per artifact type (§8.13 examples).
    #[must_use]
    pub fn defaults_for(artifact_type: &str) -> Vec<Self> {
        match artifact_type {
            "markdown" | "text" => vec![Self::Utf8],
            "pdf" => vec![Self::PdfMagic],
            "docx" | "xlsx" | "pptx" => vec![Self::ZipMagic],
            "csv" => vec![Self::Utf8, Self::CsvShape],
            "json" => vec![Self::Utf8, Self::JsonParse],
            "code" => vec![Self::Utf8],
            _ => vec![],
        }
    }
}

/// A caller-supplied extra validation check.
#[allow(clippy::type_complexity)]
pub struct CustomCheck<'a>(pub Box<dyn Fn(&[u8]) -> Result<(), String> + 'a>);

impl std::fmt::Debug for CustomCheck<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CustomCheck")
    }
}

/// One recorded validation outcome (§8.12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationOutcome {
    pub validator: String,
    pub passed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Provenance (§8.12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactProvenance {
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
    /// Generator/tool/model identity.
    pub generator: String,
    pub created_at: Timestamp,
}

/// A stored artifact record (§8.11–8.12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub artifact_type: String,
    /// Path relative to the store root.
    pub path: String,
    pub lifecycle: ArtifactLifecycle,
    pub provenance: ArtifactProvenance,
    /// SHA-256 of content, recorded at validation (§8.12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validations: Vec<ValidationOutcome>,
}

/// Lifecycle transition / validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactError {
    /// The requested lifecycle move is not allowed.
    InvalidTransition {
        from: ArtifactLifecycle,
        to: &'static str,
    },
    /// A required validation failed (artifact → REJECTED content, but the
    /// record stays for evidence).
    ValidationFailed {
        failures: Vec<ValidationOutcome>,
    },
    Io(String),
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "cannot move artifact from {from:?} to {to}")
            }
            Self::ValidationFailed { failures } => {
                let failed: Vec<String> = failures
                    .iter()
                    .filter(|f| !f.passed)
                    .map(|f| f.validator.clone())
                    .collect();
                write!(f, "validation failed: {}", failed.join(", "))
            }
            Self::Io(e) => write!(f, "artifact io error: {e}"),
        }
    }
}

impl std::error::Error for ArtifactError {}

/// A store of artifacts under one root with an index.
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    /// Creates/opens a store rooted at `root` (typically inside the
    /// workspace: `<root>/artifacts`).
    ///
    /// # Errors
    /// Filesystem errors.
    pub fn open(root: &Path) -> Result<Self, ArtifactError> {
        std::fs::create_dir_all(root)
            .map_err(|e| ArtifactError::Io(format!("creating artifact store: {e}")))?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// The store root (for evidence readers).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Persists content as a new DRAFT artifact with provenance.
    ///
    /// # Errors
    /// Filesystem/serialization failures.
    pub fn create(
        &self,
        artifact_id: &str,
        artifact_type: &str,
        file_name: &str,
        content: &[u8],
        provenance: ArtifactProvenance,
    ) -> Result<ArtifactRecord, ArtifactError> {
        let rel_dir = Path::new(artifact_id);
        let abs_dir = self.root.join(rel_dir);
        std::fs::create_dir_all(&abs_dir)
            .map_err(|e| ArtifactError::Io(format!("creating artifact dir: {e}")))?;
        let abs_file = abs_dir.join(file_name);
        std::fs::write(&abs_file, content)
            .map_err(|e| ArtifactError::Io(format!("writing artifact: {e}")))?;
        let record = ArtifactRecord {
            artifact_id: artifact_id.to_owned(),
            artifact_type: artifact_type.to_owned(),
            path: abs_dir.join(file_name).display().to_string(),
            lifecycle: ArtifactLifecycle::Draft,
            provenance,
            sha256: None,
            validations: Vec::new(),
        };
        self.persist_index(&record)?;
        Ok(record)
    }

    /// Runs the type's built-in validators plus any extra checks,
    /// recording outcomes. All must pass to reach READY_FOR_REVIEW;
    /// failures move the artifact to REJECTED (record retained).
    ///
    /// # Errors
    /// [`ArtifactError::ValidationFailed`] when any check fails;
    /// [`ArtifactError::InvalidTransition`] from a non-DRAFT state.
    pub fn validate(
        &self,
        record: &ArtifactRecord,
        extra: &[CustomCheck<'_>],
    ) -> Result<ArtifactRecord, ArtifactError> {
        if record.lifecycle != ArtifactLifecycle::Draft {
            return Err(ArtifactError::InvalidTransition {
                from: record.lifecycle,
                to: "VALIDATING",
            });
        }
        let content = std::fs::read(&record.path)
            .map_err(|e| ArtifactError::Io(format!("reading artifact: {e}")))?;
        let mut outcomes = Vec::new();
        for validator in BuiltInValidator::defaults_for(&record.artifact_type) {
            let outcome = match validator.check(&content) {
                Ok(()) => ValidationOutcome {
                    validator: format!("{validator:?}"),
                    passed: true,
                    detail: None,
                },
                Err(detail) => ValidationOutcome {
                    validator: format!("{validator:?}"),
                    passed: false,
                    detail: Some(detail),
                },
            };
            outcomes.push(outcome);
        }
        for check in extra {
            let outcome = match (check.0)(&content) {
                Ok(()) => ValidationOutcome {
                    validator: "custom".to_owned(),
                    passed: true,
                    detail: None,
                },
                Err(detail) => ValidationOutcome {
                    validator: "custom".to_owned(),
                    passed: false,
                    detail: Some(detail),
                },
            };
            outcomes.push(outcome);
        }
        let all_passed = outcomes.iter().all(|o| o.passed);
        let mut validated = record.clone();
        validated.validations = outcomes.clone();
        validated.sha256 = Some(lumi_protocol::canonical::sha256_hex(&content));
        validated.lifecycle = if all_passed {
            ArtifactLifecycle::ReadyForReview
        } else {
            ArtifactLifecycle::Rejected
        };
        self.persist_index(&validated)?;
        if all_passed {
            Ok(validated)
        } else {
            Err(ArtifactError::ValidationFailed { failures: outcomes })
        }
    }

    /// Approves a validated artifact for publication (READY_FOR_REVIEW →
    /// APPROVED). Human-review action; the store only checks state.
    ///
    /// # Errors
    /// [`ArtifactError::InvalidTransition`].
    pub fn approve(&self, record: &ArtifactRecord) -> Result<ArtifactRecord, ArtifactError> {
        self.transition(
            record,
            ArtifactLifecycle::ReadyForReview,
            ArtifactLifecycle::Approved,
        )
    }

    /// Publishes an APPROVED artifact (§8.11: publication is separate
    /// from generation and requires explicit approval first).
    ///
    /// # Errors
    /// [`ArtifactError::InvalidTransition`].
    pub fn publish(&self, record: &ArtifactRecord) -> Result<ArtifactRecord, ArtifactError> {
        self.transition(
            record,
            ArtifactLifecycle::Approved,
            ArtifactLifecycle::Published,
        )
    }

    fn transition(
        &self,
        record: &ArtifactRecord,
        required_from: ArtifactLifecycle,
        to: ArtifactLifecycle,
    ) -> Result<ArtifactRecord, ArtifactError> {
        if record.lifecycle != required_from {
            return Err(ArtifactError::InvalidTransition {
                from: record.lifecycle,
                to: match to {
                    ArtifactLifecycle::Approved => "APPROVED",
                    ArtifactLifecycle::Published => "PUBLISHED",
                    _ => "UNKNOWN",
                },
            });
        }
        let mut moved = record.clone();
        moved.lifecycle = to;
        self.persist_index(&moved)?;
        Ok(moved)
    }

    fn persist_index(&self, record: &ArtifactRecord) -> Result<(), ArtifactError> {
        let index = self
            .root
            .join(record.artifact_id.clone())
            .join("artifact.json");
        let text =
            serde_json::to_string_pretty(record).map_err(|e| ArtifactError::Io(e.to_string()))?;
        std::fs::write(index, text).map_err(|e| ArtifactError::Io(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (PathBuf, ArtifactStore) {
        let dir = std::env::temp_dir().join(format!(
            "lumi-artifacts-{}-{}",
            std::process::id(),
            Timestamp::now().nanoseconds()
        ));
        let s = ArtifactStore::open(&dir).unwrap();
        (dir, s)
    }

    fn provenance() -> ArtifactProvenance {
        ArtifactProvenance {
            task_id: "task-art".to_owned(),
            run_id: Some("run-art".to_owned()),
            source_refs: vec!["evidence-1".to_owned()],
            generator: "lumi-runtime/0.1.0".to_owned(),
            created_at: Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn lifecycle_separates_generation_from_publication() {
        let (dir, store) = store();
        let record = store
            .create("art-1", "csv", "report.csv", b"a,b\n1,2\n", provenance())
            .unwrap();
        assert_eq!(record.lifecycle, ArtifactLifecycle::Draft);

        // Cannot publish straight from DRAFT (§8.11).
        assert!(matches!(
            store.publish(&record),
            Err(ArtifactError::InvalidTransition { .. })
        ));

        let validated = store.validate(&record, &[]).unwrap();
        assert_eq!(validated.lifecycle, ArtifactLifecycle::ReadyForReview);
        assert!(validated.sha256.is_some());
        assert!(validated.validations.iter().all(|v| v.passed));

        // Cannot publish without approval.
        assert!(matches!(
            store.publish(&validated),
            Err(ArtifactError::InvalidTransition { .. })
        ));
        let approved = store.approve(&validated).unwrap();
        let published = store.publish(&approved).unwrap();
        assert_eq!(published.lifecycle, ArtifactLifecycle::Published);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn failed_validation_rejects_and_records() {
        let (dir, store) = store();
        let record = store
            .create("art-2", "csv", "broken.csv", b"a,b\n1\n", provenance())
            .unwrap();
        let err = store.validate(&record, &[]).unwrap_err();
        let failures = match err {
            ArtifactError::ValidationFailed { failures } => failures,
            other => panic!("expected ValidationFailed, got {other:?}"),
        };
        assert!(failures.iter().any(|f| !f.passed));
        // The rejected record is persisted for evidence.
        let text = std::fs::read_to_string(store.root.join("art-2").join("artifact.json")).unwrap();
        assert!(text.contains("REJECTED"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn built_in_validators_match_content_kinds() {
        assert!(BuiltInValidator::PdfMagic.check(b"%PDF-1.7 rest").is_ok());
        assert!(BuiltInValidator::PdfMagic.check(b"not a pdf").is_err());
        assert!(BuiltInValidator::ZipMagic.check(b"PK\x03\x04...").is_ok());
        assert!(BuiltInValidator::PngMagic
            .check(&[0x89, b'P', b'N', b'G', 0x0D])
            .is_ok());
        assert!(BuiltInValidator::JsonParse.check(br#"{"k": 1}"#).is_ok());
        assert!(BuiltInValidator::JsonParse.check(b"42").is_err());
        assert!(BuiltInValidator::CsvShape.check(b"a,b\n1,2\n3,4\n").is_ok());
        assert!(BuiltInValidator::CsvShape.check(b"a,b\n1\n").is_err());
    }

    #[test]
    fn extra_custom_checks_participate_in_validation() {
        let (dir, store) = store();
        let record = store
            .create(
                "art-3",
                "json",
                "out.json",
                br#"{"total": 4200}"#,
                provenance(),
            )
            .unwrap();
        let total_is_positive = CustomCheck(Box::new(|content: &[u8]| {
            let value: serde_json::Value =
                serde_json::from_slice(content).map_err(|e| e.to_string())?;
            if value["total"].as_u64().unwrap_or(0) > 0 {
                Ok(())
            } else {
                Err("total must be positive".to_owned())
            }
        }));
        let validated = store.validate(&record, &[total_is_positive]).unwrap();
        assert_eq!(validated.lifecycle, ArtifactLifecycle::ReadyForReview);
        std::fs::remove_dir_all(&dir).ok();
    }
}
