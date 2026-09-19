//! Project memory (spec 26 §26.22).
//!
//! Durable, project-scoped memory for stable knowledge: validated
//! commands, conventions, environment requirements, recovery procedures.
//! Every record carries REQUIRED provenance (what task/command/evidence
//! produced it) — arbitrary repository text, model guesses, and
//! transient task output are refused. Memory linked to a Git HEAD is
//! automatically stale once that HEAD moves, and every record can be
//! invalidated with a reason. Memory is context, never authority.

use crate::error::ProjectError;
use lumi_protocol::{TaskId, Timestamp};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What kind of stable knowledge this record holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// A command that has actually run and passed in this project.
    ValidatedCommand,
    /// A durable architectural/team convention.
    Convention,
    /// A known environment requirement (toolchain, OS feature).
    EnvironmentRequirement,
    /// A proven recovery procedure.
    RecoveryProcedure,
}

/// Why the record exists: provenance is mandatory (§26.22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProvenance {
    /// The task that produced/validated the knowledge.
    pub task_id: TaskId,
    /// The exact command run, when the memory is command-shaped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Git HEAD the knowledge was validated against, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_head: Option<String>,
    /// Free-form evidence reference (validation record, diff, log ref).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// One durable project memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub memory_id: String,
    pub kind: MemoryKind,
    pub content: String,
    pub provenance: MemoryProvenance,
    /// Git HEAD at validation time; when the project's HEAD differs the
    /// record reports stale (confidence decays, it is not deleted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validated_at_head: Option<String>,
    pub created_at: Timestamp,
    /// Set when a human or the runtime invalidates the record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalidated: Option<Invalidation>,
}

/// An explicit invalidation with its reason (§26.22: memory is
/// invalidatable when the project materially changes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invalidation {
    pub reason: String,
    pub at: Timestamp,
}

impl MemoryRecord {
    /// Creates a validated record; refuses empty content and provenance
    /// gaps (a command memory without a command is a guess).
    ///
    /// # Errors
    /// [`ProjectError::InvalidRecord`] when content or provenance is
    /// insufficient.
    pub fn new(
        memory_id: String,
        kind: MemoryKind,
        content: &str,
        provenance: MemoryProvenance,
        validated_at_head: Option<String>,
        now: Timestamp,
    ) -> Result<Self, ProjectError> {
        if content.trim().is_empty() {
            return Err(ProjectError::InvalidRecord(
                "memory content must not be empty".to_owned(),
            ));
        }
        if kind == MemoryKind::ValidatedCommand && provenance.command.is_none() {
            return Err(ProjectError::InvalidRecord(
                "a validated-command memory requires the exact command as provenance".to_owned(),
            ));
        }
        Ok(Self {
            memory_id,
            kind,
            content: content.trim().to_owned(),
            provenance,
            validated_at_head,
            created_at: now,
            invalidated: None,
        })
    }

    /// True when the record is not invalidated AND its validation head
    /// still matches the project (when it had one).
    #[must_use]
    pub fn is_trustworthy(&self, current_head: Option<&str>) -> bool {
        if self.invalidated.is_some() {
            return false;
        }
        match (&self.validated_at_head, current_head) {
            (Some(recorded), Some(current)) => recorded == current,
            (Some(_), None) => false,
            (None, _) => true,
        }
    }
}

/// Durable project memory store: `<dir>/<project_id>.json`.
#[derive(Debug, Clone)]
pub struct ProjectMemoryStore {
    dir: PathBuf,
}

impl ProjectMemoryStore {
    #[must_use]
    pub const fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path_for(&self, project_id: &str) -> PathBuf {
        self.dir.join(format!("{project_id}.json"))
    }

    /// Loads all records for a project (empty when none).
    ///
    /// # Errors
    /// [`ProjectError::Serde`] for corrupt documents.
    pub fn load(&self, project_id: &str) -> Result<Vec<MemoryRecord>, ProjectError> {
        let path = self.path_for(project_id);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(ProjectError::Io(e.to_string())),
        };
        serde_json::from_str(&text)
            .map_err(|e| ProjectError::Serde(format!("corrupt project memory: {e}")))
    }

    /// Persists records atomically.
    ///
    /// # Errors
    /// [`ProjectError::Io`] / [`ProjectError::Serde`].
    pub fn save(&self, project_id: &str, records: &[MemoryRecord]) -> Result<(), ProjectError> {
        let path = self.path_for(project_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ProjectError::Io(e.to_string()))?;
        }
        let text = serde_json::to_string_pretty(records)
            .map_err(|e| ProjectError::Serde(e.to_string()))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| ProjectError::Io(e.to_string()))?;
        std::fs::rename(&tmp, &path).map_err(|e| ProjectError::Io(e.to_string()))
    }

    /// Records validated knowledge with mandatory provenance.
    ///
    /// # Errors
    /// [`ProjectError::InvalidRecord`] for guess-shaped records; I/O
    /// errors on persistence.
    pub fn remember(&self, project_id: &str, record: MemoryRecord) -> Result<(), ProjectError> {
        let mut records = self.load(project_id)?;
        records.retain(|r| r.memory_id != record.memory_id);
        records.push(record);
        self.save(project_id, &records)
    }

    /// Invalidates a record with a reason; the record stays for audit.
    ///
    /// # Errors
    /// [`ProjectError::NotFound`] when the memory id is unknown.
    pub fn invalidate(
        &self,
        project_id: &str,
        memory_id: &str,
        reason: &str,
        now: Timestamp,
    ) -> Result<(), ProjectError> {
        let mut records = self.load(project_id)?;
        let record = records
            .iter_mut()
            .find(|r| r.memory_id == memory_id)
            .ok_or_else(|| ProjectError::NotFound {
                project_id: memory_id.to_owned(),
            })?;
        record.invalidated = Some(Invalidation {
            reason: reason.to_owned(),
            at: now,
        });
        self.save(project_id, &records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str) -> TaskId {
        TaskId::parse(id).unwrap()
    }

    fn record(head: Option<String>) -> MemoryRecord {
        MemoryRecord::new(
            "mem-1".to_owned(),
            MemoryKind::ValidatedCommand,
            "cargo test --workspace validates the sync module",
            MemoryProvenance {
                task_id: task("task-1"),
                command: Some("cargo test --workspace".to_owned()),
                git_head: head.clone(),
                evidence: Some("validation:passed".to_owned()),
            },
            head,
            Timestamp::from_epoch(0, 0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn provenance_is_required_for_command_memories() {
        let err = MemoryRecord::new(
            "mem-2".to_owned(),
            MemoryKind::ValidatedCommand,
            "npm test works",
            MemoryProvenance {
                task_id: task("task-1"),
                command: None,
                git_head: None,
                evidence: None,
            },
            None,
            Timestamp::from_epoch(0, 0).unwrap(),
        )
        .unwrap_err();
        assert!(matches!(err, ProjectError::InvalidRecord(_)));
        assert!(MemoryRecord::new(
            "mem-3".to_owned(),
            MemoryKind::Convention,
            "",
            MemoryProvenance {
                task_id: task("task-1"),
                command: None,
                git_head: None,
                evidence: None,
            },
            None,
            Timestamp::from_epoch(0, 0).unwrap(),
        )
        .is_err());
    }

    #[test]
    fn trustworthiness_tracks_head_and_invalidation() {
        let fresh = record(Some("head-a".to_owned()));
        assert!(fresh.is_trustworthy(Some("head-a")));
        assert!(!fresh.is_trustworthy(Some("head-b")), "HEAD moved: stale");
        assert!(!fresh.is_trustworthy(None), "unborn HEAD cannot confirm");

        let mut invalidated = record(Some("head-a".to_owned()));
        invalidated.invalidated = Some(Invalidation {
            reason: "command removed from CI".to_owned(),
            at: Timestamp::from_epoch(9, 0).unwrap(),
        });
        assert!(!invalidated.is_trustworthy(Some("head-a")));
    }

    #[test]
    fn memory_store_round_trips_and_invalidates() {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-project-memory-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let store = ProjectMemoryStore::new(dir.clone());
        assert!(store.load("p-1").unwrap().is_empty());
        store
            .remember("p-1", record(Some("head-a".to_owned())))
            .unwrap();
        store
            .invalidate(
                "p-1",
                "mem-1",
                "superseded",
                Timestamp::from_epoch(5, 0).unwrap(),
            )
            .unwrap();
        let records = store.load("p-1").unwrap();
        assert_eq!(records.len(), 1, "invalidation keeps the record for audit");
        assert!(!records[0].is_trustworthy(Some("head-a")));
        assert!(store
            .invalidate(
                "p-1",
                "mem-unknown",
                "x",
                Timestamp::from_epoch(6, 0).unwrap()
            )
            .is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
