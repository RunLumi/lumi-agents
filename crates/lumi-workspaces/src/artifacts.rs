//! Artifact pipeline: provenance, validation, lifecycle (spec 08
//! §8.10–8.13).
//!
//! Publication is strictly separated from generation: an artifact moves
//! DRAFT → VALIDATING → READY_FOR_REVIEW → APPROVED → PUBLISHED (or
//! REJECTED), and only APPROVED artifacts can be published. Every record
//! carries producing task/run, source refs, generator, checksum, and
//! recorded validation outcomes (§8.12).

use crate::paths::resolve_in_workspace;
use lumi_protocol::Timestamp;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

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
        validate_artifact_id(artifact_id)?;
        if file_name == "artifact.json" {
            return Err(ArtifactError::Io(
                "artifact.json is reserved for the artifact index".to_owned(),
            ));
        }
        let rel_dir = Path::new(artifact_id);
        let abs_dir = self.resolve_path(rel_dir)?;
        if abs_dir.exists() {
            return Err(ArtifactError::Io(format!(
                "artifact id already exists: {artifact_id}"
            )));
        }
        let abs_file = self.resolve_artifact_file(artifact_id, file_name)?;
        std::fs::create_dir_all(&abs_dir)
            .map_err(|e| ArtifactError::Io(format!("creating artifact dir: {e}")))?;
        std::fs::write(&abs_file, content)
            .map_err(|e| ArtifactError::Io(format!("writing artifact: {e}")))?;
        let record = ArtifactRecord {
            artifact_id: artifact_id.to_owned(),
            artifact_type: artifact_type.to_owned(),
            path: rel_dir.join(file_name).display().to_string(),
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
        validate_artifact_id(&record.artifact_id)?;
        let artifact_dir = self.resolve_path(Path::new(&record.artifact_id))?;
        let content_path = self.resolve_path(Path::new(&record.path))?;
        if !content_path.starts_with(&artifact_dir) || content_path == artifact_dir {
            return Err(ArtifactError::Io(
                "artifact record path escapes its artifact directory".to_owned(),
            ));
        }
        let content = std::fs::read(&content_path)
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
        validate_artifact_id(&record.artifact_id)?;
        let index = self.resolve_path(&Path::new(&record.artifact_id).join("artifact.json"))?;
        let text =
            serde_json::to_string_pretty(record).map_err(|e| ArtifactError::Io(e.to_string()))?;
        std::fs::write(index, text).map_err(|e| ArtifactError::Io(e.to_string()))
    }

    fn resolve_path(&self, requested: &Path) -> Result<PathBuf, ArtifactError> {
        resolve_in_workspace(&self.root, requested)
            .map_err(|error| ArtifactError::Io(format!("unsafe artifact path: {error}")))
    }

    fn resolve_artifact_file(
        &self,
        artifact_id: &str,
        file_name: &str,
    ) -> Result<PathBuf, ArtifactError> {
        validate_artifact_id(artifact_id)?;
        let artifact_dir = self.resolve_path(Path::new(artifact_id))?;
        let file_path = Path::new(file_name);
        if file_path.is_absolute() {
            return Err(ArtifactError::Io(
                "artifact file name must be relative".to_owned(),
            ));
        }
        let resolved = self.resolve_path(&Path::new(artifact_id).join(file_path))?;
        if !resolved.starts_with(&artifact_dir) || resolved == artifact_dir {
            return Err(ArtifactError::Io(
                "artifact file path escapes its artifact directory".to_owned(),
            ));
        }
        Ok(resolved)
    }
}

fn validate_artifact_id(artifact_id: &str) -> Result<(), ArtifactError> {
    let mut components = Path::new(artifact_id).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(ArtifactError::Io(
            "artifact id must be one nonempty path component".to_owned(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (PathBuf, ArtifactStore) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-artifacts-{}-{}-{}",
            std::process::id(),
            format!("{:?}", std::thread::current().id())
                .replace("ThreadId(", "")
                .replace(")", ""),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
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

    #[test]
    fn create_refuses_traversal_absolute_names_and_symlink_escapes() {
        let (dir, store) = store();
        let outside_dir = dir
            .parent()
            .unwrap()
            .join(format!("lumi-artifacts-outside-{}", std::process::id()));
        std::fs::create_dir_all(&outside_dir).unwrap();
        let outside_file = outside_dir.join("sentinel.txt");
        std::fs::write(&outside_file, b"keep").unwrap();

        let traversal = store.create(
            "../artifact-escape",
            "text",
            "output.txt",
            b"must not write",
            provenance(),
        );
        assert!(traversal.is_err(), "artifact id traversal must be refused");

        let absolute_name = store.create(
            "safe",
            "text",
            &outside_file.display().to_string(),
            b"must not write",
            provenance(),
        );
        assert!(
            absolute_name.is_err(),
            "absolute artifact names must be refused"
        );

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside_dir, dir.join("escape")).unwrap();
            let symlink_escape = store.create(
                "escape",
                "text",
                "output.txt",
                b"must not write",
                provenance(),
            );
            assert!(symlink_escape.is_err(), "symlink escape must be refused");
        }

        assert_eq!(std::fs::read(&outside_file).unwrap(), b"keep");
        std::fs::remove_dir_all(&outside_dir).ok();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validate_refuses_tampered_record_path() {
        let (dir, store) = store();
        let mut record = store
            .create("safe", "text", "output.txt", b"inside", provenance())
            .unwrap();
        let outside = dir
            .parent()
            .unwrap()
            .join(format!("lumi-artifacts-tampered-{}", std::process::id()));
        std::fs::write(&outside, b"outside sentinel").unwrap();
        record.path = outside.display().to_string();
        assert!(store.validate(&record, &[]).is_err());
        assert_eq!(std::fs::read(&outside).unwrap(), b"outside sentinel");
        std::fs::remove_file(&outside).ok();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn artifact_id_and_reserved_index_names_fail_before_writes() {
        let (dir, store) = store();
        for artifact_id in ["", "nested/id", "..", "/absolute"] {
            assert!(
                store
                    .create(
                        artifact_id,
                        "text",
                        "output.txt",
                        b"must not write",
                        provenance(),
                    )
                    .is_err(),
                "artifact id {artifact_id:?} must be refused"
            );
        }
        assert!(store
            .create(
                "reserved",
                "text",
                "artifact.json",
                b"must not overwrite index",
                provenance(),
            )
            .is_err());
        assert!(!dir.join("reserved").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn duplicate_artifact_id_does_not_overwrite_published_content() {
        let (dir, store) = store();
        let record = store
            .create("stable", "text", "output.txt", b"original", provenance())
            .unwrap();
        let validated = store.validate(&record, &[]).unwrap();
        let approved = store.approve(&validated).unwrap();
        let published = store.publish(&approved).unwrap();
        assert_eq!(published.lifecycle, ArtifactLifecycle::Published);

        assert!(store
            .create("stable", "text", "output.txt", b"replacement", provenance())
            .is_err());
        assert_eq!(
            std::fs::read(dir.join("stable").join("output.txt")).unwrap(),
            b"original"
        );
        let index = std::fs::read_to_string(dir.join("stable").join("artifact.json")).unwrap();
        assert!(index.contains("PUBLISHED"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validation_cannot_read_another_artifacts_file() {
        let (dir, store) = store();
        let record_a = store
            .create("artifact-a", "text", "a.txt", b"a", provenance())
            .unwrap();
        let record_b = store
            .create("artifact-b", "text", "b.txt", b"b", provenance())
            .unwrap();
        let mut tampered = record_a;
        tampered.path = record_b.path;
        assert!(store.validate(&tampered, &[]).is_err());
        assert_eq!(
            std::fs::read(dir.join("artifact-b").join("b.txt")).unwrap(),
            b"b"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
