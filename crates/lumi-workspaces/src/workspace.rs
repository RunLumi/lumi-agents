//! Explicit workspace roots (spec 08 §8.2).
//!
//! A workspace binds one task to one local root with owner, retention,
//! and cleanup metadata persisted inside the workspace. Everything a task
//! does with local files happens under its root; broader access requires
//! an explicit policy grant (out of workspace scope).

use lumi_protocol::{ProjectId, TaskId, Timestamp, WorkspaceKind};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// What happens to the workspace at end of life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupPolicy {
    /// Delete contents when the task reaches a terminal state.
    DeleteOnCompletion,
    /// Retain until the given many days after creation, then purge.
    RetainDays(u32),
    /// Keep until a human deletes it.
    Manual,
}

/// Persisted workspace metadata (§8.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    pub task_id: TaskId,
    /// Absolute canonical root path.
    pub root: PathBuf,
    /// Owning principal id.
    pub owner: String,
    pub created_at: Timestamp,
    pub cleanup: CleanupPolicy,
    /// Layout version for forward-compatible metadata handling.
    pub layout_version: u32,
    /// Project this workspace belongs to when project-bound (spec 08 §8.2).
    /// `None` for legacy task-depot workspaces, which are not project scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// Workspace kind when project-bound. `None` is a legacy task-depot
    /// workspace; resume must never reinterpret it as project scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_kind: Option<WorkspaceKind>,
}

/// A workspace bound to one task.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub metadata: WorkspaceMetadata,
}

impl Workspace {
    /// Creates (or re-opens) the workspace for `task_id` under `depot`.
    ///
    /// The root is `<depot>/<task_id>`; re-opening an existing workspace
    /// returns the same root and refreshes nothing (idempotent).
    ///
    /// # Errors
    /// Filesystem errors surface as strings; the caller maps them onto
    /// the failure taxonomy.
    pub fn create(
        depot: &Path,
        task_id: &TaskId,
        owner: impl Into<String>,
        cleanup: CleanupPolicy,
    ) -> Result<Self, String> {
        let root = depot.join(task_id.as_str());
        std::fs::create_dir_all(&root)
            .map_err(|e| format!("creating workspace {}: {e}", root.display()))?;
        let metadata = WorkspaceMetadata {
            task_id: task_id.clone(),
            root: Self::absolutize(&root)?,
            owner: owner.into(),
            created_at: Timestamp::now(),
            cleanup,
            layout_version: 1,
            project_id: None,
            workspace_kind: None,
        };
        let workspace = Self { metadata };
        workspace.persist_metadata()?;
        Ok(workspace)
    }

    /// Loads an existing workspace's metadata from its root.
    ///
    /// # Errors
    /// Missing or corrupt metadata fails closed.
    pub fn open(root: &Path) -> Result<Self, String> {
        let meta_path = root.join(Self::METADATA_FILE);
        let text = std::fs::read_to_string(&meta_path)
            .map_err(|e| format!("reading workspace metadata {}: {e}", meta_path.display()))?;
        let metadata: WorkspaceMetadata =
            serde_json::from_str(&text).map_err(|e| format!("corrupt workspace metadata: {e}"))?;
        if metadata.root != Self::absolutize(root)? {
            return Err("workspace metadata root does not match location".to_owned());
        }
        Ok(Self { metadata })
    }

    /// Binds work-mode execution to an EXISTING directory (typically a
    /// user project root) without writing metadata into it.
    ///
    /// Unlike [`Workspace::create`]/[`Workspace::open`], the binding is
    /// in-memory only: the user's repository stays untouched. All path
    /// containment checks still apply, and `root` must exist.
    ///
    /// # Errors
    /// The root does not exist or cannot be canonicalized.
    pub fn attach(
        root: &Path,
        task_id: &TaskId,
        owner: impl Into<String>,
        project_id: Option<ProjectId>,
        workspace_kind: Option<WorkspaceKind>,
    ) -> Result<Self, String> {
        let abs = Self::absolutize(root)?;
        if !abs.is_dir() {
            return Err(format!(
                "cannot attach workspace: {} is not a directory",
                abs.display()
            ));
        }
        Ok(Self {
            metadata: WorkspaceMetadata {
                task_id: task_id.clone(),
                root: abs,
                owner: owner.into(),
                created_at: Timestamp::now(),
                cleanup: CleanupPolicy::Manual,
                layout_version: 1,
                project_id,
                workspace_kind,
            },
        })
    }

    pub(crate) const METADATA_FILE: &'static str = ".lumi-workspace.json";

    fn persist_metadata(&self) -> Result<(), String> {
        let path = self.metadata.root.join(Self::METADATA_FILE);
        let text = serde_json::to_string_pretty(&self.metadata)
            .map_err(|e| format!("serializing workspace metadata: {e}"))?;
        std::fs::write(&path, text).map_err(|e| format!("writing workspace metadata: {e}"))
    }

    fn absolutize(path: &Path) -> Result<PathBuf, String> {
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .map_err(|e| format!("resolving cwd: {e}"))
        }
    }

    /// The absolute root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.metadata.root
    }

    /// Workspace-local trash directory for reversible deletes (§8.5).
    #[must_use]
    pub fn trash_dir(&self) -> PathBuf {
        self.metadata.root.join(".lumi-trash")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_depot() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-ws-{}-{}-{}",
            std::process::id(),
            format!("{:?}", std::thread::current().id())
                .replace("ThreadId(", "")
                .replace(")", ""),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn create_persists_and_reload_matches() {
        let depot = unique_depot();
        let task = TaskId::parse("task-ws-meta").unwrap();
        let ws = Workspace::create(&depot, &task, "u-owner", CleanupPolicy::RetainDays(7)).unwrap();
        let reloaded = Workspace::open(ws.root()).unwrap();
        assert_eq!(reloaded.metadata.task_id, task);
        assert_eq!(reloaded.metadata.owner, "u-owner");
        assert_eq!(reloaded.metadata.cleanup, CleanupPolicy::RetainDays(7));
        assert!(ws.root().join(Workspace::METADATA_FILE).is_file());
        std::fs::remove_dir_all(&depot).ok();
    }

    #[test]
    fn open_without_metadata_fails_closed() {
        let depot = unique_depot();
        let empty = depot.join("task-empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(Workspace::open(&empty).is_err());
        std::fs::remove_dir_all(&depot).ok();
    }

    #[test]
    fn trash_dir_is_inside_root() {
        let depot = unique_depot();
        let ws = Workspace::create(
            &depot,
            &TaskId::parse("task-ws-trash").unwrap(),
            "u",
            CleanupPolicy::Manual,
        )
        .unwrap();
        assert!(ws.trash_dir().starts_with(ws.root()));
        std::fs::remove_dir_all(&depot).ok();
    }
}
