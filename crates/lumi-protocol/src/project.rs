//! Project binding vocabulary (spec 26, spec 08 §8.2).
//!
//! A Project is a durable, policy-bounded working context rooted in one
//! ExecutionEnvironment. This module carries only the shared vocabulary:
//! identity types, the canonical task-workspace kinds, and the durable
//! task-to-project-workspace binding. Registry storage and filesystem
//! authority live in `lumi-project`; nothing here grants authority.

use crate::ids::{EnvironmentId, ProjectId};
use serde::{Deserialize, Serialize};

/// Which physical/logical workspace a task operates against
/// (spec 08 §8.2, spec 26 §26.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkspaceKind {
    /// The live project root directly.
    ProjectRoot,
    /// A subdirectory of the project root.
    ProjectSubdir,
    /// An isolated worktree/checkout linked to the project.
    IsolatedWorktree,
    /// A temporary staging directory linked to the project.
    TempStaging,
    /// A sandbox/container projection of the project.
    SandboxProjection,
}

/// Durable binding of a task to one project workspace (spec 26 §26.6).
///
/// Persisted on the task so resume restores the same project/workspace
/// relationship. A task MUST NOT silently switch workspace kind or move
/// from an isolated workspace into the live project root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTaskBinding {
    pub project_id: ProjectId,
    pub execution_environment_id: EnvironmentId,
    /// Canonical absolute path of the concrete workspace root.
    pub workspace_root: String,
    pub workspace_kind: WorkspaceKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_kind_serde_is_screaming_snake_case() {
        assert_eq!(
            serde_json::to_string(&WorkspaceKind::ProjectRoot).unwrap(),
            "\"PROJECT_ROOT\""
        );
        assert_eq!(
            serde_json::to_string(&WorkspaceKind::IsolatedWorktree).unwrap(),
            "\"ISOLATED_WORKTREE\""
        );
        let kind: WorkspaceKind = serde_json::from_str("\"SANDBOX_PROJECTION\"").unwrap();
        assert_eq!(kind, WorkspaceKind::SandboxProjection);
    }

    #[test]
    fn binding_round_trips() {
        let binding = ProjectTaskBinding {
            project_id: ProjectId::parse("proj-1").unwrap(),
            execution_environment_id: EnvironmentId::parse("env-1").unwrap(),
            workspace_root: "/tmp/demo".to_owned(),
            workspace_kind: WorkspaceKind::ProjectRoot,
        };
        let json = serde_json::to_string(&binding).unwrap();
        let back: ProjectTaskBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back, binding);
    }

    #[test]
    fn unknown_workspace_kind_fails_explicitly() {
        assert!(serde_json::from_str::<WorkspaceKind>("\"HOME_DIR\"").is_err());
    }
}
