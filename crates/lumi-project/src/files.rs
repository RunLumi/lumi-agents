//! Project-confined file operations (spec 26 §26.9–26.10, §26.13,
//! §26.24).
//!
//! Every path resolves through [`resolve_in_project`] (fail-closed on
//! traversal/symlink escape; unauthorized roots never consulted).
//! Mutations on existing files REQUIRE the caller's expected checksum:
//! a mismatch is a [`FileOpsError::StaleWrite`] conflict that surfaces
//! rather than clobbers (§26.9, §26.24). Deletion is reversible by
//! default (project trash under `.lumi/trash/`); permanent deletion is
//! an explicit DESTRUCTIVE choice for policy to gate upstream. Reads are
//! bounded by size; binary detection prevents model-unsafe content from
//! flowing through read/search (§26.13).

use crate::changeset::{checksum, text_patch};
use crate::error::ProjectError;
use crate::record::{ProjectRecord, RootAccess};
use crate::roots::resolve_in_project;
use lumi_protocol::canonical::sha256_hex;
use lumi_protocol::Timestamp;
use std::path::{Path, PathBuf};

/// Default read bound (bytes). Callers may pass a tighter bound; a
/// larger one is refused so the default stays an actual ceiling.
pub const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;

/// Patch capture cap for a single modification entry (§26.31).
pub const MAX_PATCH_BYTES: usize = 256 * 1024;

/// Why a file operation was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOpsError {
    /// Path refused (traversal, symlink escape, or outside every root).
    UnsafePath(ProjectError),
    /// The containing root is read-only.
    ReadOnlyRoot {
        path: String,
    },
    /// Create target already exists (overwrite requires an expected
    /// checksum edit, never a blind create).
    AlreadyExists {
        path: String,
    },
    /// Target missing.
    NotFound {
        path: String,
    },
    /// Expected checksum does not match current content: an external
    /// editor changed the file. The prepared change is NOT applied.
    StaleWrite {
        path: String,
        expected: String,
        actual: String,
    },
    /// Read exceeds the size bound.
    FileTooLarge {
        path: String,
        size: u64,
        max: u64,
    },
    /// Content looks binary where text is required.
    BinaryContent {
        path: String,
    },
    Io(String),
}

impl std::fmt::Display for FileOpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsafePath(e) => write!(f, "unsafe path: {e}"),
            Self::ReadOnlyRoot { path } => write!(f, "root is read-only: {path}"),
            Self::AlreadyExists { path } => write!(f, "already exists: {path}"),
            Self::NotFound { path } => write!(f, "not found: {path}"),
            Self::StaleWrite {
                path,
                expected,
                actual,
            } => write!(
                f,
                "stale write for {path}: prepared against {expected}, current is {actual}"
            ),
            Self::FileTooLarge { path, size, max } => {
                write!(
                    f,
                    "file {path} is {size} bytes, over the {max} byte read bound"
                )
            }
            Self::BinaryContent { path } => write!(f, "binary content: {path}"),
            Self::Io(e) => write!(f, "file operation failed: {e}"),
        }
    }
}

impl std::error::Error for FileOpsError {}

impl From<ProjectError> for FileOpsError {
    fn from(e: ProjectError) -> Self {
        Self::UnsafePath(e)
    }
}

/// Outcome of one executed mutation, ready to record in a change set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationOutcome {
    /// Project-relative path (destination for moves).
    pub path: String,
    /// Project-relative source path for moves.
    pub from_path: Option<String>,
    pub sha256_before: Option<String>,
    pub sha256_after: Option<String>,
    /// Captured patch for text modifications (may be `None` for
    /// binary/oversized content).
    pub patch: Option<String>,
    /// Trash location for reversible deletes.
    pub undo_path: Option<PathBuf>,
}

/// Project file operations for one project record.
pub struct ProjectFiles<'a> {
    project: &'a ProjectRecord,
}

impl<'a> ProjectFiles<'a> {
    /// The project record these operations are bounded by.
    #[must_use]
    pub const fn project(&self) -> &'a ProjectRecord {
        self.project
    }
}

impl<'a> ProjectFiles<'a> {
    #[must_use]
    pub const fn new(project: &'a ProjectRecord) -> Self {
        Self { project }
    }

    fn resolve(&self, path: &Path) -> Result<PathBuf, FileOpsError> {
        Ok(resolve_in_project(self.project, path)?.resolved)
    }

    /// Resolves AND requires write authority on the containing root.
    fn resolve_writable(&self, path: &Path) -> Result<PathBuf, FileOpsError> {
        let resolved = resolve_in_project(self.project, path)?;
        if resolved.root.access != RootAccess::ReadWrite {
            return Err(FileOpsError::ReadOnlyRoot {
                path: resolved.root.path.display().to_string(),
            });
        }
        Ok(resolved.resolved)
    }

    fn current_hash(path: &Path) -> Option<String> {
        std::fs::read(path).ok().map(|bytes| sha256_hex(&bytes))
    }

    /// Reads a file with a size bound (§26.13). Text detection is the
    /// caller's concern; this refuses only oversized reads.
    pub fn read(&self, path: &Path, max_bytes: u64) -> Result<Vec<u8>, FileOpsError> {
        let resolved = self.resolve(path)?;
        let max = max_bytes.min(MAX_READ_BYTES);
        let meta = std::fs::metadata(&resolved).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileOpsError::NotFound {
                    path: path.display().to_string(),
                }
            } else {
                FileOpsError::Io(e.to_string())
            }
        })?;
        if meta.len() > max {
            return Err(FileOpsError::FileTooLarge {
                path: path.display().to_string(),
                size: meta.len(),
                max,
            });
        }
        std::fs::read(&resolved).map_err(|e| FileOpsError::Io(e.to_string()))
    }

    /// Reads a file as UTF-8 text, refusing binary content.
    pub fn read_text(&self, path: &Path, max_bytes: u64) -> Result<String, FileOpsError> {
        let bytes = self.read(path, max_bytes)?;
        if bytes.get(..8_192).unwrap_or(&bytes).contains(&0) {
            return Err(FileOpsError::BinaryContent {
                path: path.display().to_string(),
            });
        }
        String::from_utf8(bytes).map_err(|_| FileOpsError::BinaryContent {
            path: path.display().to_string(),
        })
    }

    /// Creates a file, refusing to overwrite (§26.9: no silent
    /// overwrite; replacing an existing file is a checksum-guarded edit).
    pub fn create_file(
        &self,
        path: &Path,
        content: &[u8],
    ) -> Result<MutationOutcome, FileOpsError> {
        let resolved = self.resolve_writable(path)?;
        if resolved.exists() {
            return Err(FileOpsError::AlreadyExists {
                path: path.display().to_string(),
            });
        }
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent).map_err(|e| FileOpsError::Io(e.to_string()))?;
        }
        std::fs::write(&resolved, content).map_err(|e| FileOpsError::Io(e.to_string()))?;
        Ok(MutationOutcome {
            path: path.display().to_string(),
            from_path: None,
            sha256_before: None,
            sha256_after: Some(checksum(content)),
            patch: None,
            undo_path: None,
        })
    }

    /// Creates a directory (parents included).
    pub fn create_dir(&self, path: &Path) -> Result<(), FileOpsError> {
        let resolved = self.resolve_writable(path)?;
        std::fs::create_dir_all(&resolved).map_err(|e| FileOpsError::Io(e.to_string()))
    }

    /// Writes `content` to an EXISTING file, guarded by the checksum the
    /// change was prepared against (§26.24). A mismatch refuses with
    /// [`FileOpsError::StaleWrite`] and preserves the external version.
    pub fn edit_file(
        &self,
        path: &Path,
        expected_sha256: &str,
        content: &[u8],
    ) -> Result<MutationOutcome, FileOpsError> {
        let resolved = self.resolve_writable(path)?;
        if !resolved.is_file() {
            return Err(FileOpsError::NotFound {
                path: path.display().to_string(),
            });
        }
        let before_bytes = std::fs::read(&resolved).map_err(|e| FileOpsError::Io(e.to_string()))?;
        let actual = sha256_hex(&before_bytes);
        if actual != expected_sha256 {
            return Err(FileOpsError::StaleWrite {
                path: path.display().to_string(),
                expected: expected_sha256.to_owned(),
                actual,
            });
        }
        std::fs::write(&resolved, content).map_err(|e| FileOpsError::Io(e.to_string()))?;
        let patch = text_patch(
            &before_bytes,
            content,
            &path.display().to_string(),
            MAX_PATCH_BYTES,
        );
        Ok(MutationOutcome {
            path: path.display().to_string(),
            from_path: None,
            sha256_before: Some(actual),
            sha256_after: Some(checksum(content)),
            patch,
            undo_path: None,
        })
    }

    /// Moves/renames within project scope, checksum-guarded for files.
    pub fn move_path(
        &self,
        from: &Path,
        to: &Path,
        expected_sha256: Option<&str>,
    ) -> Result<MutationOutcome, FileOpsError> {
        let resolved_from = self.resolve_writable(from)?;
        let resolved_to = self.resolve_writable(to)?;
        if !resolved_from.exists() {
            return Err(FileOpsError::NotFound {
                path: from.display().to_string(),
            });
        }
        if resolved_to.exists() {
            return Err(FileOpsError::AlreadyExists {
                path: to.display().to_string(),
            });
        }
        let sha_before = if resolved_from.is_file() {
            let bytes =
                std::fs::read(&resolved_from).map_err(|e| FileOpsError::Io(e.to_string()))?;
            let hash = sha256_hex(&bytes);
            if let Some(expected) = expected_sha256 {
                if hash != expected {
                    return Err(FileOpsError::StaleWrite {
                        path: from.display().to_string(),
                        expected: expected.to_owned(),
                        actual: hash,
                    });
                }
            }
            Some(hash)
        } else {
            None
        };
        if let Some(parent) = resolved_to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| FileOpsError::Io(e.to_string()))?;
        }
        std::fs::rename(&resolved_from, &resolved_to)
            .map_err(|e| FileOpsError::Io(e.to_string()))?;
        Ok(MutationOutcome {
            path: to.display().to_string(),
            from_path: Some(from.display().to_string()),
            sha256_before: sha_before,
            sha256_after: Self::current_hash(&resolved_to),
            patch: None,
            undo_path: Some(resolved_from),
        })
    }

    /// Copies a file within project scope.
    pub fn copy_file(&self, from: &Path, to: &Path) -> Result<MutationOutcome, FileOpsError> {
        let resolved_from = self.resolve(from)?;
        let resolved_to = self.resolve_writable(to)?;
        if !resolved_from.is_file() {
            return Err(FileOpsError::NotFound {
                path: from.display().to_string(),
            });
        }
        if resolved_to.exists() {
            return Err(FileOpsError::AlreadyExists {
                path: to.display().to_string(),
            });
        }
        if let Some(parent) = resolved_to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| FileOpsError::Io(e.to_string()))?;
        }
        let before = Self::current_hash(&resolved_from);
        std::fs::copy(&resolved_from, &resolved_to).map_err(|e| FileOpsError::Io(e.to_string()))?;
        Ok(MutationOutcome {
            path: to.display().to_string(),
            from_path: None,
            sha256_before: before,
            sha256_after: Self::current_hash(&resolved_to),
            patch: None,
            undo_path: Some(resolved_to.clone()),
        })
    }

    /// Deletes a file. Reversible by default: the file moves into the
    /// project trash and [`MutationOutcome::undo_path`] records where.
    /// `permanent` is the DESTRUCTIVE path — upstream policy must have
    /// authorized it (§26.10).
    pub fn delete_file(
        &self,
        path: &Path,
        expected_sha256: Option<&str>,
        permanent: bool,
    ) -> Result<MutationOutcome, FileOpsError> {
        let resolved = self.resolve_writable(path)?;
        if !resolved.is_file() {
            return Err(FileOpsError::NotFound {
                path: path.display().to_string(),
            });
        }
        let bytes = std::fs::read(&resolved).map_err(|e| FileOpsError::Io(e.to_string()))?;
        let hash = sha256_hex(&bytes);
        if let Some(expected) = expected_sha256 {
            if hash != expected {
                return Err(FileOpsError::StaleWrite {
                    path: path.display().to_string(),
                    expected: expected.to_owned(),
                    actual: hash,
                });
            }
        }
        if permanent {
            std::fs::remove_file(&resolved).map_err(|e| FileOpsError::Io(e.to_string()))?;
            return Ok(MutationOutcome {
                path: path.display().to_string(),
                from_path: None,
                sha256_before: Some(hash),
                sha256_after: None,
                patch: None,
                undo_path: None,
            });
        }
        let trash = self.project.primary_root.join(".lumi").join("trash");
        std::fs::create_dir_all(&trash).map_err(|e| FileOpsError::Io(e.to_string()))?;
        let unique = format!(
            "{}-{}",
            Timestamp::now().nanoseconds(),
            resolved.file_name().unwrap_or_default().to_string_lossy()
        );
        let trash_path = trash.join(unique);
        std::fs::rename(&resolved, &trash_path).map_err(|e| FileOpsError::Io(e.to_string()))?;
        Ok(MutationOutcome {
            path: path.display().to_string(),
            from_path: None,
            sha256_before: Some(hash),
            sha256_after: None,
            patch: None,
            undo_path: Some(trash_path),
        })
    }

    /// Restores a reversible delete (§26.9 restore).
    pub fn restore(&self, outcome: &MutationOutcome) -> Result<(), FileOpsError> {
        let Some(trash_path) = &outcome.undo_path else {
            return Err(FileOpsError::Io("operation is not reversible".to_owned()));
        };
        let resolved = self.resolve_writable(Path::new(&outcome.path))?;
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent).map_err(|e| FileOpsError::Io(e.to_string()))?;
        }
        std::fs::rename(trash_path, &resolved).map_err(|e| FileOpsError::Io(e.to_string()))
    }

    /// Current checksum of a file — the bounded-revalidation primitive
    /// external-edit detection builds on (§26.24).
    pub fn checksum(&self, path: &Path) -> Result<String, FileOpsError> {
        let resolved = self.resolve(path)?;
        let bytes = std::fs::read(&resolved).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileOpsError::NotFound {
                    path: path.display().to_string(),
                }
            } else {
                FileOpsError::Io(e.to_string())
            }
        })?;
        Ok(sha256_hex(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{AuthorizedRoot, DetectedSource, IndexingState};
    use lumi_protocol::ids::{EnvironmentId, PrincipalId, ProjectId};
    use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
    use lumi_protocol::TenantId;

    fn principal() -> Principal {
        Principal {
            principal_id: PrincipalId::parse("u-1").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            kind: PrincipalKind::User,
            authenticated_at: Some(Timestamp::from_epoch(0, 0).unwrap()),
            authentication_strength: Some(AuthenticationStrength::DevicePossession),
        }
    }

    fn unique_dir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-project-files-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }

    fn project_with(root: PathBuf, access: RootAccess, root_id: &str) -> ProjectRecord {
        let now = Timestamp::from_epoch(0, 0).unwrap();
        let record = ProjectRecord {
            project_id: ProjectId::parse("p-files").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            owner: principal(),
            execution_environment_id: EnvironmentId::parse("env-1").unwrap(),
            display_name: "files".to_owned(),
            primary_root: root.clone(),
            authorized_roots: vec![AuthorizedRoot {
                root_id: root_id.to_owned(),
                path: root,
                access,
            }],
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

    #[test]
    fn create_edit_delete_cycle_records_checksums() {
        let root = unique_dir("cycle");
        let record = project_with(root.clone(), RootAccess::ReadWrite, "primary");
        let files = ProjectFiles::new(&record);

        let created = files
            .create_file(Path::new("docs/note.txt"), b"v1")
            .unwrap();
        assert_eq!(created.path, "docs/note.txt");
        assert_eq!(created.sha256_after.as_deref(), Some(&*checksum(b"v1")));

        let expected = files.checksum(Path::new("docs/note.txt")).unwrap();
        let edited = files
            .edit_file(Path::new("docs/note.txt"), &expected, b"v2 content")
            .unwrap();
        assert_eq!(edited.sha256_before.as_deref(), Some(expected.as_str()));
        assert!(edited.patch.is_some());

        let deleted = files
            .delete_file(
                Path::new("docs/note.txt"),
                Some(&edited.sha256_after.unwrap()),
                false,
            )
            .unwrap();
        assert!(deleted.undo_path.is_some());
        assert!(!root.join("docs/note.txt").exists());

        files.restore(&deleted).unwrap();
        assert_eq!(
            files.read_text(Path::new("docs/note.txt"), 1024).unwrap(),
            "v2 content"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn stale_write_is_refused_and_external_work_preserved() {
        let root = unique_dir("stale");
        let record = project_with(root.clone(), RootAccess::ReadWrite, "primary");
        let files = ProjectFiles::new(&record);
        files.create_file(Path::new("a.txt"), b"original").unwrap();
        let expected = files.checksum(Path::new("a.txt")).unwrap();

        // External editor changes the file after Lumi prepared its edit.
        std::fs::write(root.join("a.txt"), b"external edit").unwrap();

        let err = files
            .edit_file(Path::new("a.txt"), &expected, b"lumi version")
            .unwrap_err();
        let FileOpsError::StaleWrite { actual, .. } = err else {
            panic!("expected StaleWrite, got {err:?}")
        };
        assert_eq!(actual, checksum(b"external edit"));
        assert_eq!(
            std::fs::read(root.join("a.txt")).unwrap(),
            b"external edit",
            "external work must survive"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn create_refuses_existing_and_move_refuses_existing_destination() {
        let root = unique_dir("exists");
        let record = project_with(root.clone(), RootAccess::ReadWrite, "primary");
        let files = ProjectFiles::new(&record);
        files.create_file(Path::new("a.txt"), b"a").unwrap();
        files.create_file(Path::new("b.txt"), b"b").unwrap();
        assert!(matches!(
            files.create_file(Path::new("a.txt"), b"x"),
            Err(FileOpsError::AlreadyExists { .. })
        ));
        let sha = files.checksum(Path::new("a.txt")).unwrap();
        assert!(matches!(
            files.move_path(Path::new("a.txt"), Path::new("b.txt"), Some(&sha)),
            Err(FileOpsError::AlreadyExists { .. })
        ));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn read_bounds_refuse_oversized_files() {
        let root = unique_dir("big");
        let record = project_with(root.clone(), RootAccess::ReadWrite, "primary");
        let files = ProjectFiles::new(&record);
        let f = std::fs::File::create(root.join("big.bin")).unwrap();
        f.set_len(10_000).unwrap();
        drop(f);
        let err = files.read(Path::new("big.bin"), 1_000).unwrap_err();
        let FileOpsError::FileTooLarge { size, max, .. } = err else {
            panic!("expected FileTooLarge, got {err:?}")
        };
        assert_eq!((size, max), (10_000, 1_000));

        // The global ceiling applies even when the caller asks for more:
        // a sparse file over MAX_READ_BYTES is refused, never read.
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(root.join("big.bin"))
            .unwrap();
        f.set_len(MAX_READ_BYTES + 1).unwrap();
        drop(f);
        let err = files.read(Path::new("big.bin"), u64::MAX).unwrap_err();
        let FileOpsError::FileTooLarge { size, max, .. } = err else {
            panic!("expected FileTooLarge, got {err:?}")
        };
        assert_eq!((size, max), (MAX_READ_BYTES + 1, MAX_READ_BYTES));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn binary_content_is_refused_for_text_reads() {
        let root = unique_dir("binary");
        let record = project_with(root.clone(), RootAccess::ReadWrite, "primary");
        let files = ProjectFiles::new(&record);
        std::fs::write(root.join("img.bin"), [b'a', 0, b'b']).unwrap();
        let err = files.read_text(Path::new("img.bin"), 1024).unwrap_err();
        assert!(matches!(err, FileOpsError::BinaryContent { .. }), "{err:?}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn read_only_root_refuses_mutations_but_allows_reads() {
        let main = unique_dir("ro-main");
        let docs = unique_dir("ro-docs");
        std::fs::write(docs.join("spec.md"), b"spec").unwrap();
        let record = project_with(main.clone(), RootAccess::ReadWrite, "primary");
        let mut multi = record.clone();
        multi.authorized_roots.push(AuthorizedRoot {
            root_id: "docs".to_owned(),
            path: docs.clone(),
            access: RootAccess::ReadOnly,
        });
        let files = ProjectFiles::new(&multi);

        assert!(files.read_text(Path::new("spec.md"), 1024).is_ok());
        assert!(matches!(
            files.create_file(Path::new("spec.md"), b"x"),
            Err(FileOpsError::ReadOnlyRoot { .. })
        ));
        assert!(matches!(
            files.delete_file(Path::new("spec.md"), None, false),
            Err(FileOpsError::ReadOnlyRoot { .. })
        ));
        std::fs::remove_dir_all(&main).ok();
        std::fs::remove_dir_all(&docs).ok();
    }

    #[test]
    fn permanent_delete_removes_without_undo() {
        let root = unique_dir("perm");
        let record = project_with(root.clone(), RootAccess::ReadWrite, "primary");
        let files = ProjectFiles::new(&record);
        files.create_file(Path::new("gone.txt"), b"bye").unwrap();
        let sha = files.checksum(Path::new("gone.txt")).unwrap();
        let outcome = files
            .delete_file(Path::new("gone.txt"), Some(&sha), true)
            .unwrap();
        assert!(outcome.undo_path.is_none());
        assert!(!root.join("gone.txt").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn move_records_origin_and_checksum_guard() {
        let root = unique_dir("move");
        let record = project_with(root.clone(), RootAccess::ReadWrite, "primary");
        let files = ProjectFiles::new(&record);
        files.create_file(Path::new("old.txt"), b"data").unwrap();
        let sha = files.checksum(Path::new("old.txt")).unwrap();

        let outcome = files
            .move_path(Path::new("old.txt"), Path::new("new/deep.txt"), Some(&sha))
            .unwrap();
        assert_eq!(outcome.from_path.as_deref(), Some("old.txt"));
        assert!(!root.join("old.txt").exists());
        assert_eq!(
            files.read_text(Path::new("new/deep.txt"), 1024).unwrap(),
            "data"
        );

        // Wrong expected checksum refuses the move.
        files.create_file(Path::new("x.txt"), b"x").unwrap();
        assert!(matches!(
            files.move_path(Path::new("x.txt"), Path::new("y.txt"), Some("deadbeef")),
            Err(FileOpsError::StaleWrite { .. })
        ));
        std::fs::remove_dir_all(&root).ok();
    }
}
