//! Workspace-confined file operations (spec 08 §8.3–8.5).
//!
//! Operations are the canonical file capabilities (read / create / edit /
//! move / delete / export / share). Every path passes through
//! [`resolve_in_workspace`] before touching the filesystem. Deletes are
//! reversible by default (moved into the workspace trash); permanent
//! deletion is the caller's explicit choice and maps to DESTRUCTIVE risk
//! at the policy layer. Every operation returns a [`FileOpRecord`] with
//! before/after hashes for evidence.

use crate::paths::{resolve_in_workspace, PathError};
use crate::workspace::Workspace;
use lumi_protocol::canonical::sha256_hex;
use std::path::{Path, PathBuf};

/// The canonical file operation requested (spec 08 §8.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOp {
    /// files.read — returns content + checksum.
    Read { path: PathBuf },
    /// files.create — fails if the file exists unless `allow_overwrite`
    /// is set, which callers gate behind policy (§8.14 overwrite approval).
    Create {
        path: PathBuf,
        content: Vec<u8>,
        allow_overwrite: bool,
    },
    /// files.edit — overwrite an existing file (same approval semantics).
    Edit { path: PathBuf, content: Vec<u8> },
    /// files.move — rename within the workspace.
    Move { from: PathBuf, to: PathBuf },
    /// files.delete — reversible by default; `permanent: true` is
    /// DESTRUCTIVE and must be policy-gated upstream.
    Delete { path: PathBuf, permanent: bool },
    /// files.export / files.share — copy to a caller-authorized
    /// destination outside the workspace (the destination must already
    /// exist and be policy-approved; the workspace does not decide it).
    Export { path: PathBuf, destination: PathBuf },
}

/// Record of one executed file operation (evidence payload).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileOpRecord {
    pub capability: &'static str,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256_after: Option<String>,
    /// Whether the operation can be undone (trash path for deletes).
    pub reversible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undo_path: Option<String>,
}

/// File operation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOpError {
    /// Path refused by the safety resolver.
    UnsafePath(PathError),
    /// Create/Edit refused because the file exists and overwrite was not
    /// authorized (§8.14).
    OverwriteNotAuthorized { path: String },
    /// Read/Delete/Move/Export target does not exist.
    NotFound { path: String },
    /// Filesystem failure.
    Io(String),
}

impl From<PathError> for FileOpError {
    fn from(e: PathError) -> Self {
        Self::UnsafePath(e)
    }
}

impl std::fmt::Display for FileOpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsafePath(e) => write!(f, "unsafe path: {e}"),
            Self::OverwriteNotAuthorized { path } => {
                write!(f, "overwrite not authorized for {path:?}")
            }
            Self::NotFound { path } => write!(f, "not found: {path:?}"),
            Self::Io(e) => write!(f, "file operation failed: {e}"),
        }
    }
}

impl std::error::Error for FileOpError {}

/// Executes file operations confined to one workspace.
pub struct WorkspaceFiles<'a> {
    pub workspace: &'a Workspace,
}

impl<'a> WorkspaceFiles<'a> {
    #[must_use]
    pub const fn new(workspace: &'a Workspace) -> Self {
        Self { workspace }
    }

    fn resolve(&self, path: &Path) -> Result<PathBuf, FileOpError> {
        resolve_in_workspace(self.workspace.root(), path).map_err(FileOpError::UnsafePath)
    }

    fn hash(path: &Path) -> Option<String> {
        std::fs::read(path).ok().map(|bytes| sha256_hex(&bytes))
    }

    /// Executes one operation.
    ///
    /// # Errors
    /// [`FileOpError`] on unsafe paths, unauthorized overwrites, missing
    /// targets, or IO failures.
    pub fn execute(&self, op: &FileOp) -> Result<FileOpRecord, FileOpError> {
        match op {
            FileOp::Read { path } => {
                let resolved = self.resolve(path)?;
                let content = std::fs::read(&resolved).map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        FileOpError::NotFound {
                            path: path.display().to_string(),
                        }
                    } else {
                        FileOpError::Io(e.to_string())
                    }
                })?;
                Ok(FileOpRecord {
                    capability: "files.read",
                    path: path.display().to_string(),
                    sha256_before: Some(sha256_hex(&content)),
                    sha256_after: None,
                    reversible: true,
                    undo_path: None,
                })
            }
            FileOp::Create {
                path,
                content,
                allow_overwrite,
            } => {
                let resolved = self.resolve(path)?;
                let existed = resolved.exists();
                if existed && !allow_overwrite {
                    return Err(FileOpError::OverwriteNotAuthorized {
                        path: path.display().to_string(),
                    });
                }
                let before = Self::hash(&resolved);
                if let Some(parent) = resolved.parent() {
                    // Safe: the full path was already validated in-root.
                    std::fs::create_dir_all(parent).map_err(|e| FileOpError::Io(e.to_string()))?;
                }
                std::fs::write(&resolved, content).map_err(|e| FileOpError::Io(e.to_string()))?;
                Ok(FileOpRecord {
                    capability: if existed {
                        "files.edit"
                    } else {
                        "files.create"
                    },
                    path: path.display().to_string(),
                    sha256_before: before,
                    sha256_after: Some(sha256_hex(content)),
                    reversible: existed,
                    undo_path: None,
                })
            }
            FileOp::Edit { path, content } => {
                let resolved = self.resolve(path)?;
                if !resolved.exists() {
                    return Err(FileOpError::NotFound {
                        path: path.display().to_string(),
                    });
                }
                let before = Self::hash(&resolved);
                std::fs::write(&resolved, content).map_err(|e| FileOpError::Io(e.to_string()))?;
                Ok(FileOpRecord {
                    capability: "files.edit",
                    path: path.display().to_string(),
                    sha256_before: before,
                    sha256_after: Some(sha256_hex(content)),
                    reversible: false,
                    undo_path: None,
                })
            }
            FileOp::Move { from, to } => {
                let resolved_from = self.resolve(from)?;
                let resolved_to = self.resolve(to)?;
                if !resolved_from.exists() {
                    return Err(FileOpError::NotFound {
                        path: from.display().to_string(),
                    });
                }
                std::fs::rename(&resolved_from, &resolved_to)
                    .map_err(|e| FileOpError::Io(e.to_string()))?;
                Ok(FileOpRecord {
                    capability: "files.move",
                    path: from.display().to_string(),
                    sha256_before: None,
                    sha256_after: Self::hash(&resolved_to),
                    reversible: true,
                    undo_path: Some(resolved_from.display().to_string()),
                })
            }
            FileOp::Delete { path, permanent } => {
                let resolved = self.resolve(path)?;
                if !resolved.exists() {
                    return Err(FileOpError::NotFound {
                        path: path.display().to_string(),
                    });
                }
                let before = Self::hash(&resolved);
                if *permanent {
                    // DESTRUCTIVE: policy must have approved upstream.
                    std::fs::remove_file(&resolved).map_err(|e| FileOpError::Io(e.to_string()))?;
                    Ok(FileOpRecord {
                        capability: "files.delete",
                        path: path.display().to_string(),
                        sha256_before: before,
                        sha256_after: None,
                        reversible: false,
                        undo_path: None,
                    })
                } else {
                    // Reversible: move into the workspace trash.
                    let trash = self.workspace.trash_dir();
                    std::fs::create_dir_all(&trash).map_err(|e| FileOpError::Io(e.to_string()))?;
                    let unique = format!(
                        "{}-{}",
                        Timestamp::now().nanoseconds(),
                        resolved.file_name().unwrap_or_default().to_string_lossy()
                    );
                    let trash_path = trash.join(unique);
                    std::fs::rename(&resolved, &trash_path)
                        .map_err(|e| FileOpError::Io(e.to_string()))?;
                    Ok(FileOpRecord {
                        capability: "files.delete",
                        path: path.display().to_string(),
                        sha256_before: before,
                        sha256_after: None,
                        reversible: true,
                        undo_path: Some(trash_path.display().to_string()),
                    })
                }
            }
            FileOp::Export { path, destination } => {
                let resolved = self.resolve(path)?;
                if !resolved.exists() {
                    return Err(FileOpError::NotFound {
                        path: path.display().to_string(),
                    });
                }
                // The destination is a policy-approved external target;
                // create its parent if needed and copy.
                if let Some(parent) = destination.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| FileOpError::Io(e.to_string()))?;
                }
                std::fs::copy(&resolved, destination)
                    .map_err(|e| FileOpError::Io(e.to_string()))?;
                Ok(FileOpRecord {
                    capability: "files.export",
                    path: path.display().to_string(),
                    sha256_before: Self::hash(&resolved),
                    sha256_after: Self::hash(destination),
                    reversible: true,
                    undo_path: Some(destination.display().to_string()),
                })
            }
        }
    }

    /// Restores a reversible delete from its trash path (§8.5 undo).
    ///
    /// # Errors
    /// IO failure or missing trash file.
    pub fn restore(&self, record: &FileOpRecord) -> Result<(), FileOpError> {
        let Some(undo) = &record.undo_path else {
            return Err(FileOpError::Io("operation is not reversible".to_owned()));
        };
        let resolved = self.resolve(Path::new(&record.path))?;
        std::fs::rename(Path::new(undo), &resolved).map_err(|e| FileOpError::Io(e.to_string()))
    }
}

use lumi_protocol::Timestamp;
