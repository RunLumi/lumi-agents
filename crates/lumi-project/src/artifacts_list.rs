//! Project artifact listing (spec 26 §26.21, §26.29).
//!
//! The Artifacts surface shows REAL generated outputs: entries found
//! under the project's `.lumi/artifacts` store (the spec 08 layout with
//! `artifact.json` index records) and files in those artifact directories.
//! An empty project has an empty list — no sample rows, ever.

use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Store location convention inside a project root.
pub const ARTIFACTS_DIR: &str = ".lumi/artifacts";

/// One listed artifact for the desktop surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArtifactEntry {
    /// File name on disk.
    pub name: String,
    /// Path relative to the project root.
    pub path: String,
    /// Declared type from the artifact index when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,
    /// Lifecycle from the index when present (draft/ready/published…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<String>,
    /// SHA-256 from the index when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub size: u64,
    /// Modified time (unix seconds).
    pub modified_at: i64,
}

// Bound optional index metadata independently from the result count.
const MAX_INDEX_BYTES: u64 = 64 * 1024;

fn scoped_path(root: &Path, relative: &Path) -> Result<PathBuf, crate::ProjectError> {
    lumi_workspaces::resolve_in_workspace(root, relative).map_err(|e| match e {
        lumi_workspaces::PathError::Io(message) => crate::ProjectError::Io(message),
        _ => crate::ProjectError::OutsideProjectRoots {
            requested: relative.display().to_string(),
        },
    })
}

/// Lists bounded artifact entries for a project root (§26.7 budgets:
/// entry cap, no recursion beyond the store's two levels).
///
/// # Errors
/// [`crate::ProjectError::RootMissing`] when the root is gone; missing
/// artifact directories are simply an empty list. Escapes from the project
/// root are refused, including store, artifact, index, and output symlinks.
/// Index fields are declarations, not independent verification of the output.
pub fn list_artifacts(
    project_root: &Path,
    max_entries: usize,
) -> Result<Vec<ArtifactEntry>, crate::ProjectError> {
    if !project_root.exists() {
        return Err(crate::ProjectError::RootMissing {
            path: project_root.display().to_string(),
        });
    }
    let root =
        std::fs::canonicalize(project_root).map_err(|e| crate::ProjectError::Io(e.to_string()))?;
    if !root.is_dir() {
        return Err(crate::ProjectError::RootNotADirectory {
            path: root.display().to_string(),
        });
    }
    let max_entries = max_entries.min(crate::search::MAX_RESULTS);
    if max_entries == 0 {
        return Ok(Vec::new());
    }
    let store = scoped_path(&root, Path::new(ARTIFACTS_DIR))?;
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&store) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(crate::ProjectError::Io(e.to_string())),
    };
    for entry in entries.take(max_entries) {
        let entry = entry.map_err(|e| crate::ProjectError::Io(e.to_string()))?;
        let relative_dir = Path::new(ARTIFACTS_DIR).join(entry.file_name());
        let dir_path = scoped_path(&root, &relative_dir)?;
        if !dir_path.is_dir() {
            continue;
        }
        let index = scoped_path(&root, &relative_dir.join("artifact.json"))?;
        #[derive(serde::Deserialize)]
        struct Index {
            artifact_type: String,
            #[serde(default)]
            lifecycle: Option<String>,
            #[serde(default)]
            sha256: Option<String>,
        }
        let index: Option<Index> = if index.is_file() {
            let mut bytes = Vec::new();
            std::fs::File::open(&index)
                .and_then(|file| file.take(MAX_INDEX_BYTES + 1).read_to_end(&mut bytes))
                .ok()
                .filter(|size| *size <= MAX_INDEX_BYTES as usize)
                .and_then(|_| serde_json::from_slice(&bytes).ok())
        } else {
            None
        };
        let files =
            std::fs::read_dir(&dir_path).map_err(|e| crate::ProjectError::Io(e.to_string()))?;
        // Bound visited entries as well as returned files (one extra for index).
        for file in files.take(max_entries + 1) {
            let file = file.map_err(|e| crate::ProjectError::Io(e.to_string()))?;
            let relative = relative_dir.join(file.file_name());
            let path = scoped_path(&root, &relative)?;
            if file.file_name() == "artifact.json" {
                continue;
            }
            let meta =
                std::fs::metadata(&path).map_err(|e| crate::ProjectError::Io(e.to_string()))?;
            if !meta.is_file() {
                continue;
            }
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let relative = relative.display().to_string();
            out.push(ArtifactEntry {
                name: file.file_name().to_string_lossy().to_string(),
                path: relative,
                artifact_type: index.as_ref().map(|i| i.artifact_type.clone()),
                lifecycle: index.as_ref().and_then(|i| i.lifecycle.clone()),
                sha256: index.as_ref().and_then(|i| i.sha256.clone()),
                size: meta.len(),
                modified_at: modified,
            });
            if out.len() >= max_entries {
                out.sort_by_key(|entry| std::cmp::Reverse(entry.modified_at));
                return Ok(out);
            }
        }
    }
    out.sort_by_key(|entry| std::cmp::Reverse(entry.modified_at));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_has_empty_artifacts() {
        let dir = std::env::temp_dir().join(format!("lumi-artifacts-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(list_artifacts(&dir, 50).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn artifacts_list_reads_store_entries() {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-artifacts-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let artifact_dir = dir.join(".lumi/artifacts/report-1");
        std::fs::create_dir_all(&artifact_dir).unwrap();
        std::fs::write(
            artifact_dir.join("artifact.json"),
            r#"{"artifact_id":"report-1","artifact_type":"test_report","path":"results.json","lifecycle":"READY","sha256":"abc"}"#,
        )
        .unwrap();
        std::fs::write(artifact_dir.join("results.json"), b"{}").unwrap();

        let listed = list_artifacts(&dir, 50).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "results.json");
        assert_eq!(listed[0].artifact_type.as_deref(), Some("test_report"));
        assert_eq!(listed[0].lifecycle.as_deref(), Some("READY"));
        assert_eq!(listed[0].sha256.as_deref(), Some("abc"));
        assert!(listed[0]
            .path
            .replace('\\', "/")
            .starts_with(".lumi/artifacts"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_root_fails_closed() {
        let dir = std::env::temp_dir().join("lumi-artifacts-gone");
        std::fs::remove_dir_all(&dir).ok();
        assert!(matches!(
            list_artifacts(&dir, 50),
            Err(crate::ProjectError::RootMissing { .. })
        ));
    }

    #[test]
    fn entry_budget_and_invalid_index_do_not_invent_metadata() {
        let dir =
            std::env::temp_dir().join(format!("lumi-artifacts-budget-{}", std::process::id()));
        let store = dir.join(ARTIFACTS_DIR).join("report");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::write(store.join("artifact.json"), b"invalid json").unwrap();
        for name in ["one.txt", "two.txt", "three.txt"] {
            std::fs::write(store.join(name), b"output").unwrap();
        }
        assert!(list_artifacts(&dir, 0).unwrap().is_empty());
        let entries = list_artifacts(&dir, 2).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|entry| entry.artifact_type.is_none()));
        std::fs::write(
            store.join("artifact.json"),
            vec![b' '; MAX_INDEX_BYTES as usize + 1],
        )
        .unwrap();
        assert_eq!(list_artifacts(&dir, 10).unwrap().len(), 3);
        assert_eq!(list_artifacts(&dir, usize::MAX).unwrap().len(), 3);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_escape_at_every_store_level() {
        use std::os::unix::fs::symlink;
        for level in ["store", "artifact", "index", "output"] {
            let base = std::env::temp_dir().join(format!(
                "lumi-artifacts-escape-{level}-{}",
                std::process::id()
            ));
            let root = base.join("project");
            let outside = base.join("outside");
            std::fs::create_dir_all(&root).unwrap();
            std::fs::create_dir_all(&outside).unwrap();
            std::fs::write(outside.join("secret.txt"), b"private").unwrap();
            let store = root.join(ARTIFACTS_DIR);
            let artifact = store.join("report");
            match level {
                "store" => {
                    std::fs::create_dir_all(root.join(".lumi")).unwrap();
                    symlink(&outside, &store).unwrap();
                }
                "artifact" => {
                    std::fs::create_dir_all(&store).unwrap();
                    symlink(&outside, &artifact).unwrap();
                }
                "index" | "output" => {
                    std::fs::create_dir_all(&artifact).unwrap();
                    let name = if level == "index" {
                        "artifact.json"
                    } else {
                        "leak.txt"
                    };
                    symlink(outside.join("secret.txt"), artifact.join(name)).unwrap();
                }
                _ => unreachable!(),
            }
            assert!(
                matches!(
                    list_artifacts(&root, 50),
                    Err(crate::ProjectError::OutsideProjectRoots { .. })
                ),
                "escape at {level}"
            );
            std::fs::remove_dir_all(base).unwrap();
        }
    }
}
