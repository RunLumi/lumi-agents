//! Lumi change sets (spec 26 §26.18).
//!
//! A task that mutates project files maintains an inspectable change
//! set: what was created, modified, moved, deleted, and restored — with
//! before/after checksums, a bounded text patch where practical, and the
//! task that produced it. The change set answers "what did Lumi change
//! and is it correct?", not raw tool-call telemetry.
//!
//! Source classification: entries record whether they were produced by
//! the agent or observed as an external conflict (§26.24). Pre-existing
//! user changes are NOT entries here; they are detected at task start
//! via Git state (spec 26 §26.19) and never claimed by Lumi.

use crate::error::ProjectError;
use crate::validation::{ValidationRecord, ValidationStatus};
use lumi_protocol::canonical::sha256_hex;
use lumi_protocol::{TaskId, Timestamp};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What kind of mutation an entry records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Created,
    Modified,
    Moved,
    Deleted,
    Restored,
}

/// Who produced the change (§26.18: Lumi must be able to distinguish
/// its own work from the world's).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSource {
    /// Applied by the Lumi agent for this task.
    Agent,
    /// An external modification detected while Lumi was working
    /// (stale-write conflict); Lumi did NOT apply it.
    ExternalConflict,
}

/// One recorded file change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeEntry {
    pub kind: ChangeKind,
    pub source: ChangeSource,
    /// Project-relative path after the change (for moves: destination).
    pub path: String,
    /// Project-relative source path for moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_after: Option<String>,
    /// Bounded unified diff for text modifications (§26.31 evidence
    /// minimization: binary and oversized patches are skipped).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<String>,
    pub task_id: TaskId,
    pub recorded_at: Timestamp,
}

/// One bounded shell command a task ran (§26.18 "commands run").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    /// Why the command ran ("validate test", "install deps", ...).
    pub purpose: String,
    /// The exact command as submitted.
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub recorded_at: Timestamp,
}

/// The change set of one task against one project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub project_id: String,
    pub task_id: TaskId,
    pub entries: Vec<ChangeEntry>,
    pub updated_at: Timestamp,
    /// Bounded shell commands the task ran (§26.18).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<CommandRecord>,
    /// Validations the task executed with honest outcomes (§26.20).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validations: Vec<ValidationRecord>,
}

impl ChangeSet {
    #[must_use]
    pub fn new(project_id: &str, task_id: TaskId, now: Timestamp) -> Self {
        Self {
            project_id: project_id.to_owned(),
            task_id,
            entries: Vec::new(),
            updated_at: now,
            commands: Vec::new(),
            validations: Vec::new(),
        }
    }

    /// Appends an entry and refreshes the update time.
    pub fn push(&mut self, entry: ChangeEntry, now: Timestamp) {
        self.entries.push(entry);
        self.updated_at = now;
    }

    /// Agent-produced entries only (§26.18: inspect what Lumi changed).
    pub fn agent_entries(&self) -> impl Iterator<Item = &ChangeEntry> {
        self.entries
            .iter()
            .filter(|e| e.source == ChangeSource::Agent)
    }

    /// Appends a command record (§26.18 "commands run").
    pub fn push_command(&mut self, command: CommandRecord, now: Timestamp) {
        self.commands.push(command);
        self.updated_at = now;
    }

    /// Appends a validation record (§26.20 honest outcomes).
    pub fn push_validation(&mut self, validation: ValidationRecord) {
        self.validations.push(validation);
        self.updated_at = Timestamp::now();
    }

    /// The validation that decides the task's honest completion claim:
    /// `Some(true)` only when every recorded validation passed, `None`
    /// when nothing was validated.
    #[must_use]
    pub fn all_validations_passed(&self) -> Option<bool> {
        if self.validations.is_empty() {
            return None;
        }
        Some(
            self.validations
                .iter()
                .all(|v| v.status == ValidationStatus::Passed),
        )
    }

    /// True when the task changed nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Durable store for task change sets:
/// `<dir>/<project_id>/<task_id>.json`.
#[derive(Debug, Clone)]
pub struct ChangeSetStore {
    dir: PathBuf,
}

impl ChangeSetStore {
    #[must_use]
    pub const fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path_for(&self, project_id: &str, task_id: &TaskId) -> PathBuf {
        self.dir
            .join(project_id)
            .join(format!("{}.json", task_id.as_str()))
    }

    /// Loads (or starts) the change set for a task.
    ///
    /// # Errors
    /// [`ProjectError::Serde`] for corrupt documents, [`ProjectError::Io`]
    /// for read failures.
    pub fn load(&self, project_id: &str, task_id: &TaskId) -> Result<ChangeSet, ProjectError> {
        let path = self.path_for(project_id, task_id);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ChangeSet::new(
                    project_id,
                    task_id.clone(),
                    Timestamp::now(),
                ));
            }
            Err(e) => return Err(ProjectError::Io(e.to_string())),
        };
        serde_json::from_str(&text)
            .map_err(|e| ProjectError::Serde(format!("corrupt change set: {e}")))
    }

    /// Persists the change set atomically (temp + rename).
    ///
    /// # Errors
    /// [`ProjectError::Io`] / [`ProjectError::Serde`].
    pub fn save(&self, set: &ChangeSet) -> Result<(), ProjectError> {
        let path = self.path_for(&set.project_id, &set.task_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ProjectError::Io(e.to_string()))?;
        }
        let text =
            serde_json::to_string_pretty(set).map_err(|e| ProjectError::Serde(e.to_string()))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| ProjectError::Io(e.to_string()))?;
        std::fs::rename(&tmp, &path).map_err(|e| ProjectError::Io(e.to_string()))
    }

    /// Change sets for one project, most recently updated first.
    ///
    /// # Errors
    /// [`ProjectError::Io`] / [`ProjectError::Serde`] per corrupt entry —
    /// corrupt documents are skipped, not fatal (the change set is
    /// evidence, not authority state).
    pub fn list_for_project(&self, project_id: &str) -> Result<Vec<ChangeSet>, ProjectError> {
        let mut sets = Vec::new();
        let project_dir = self.dir.join(project_id);
        let entries = match std::fs::read_dir(&project_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(sets),
            Err(e) => return Err(ProjectError::Io(e.to_string())),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(set) = serde_json::from_str::<ChangeSet>(&text) {
                    sets.push(set);
                }
            }
        }
        sets.sort_by(|a, b| {
            let a_key = (a.updated_at.epoch_seconds(), a.updated_at.nanoseconds());
            let b_key = (b.updated_at.epoch_seconds(), b.updated_at.nanoseconds());
            b_key.cmp(&a_key)
        });
        Ok(sets)
    }
}

/// Builds a bounded unified-style patch between two text versions
/// (§26.18 "patch/diff where practical"). Binary content and oversized
/// patches are skipped (`None`).
///
/// The v1 diff is a single replace-hunk after longest common
/// prefix/suffix trimming — deterministic, dependency-free, and faithful
/// for the small, targeted edits project work produces.
#[must_use]
pub fn text_patch(
    before: &[u8],
    after: &[u8],
    path: &str,
    max_patch_bytes: usize,
) -> Option<String> {
    let is_text = |bytes: &[u8]| !bytes.get(..8_192).unwrap_or(bytes).contains(&0);
    if !is_text(before) || !is_text(after) {
        return None;
    }
    let before_lines: Vec<&[u8]> = split_lines(before);
    let after_lines: Vec<&[u8]> = split_lines(after);

    let mut start = 0usize;
    while start < before_lines.len()
        && start < after_lines.len()
        && before_lines[start] == after_lines[start]
    {
        start += 1;
    }
    let mut end_before = before_lines.len();
    let mut end_after = after_lines.len();
    while end_before > start
        && end_after > start
        && before_lines[end_before - 1] == after_lines[end_after - 1]
    {
        end_before -= 1;
        end_after -= 1;
    }

    let mut patch = String::new();
    patch.push_str(&format!("@@ -{path} @@\n"));
    let push_line = |patch: &mut String, marker: char, line: &[u8]| {
        patch.push(marker);
        patch.push_str(&String::from_utf8_lossy(line));
        patch.push('\n');
    };
    for line in &before_lines[start..end_before] {
        push_line(&mut patch, '-', line);
    }
    for line in &after_lines[start..end_after] {
        push_line(&mut patch, '+', line);
    }
    if patch.len() > max_patch_bytes {
        return None;
    }
    if patch.ends_with("@@\n") {
        return None; // no differences
    }
    Some(patch)
}

fn split_lines(bytes: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' {
            lines.push(&bytes[start..=i]);
            start = i + 1;
        }
    }
    if start < bytes.len() {
        lines.push(&bytes[start..]);
    }
    lines
}

/// Convenience: checksum helper for change bookkeeping.
#[must_use]
pub fn checksum(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str) -> TaskId {
        TaskId::parse(id).unwrap()
    }

    #[test]
    fn change_set_records_and_classifies() {
        let now = Timestamp::from_epoch(0, 0).unwrap();
        let mut set = ChangeSet::new("p-1", task("task-1"), now);
        set.push(
            ChangeEntry {
                kind: ChangeKind::Modified,
                source: ChangeSource::Agent,
                path: "src/lib.rs".to_owned(),
                from_path: None,
                sha256_before: Some("aaa".to_owned()),
                sha256_after: Some("bbb".to_owned()),
                patch: None,
                task_id: task("task-1"),
                recorded_at: now,
            },
            now,
        );
        set.push(
            ChangeEntry {
                kind: ChangeKind::Modified,
                source: ChangeSource::ExternalConflict,
                path: "src/other.rs".to_owned(),
                from_path: None,
                sha256_before: None,
                sha256_after: None,
                patch: None,
                task_id: task("task-1"),
                recorded_at: now,
            },
            now,
        );
        assert_eq!(set.entries.len(), 2);
        assert_eq!(set.agent_entries().count(), 1);
        assert!(!set.is_empty());
    }

    #[test]
    fn change_set_store_round_trips_and_lists() {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-changeset-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let store = ChangeSetStore::new(dir.clone());
        let now = Timestamp::from_epoch(5, 0).unwrap();
        let mut set = store.load("proj-1", &task("task-1")).unwrap();
        assert!(set.is_empty(), "missing set starts empty");
        set.push(
            ChangeEntry {
                kind: ChangeKind::Created,
                source: ChangeSource::Agent,
                path: "new.txt".to_owned(),
                from_path: None,
                sha256_before: None,
                sha256_after: Some(checksum(b"hello")),
                patch: None,
                task_id: task("task-1"),
                recorded_at: now,
            },
            now,
        );
        store.save(&set).unwrap();
        let reloaded = store.load("proj-1", &task("task-1")).unwrap();
        assert_eq!(reloaded.entries.len(), 1);
        let all = store.list_for_project("proj-1").unwrap();
        assert_eq!(all.len(), 1);
        assert!(store.list_for_project("proj-none").unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn text_patch_produces_replace_hunk() {
        let patch = text_patch(b"a\nb\nc\n", b"a\nB\nc\n", "f.txt", 4096).unwrap();
        assert!(patch.contains("-b\n"));
        assert!(patch.contains("+B\n"));
        assert!(patch.contains("f.txt"));
    }

    #[test]
    fn text_patch_skips_binary_and_oversized() {
        assert!(text_patch(&[0, 1, 2], &[3, 4], "bin", 4096).is_none());
        let big_after = vec![b'x'; 8192];
        assert!(text_patch(b"a\n", &big_after, "f.txt", 1024).is_none());
        assert!(text_patch(b"same\n", b"same\n", "f.txt", 4096).is_none());
    }
}
