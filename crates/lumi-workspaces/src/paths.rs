//! Path canonicalization and escape prevention (spec 08 §8.4).
//!
//! Paths are resolved against the workspace root BEFORE any other use:
//! absolute requests are re-anchored under the root (never honored as-is),
//! lexical `..` traversal above the root is refused, symlinks are resolved
//! and required to stay inside the root, and non-existent paths are
//! validated through their deepest existing ancestor. Every check fails
//! closed.

use std::path::{Component, Path, PathBuf};

/// Why a path was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// Lexical `..` traversal escapes the workspace root.
    TraversalOutsideRoot { requested: String },
    /// A symlink resolves to a location outside the root.
    SymlinkEscape { requested: String, target: String },
    /// The canonical path escapes the root for another reason.
    OutsideRoot { requested: String },
    /// Filesystem access failed while resolving.
    Io(String),
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TraversalOutsideRoot { requested } => {
                write!(f, "path traversal escapes workspace root: {requested:?}")
            }
            Self::SymlinkEscape { requested, target } => {
                write!(
                    f,
                    "symlink at {requested:?} escapes workspace root to {target:?}"
                )
            }
            Self::OutsideRoot { requested } => {
                write!(f, "canonical path escapes workspace root: {requested:?}")
            }
            Self::Io(e) => write!(f, "path resolution failed: {e}"),
        }
    }
}

impl std::error::Error for PathError {}

/// Re-anchors `requested` (absolute or relative) under `root` and returns
/// the safe absolute path.
///
/// # Errors
/// [`PathError`] for any escape or resolution failure — callers fail
/// closed.
pub fn resolve_in_workspace(root: &Path, requested: &Path) -> Result<PathBuf, PathError> {
    let requested_display = requested.display().to_string();
    let base = std::fs::canonicalize(root)
        .map_err(|e| PathError::Io(format!("canonicalizing root: {e}")))?;

    // Anchored requests are RE-ANCHORED under the root: "/etc/passwd"
    // means "<root>/etc/passwd", never the real /etc/passwd. Strip ONLY
    // the anchor components (drive prefix + leading root) — on Windows a
    // leading "/" is NOT `is_absolute()`, and `..` must survive the strip
    // so lexical traversal above the root is still refused.
    let relative: PathBuf = requested
        .components()
        .filter(|c| !matches!(c, Component::Prefix(_) | Component::RootDir))
        .collect();

    // 1. Lexical normalization: build from the canonical root, refusing
    // any `..` that would climb above it.
    let mut normalized = base.clone();
    for component in relative.components() {
        match component {
            Component::Normal(name) => normalized.push(name),
            Component::CurDir => {}
            Component::ParentDir => {
                if normalized == base {
                    return Err(PathError::TraversalOutsideRoot {
                        requested: requested_display,
                    });
                }
                if !normalized.pop() || !normalized.starts_with(&base) {
                    return Err(PathError::TraversalOutsideRoot {
                        requested: requested_display,
                    });
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                // Cannot occur: absolute requests were stripped above and
                // we always build from `base`.
                return Err(PathError::TraversalOutsideRoot {
                    requested: requested_display,
                });
            }
        }
    }

    // 2. Symlink resolution along the path: every existing prefix is
    // resolved and its target must stay inside the root (§8.4).
    let mut current = base.clone();
    let suffix: Vec<Component> = normalized
        .strip_prefix(&base)
        .map_err(|e| PathError::Io(e.to_string()))?
        .components()
        .collect();
    for component in &suffix {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let target =
                    std::fs::canonicalize(&current).map_err(|e| PathError::Io(e.to_string()))?;
                if !target.starts_with(&base) {
                    return Err(PathError::SymlinkEscape {
                        requested: requested_display,
                        target: target.display().to_string(),
                    });
                }
                // Legitimate in-root symlink: continue from its target.
                current = target;
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Deepest existing ancestor reached; the remainder cannot
                // contain symlinks yet.
                break;
            }
            Err(e) => return Err(PathError::Io(e.to_string())),
        }
    }

    // 3. Final canonical check when the full path exists.
    if normalized.exists() {
        let canonical =
            std::fs::canonicalize(&normalized).map_err(|e| PathError::Io(e.to_string()))?;
        if !canonical.starts_with(&base) {
            return Err(PathError::OutsideRoot {
                requested: requested_display,
            });
        }
        return Ok(canonical);
    }

    // Non-existent: validate the deepest existing ancestor, then return
    // the lexically-safe path (the file does not exist yet).
    let mut ancestor = normalized.clone();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    while !ancestor.exists() {
        match (ancestor.parent(), ancestor.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name.to_os_string());
                ancestor = parent.to_path_buf();
            }
            _ => break,
        }
    }
    let ancestor_canonical =
        std::fs::canonicalize(&ancestor).map_err(|e| PathError::Io(e.to_string()))?;
    if !ancestor_canonical.starts_with(&base) {
        return Err(PathError::OutsideRoot {
            requested: requested_display,
        });
    }
    let mut result = ancestor_canonical;
    for part in tail.into_iter().rev() {
        result.push(part);
    }
    Ok(result)
}

/// The canonical form of `root` for comparisons.
///
/// # Errors
/// [`PathError::Io`] when the root cannot be canonicalized.
pub fn canonical_root(root: &Path) -> Result<PathBuf, PathError> {
    std::fs::canonicalize(root).map_err(|e| PathError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn depot() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lumi-paths-{}-{}",
            std::process::id(),
            Timestamp::now().nanoseconds()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn normal_relative_paths_resolve_inside_root() {
        let root = depot();
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::write(root.join("a/b/file.txt"), b"x").unwrap();
        let resolved = resolve_in_workspace(&root, Path::new("a/b/file.txt")).unwrap();
        assert!(resolved.starts_with(std::fs::canonicalize(&root).unwrap()));
        assert_eq!(resolved.file_name().unwrap(), "file.txt");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn traversal_above_root_is_refused() {
        let root = depot();
        let err = resolve_in_workspace(&root, Path::new("../../etc/passwd")).unwrap_err();
        assert!(
            matches!(err, PathError::TraversalOutsideRoot { .. }),
            "{err:?}"
        );
        let err = resolve_in_workspace(&root, Path::new("a/../../../etc")).unwrap_err();
        assert!(matches!(err, PathError::TraversalOutsideRoot { .. }));
        let err = resolve_in_workspace(&root, Path::new("..")).unwrap_err();
        assert!(matches!(err, PathError::TraversalOutsideRoot { .. }));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn internal_traversal_that_stays_inside_is_allowed() {
        let root = depot();
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::write(root.join("a/target.txt"), b"x").unwrap();
        let resolved = resolve_in_workspace(&root, Path::new("a/b/../target.txt")).unwrap();
        assert!(resolved.ends_with("a/target.txt"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn symlink_escape_is_refused() {
        let root = depot();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc", root.join("escape")).unwrap();
            let err = resolve_in_workspace(&root, Path::new("escape/passwd")).unwrap_err();
            assert!(matches!(err, PathError::SymlinkEscape { .. }), "{err:?}");
        }
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn in_root_symlink_is_followed() {
        let root = depot();
        std::fs::create_dir_all(root.join("real")).unwrap();
        std::fs::write(root.join("real/data.txt"), b"x").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("real", root.join("alias")).unwrap();
            let resolved = resolve_in_workspace(&root, Path::new("alias/data.txt")).unwrap();
            assert!(resolved.starts_with(std::fs::canonicalize(&root).unwrap()));
            assert!(resolved.ends_with("data.txt"));
        }
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn new_files_validate_through_existing_ancestor() {
        let root = depot();
        std::fs::create_dir_all(root.join("out")).unwrap();
        let resolved = resolve_in_workspace(&root, Path::new("out/new/deep/file.json")).unwrap();
        assert!(resolved.starts_with(std::fs::canonicalize(&root).unwrap()));
        assert!(resolved.ends_with("out/new/deep/file.json"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn absolute_requested_paths_are_reanchored_under_root() {
        let root = depot();
        // An absolute request is re-anchored: "/etc/passwd" means
        // "<root>/etc/passwd", never the real /etc/passwd.
        let resolved = resolve_in_workspace(&root, Path::new("/etc/passwd")).unwrap();
        assert!(resolved.starts_with(std::fs::canonicalize(&root).unwrap()));
        assert_eq!(resolved.file_name().unwrap(), "passwd");
        std::fs::remove_dir_all(&root).ok();
    }

    use lumi_protocol::Timestamp;
}
