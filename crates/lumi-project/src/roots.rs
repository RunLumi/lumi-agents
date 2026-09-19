//! Project-root filesystem authority (spec 26 §26.5, §26.26).
//!
//! The authorized roots are the default boundary for file reads/writes,
//! search, shell cwd, artifacts, instruction discovery, and Git. A
//! request resolves against the root that contains it; containment and
//! symlink/junction escape checks are delegated to the spec 08 resolver
//! so project scope inherits the same fail-closed guarantees.

use crate::error::ProjectError;
use crate::record::{AuthorizedRoot, ProjectRecord};
use lumi_workspaces::resolve_in_workspace;
use std::path::Path;

/// A request resolved into one authorized root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInProject {
    /// The root that contained the request.
    pub root: AuthorizedRoot,
    /// Safe absolute path (canonicalized, escape-checked).
    pub resolved: std::path::PathBuf,
}

/// Resolves `requested` against the project's authorized roots.
///
/// Absolute requests are re-anchored under their containing root (never
/// honored as-is), `..` traversal above a root is refused, and symlinks
/// must stay inside the containing root. Missing roots fail closed.
///
/// # Errors
/// - [`ProjectError::RootMissing`] when a containing root is gone;
/// - [`ProjectError::OutsideProjectRoots`] when no root can contain the
///   request (traversal, symlink escape, or refusal by every root).
pub fn resolve_in_project(
    project: &ProjectRecord,
    requested: &Path,
) -> Result<ResolvedInProject, ProjectError> {
    let mut last_escape: Option<lumi_workspaces::PathError> = None;
    // Existence-priority routing: an existing file resolves in the root
    // that actually contains it; a path that exists nowhere (a create
    // target) falls back to the first root so behavior is deterministic.
    let mut fallback: Option<ResolvedInProject> = None;
    for root in &project.authorized_roots {
        if !root.path.exists() {
            return Err(ProjectError::RootMissing {
                path: root.path.display().to_string(),
            });
        }
        match resolve_in_workspace(&root.path, requested) {
            Ok(resolved) => {
                if resolved.exists() {
                    return Ok(ResolvedInProject {
                        root: root.clone(),
                        resolved,
                    });
                }
                if fallback.is_none() {
                    fallback = Some(ResolvedInProject {
                        root: root.clone(),
                        resolved,
                    });
                }
            }
            Err(e @ lumi_workspaces::PathError::Io(_)) => {
                return Err(ProjectError::Io(e.to_string()));
            }
            Err(e) => {
                last_escape = Some(e);
            }
        }
    }
    match fallback {
        Some(resolved) => Ok(resolved),
        None => Err(ProjectError::OutsideProjectRoots {
            requested: match last_escape {
                Some(escape) => format!("{} ({escape})", requested.display()),
                None => requested.display().to_string(),
            },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{DetectedSource, IndexingState, RootAccess};
    use lumi_protocol::ids::{EnvironmentId, PrincipalId, ProjectId};
    use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
    use lumi_protocol::{TenantId, Timestamp};
    use std::path::PathBuf;

    fn principal() -> Principal {
        Principal {
            principal_id: PrincipalId::parse("u-owner").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            kind: PrincipalKind::User,
            authenticated_at: Some(Timestamp::from_epoch(0, 0).unwrap()),
            authentication_strength: Some(AuthenticationStrength::DevicePossession),
        }
    }

    fn unique_dir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-project-roots-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }

    fn project_with_roots(roots: Vec<AuthorizedRoot>) -> ProjectRecord {
        let now = Timestamp::from_epoch(0, 0).unwrap();
        let primary = roots[0].path.clone();
        let record = ProjectRecord {
            project_id: ProjectId::parse("p-roots").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            owner: principal(),
            execution_environment_id: EnvironmentId::parse("env-1").unwrap(),
            display_name: "roots-demo".to_owned(),
            primary_root: primary,
            authorized_roots: roots,
            created_at: now,
            last_opened_at: now,
            detected_source: DetectedSource::Generic,
            capability_snapshot: vec![],
            policy_reference: None,
            instruction_sources: vec![],
            indexing_state: IndexingState::NotIndexed,
            git_metadata: None,
        };
        record.validate().unwrap();
        record
    }

    fn read_write(root_id: &str, path: &Path) -> AuthorizedRoot {
        AuthorizedRoot {
            root_id: root_id.to_owned(),
            path: path.to_path_buf(),
            access: RootAccess::ReadWrite,
        }
    }

    #[test]
    fn relative_request_resolves_into_primary_root() {
        let dir = unique_dir("single");
        std::fs::write(dir.join("file.txt"), b"x").unwrap();
        let project = project_with_roots(vec![read_write("primary", &dir)]);
        let resolved = resolve_in_project(&project, Path::new("file.txt")).unwrap();
        assert_eq!(resolved.root.root_id, "primary");
        assert_eq!(resolved.resolved, dir.join("file.txt"));
        assert!(resolved.resolved.starts_with(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn traversal_above_root_is_refused() {
        let dir = unique_dir("traversal");
        let project = project_with_roots(vec![read_write("primary", &dir)]);
        let err = resolve_in_project(&project, Path::new("../../secrets")).unwrap_err();
        assert!(
            matches!(err, ProjectError::OutsideProjectRoots { .. }),
            "{err}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_refused() {
        let dir = unique_dir("symlink");
        let outside = unique_dir("outside");
        std::os::unix::fs::symlink(&outside, dir.join("escape")).unwrap();
        let project = project_with_roots(vec![read_write("primary", &dir)]);
        let err = resolve_in_project(&project, Path::new("escape/file")).unwrap_err();
        assert!(
            matches!(err, ProjectError::OutsideProjectRoots { .. }),
            "{err}"
        );
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&outside).ok();
    }

    #[test]
    fn missing_root_fails_closed() {
        let dir = unique_dir("missing");
        let project = project_with_roots(vec![read_write("primary", &dir)]);
        std::fs::remove_dir_all(&dir).unwrap();
        let err = resolve_in_project(&project, Path::new("file.txt")).unwrap_err();
        assert!(matches!(err, ProjectError::RootMissing { .. }), "{err}");
    }

    #[test]
    fn multi_root_resolution_uses_the_containing_root() {
        let main = unique_dir("multi-main");
        let docs = unique_dir("multi-docs");
        std::fs::write(docs.join("spec.md"), b"spec").unwrap();
        let project = project_with_roots(vec![
            read_write("primary", &main),
            AuthorizedRoot {
                root_id: "docs".to_owned(),
                path: docs.clone(),
                access: RootAccess::ReadOnly,
            },
        ]);
        let resolved = resolve_in_project(&project, Path::new("spec.md")).unwrap();
        assert_eq!(resolved.root.root_id, "docs");
        assert_eq!(resolved.root.access, RootAccess::ReadOnly);
        assert!(resolved.resolved.starts_with(&docs));
        std::fs::remove_dir_all(&main).ok();
        std::fs::remove_dir_all(&docs).ok();
    }

    #[test]
    fn nonexistent_create_target_falls_back_to_primary_root() {
        let main = unique_dir("fallback-main");
        let docs = unique_dir("fallback-docs");
        let project = project_with_roots(vec![
            read_write("primary", &main),
            AuthorizedRoot {
                root_id: "docs".to_owned(),
                path: docs.clone(),
                access: RootAccess::ReadWrite,
            },
        ]);
        let resolved = resolve_in_project(&project, Path::new("new-file.txt")).unwrap();
        assert_eq!(resolved.root.root_id, "primary");
        assert_eq!(resolved.resolved, main.join("new-file.txt"));
        std::fs::remove_dir_all(&main).ok();
        std::fs::remove_dir_all(&docs).ok();
    }
}

/// Resume-target guard (spec 26 §26.28, §26.34 "resume against wrong
/// Project"): a durable task binding must name EXACTLY the project and
/// environment the resume request targets. A mismatch is a fail-closed
/// refusal — the task's authority never travels to another project, and
/// a remote/client-supplied project id can never substitute for the
/// binding recorded at task creation.
///
/// # Errors
/// [`crate::ProjectError::InvalidRecord`] on any mismatch.
pub fn assert_resume_target(
    binding: &lumi_protocol::ProjectTaskBinding,
    project_id: &lumi_protocol::ProjectId,
    environment_id: &lumi_protocol::EnvironmentId,
) -> Result<(), crate::ProjectError> {
    if binding.project_id != *project_id {
        return Err(crate::ProjectError::InvalidRecord(format!(
            "task is bound to project {}, not {}",
            binding.project_id, project_id
        )));
    }
    if binding.execution_environment_id != *environment_id {
        return Err(crate::ProjectError::InvalidRecord(format!(
            "task is bound to environment {}, not {}",
            binding.execution_environment_id, environment_id
        )));
    }
    Ok(())
}
