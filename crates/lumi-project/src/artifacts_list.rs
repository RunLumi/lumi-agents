//! Project artifact listing (spec 26 §26.21, §26.29).
//!
//! The Artifacts surface shows REAL generated outputs: entries found
//! under the project's `.lumi/artifacts` store (the spec 08 layout with
//! `artifact.json` index records) plus any loose files Lumi wrote there.
//! An empty project has an empty list — no sample rows, ever.

use serde::Serialize;
use std::path::Path;

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

/// Lists bounded artifact entries for a project root (§26.7 budgets:
/// entry cap, no recursion beyond the store's two levels).
///
/// # Errors
/// [`crate::ProjectError::RootMissing`] when the root is gone; missing
/// artifact directories are simply an empty list.
pub fn list_artifacts(
    project_root: &Path,
    max_entries: usize,
) -> Result<Vec<ArtifactEntry>, crate::ProjectError> {
    if !project_root.exists() {
        return Err(crate::ProjectError::RootMissing {
            path: project_root.display().to_string(),
        });
    }
    let store = project_root.join(ARTIFACTS_DIR);
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&store) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(crate::ProjectError::Io(e.to_string())),
    };
    for entry in entries.flatten().take(max_entries) {
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        let index = dir_path.join("artifact.json");
        #[derive(serde::Deserialize)]
        struct Index {
            artifact_type: String,
            #[serde(default)]
            lifecycle: Option<String>,
            #[serde(default)]
            sha256: Option<String>,
        }
        let index: Option<Index> = std::fs::read_to_string(&index)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        let files = match std::fs::read_dir(&dir_path) {
            Ok(files) => files,
            Err(_) => continue,
        };
        for file in files.flatten() {
            let path = file.path();
            if path.file_name().is_some_and(|n| n == "artifact.json") {
                continue;
            }
            let Ok(meta) = file.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let relative = path
                .strip_prefix(project_root)
                .map(|p| p.display().to_string())
                .unwrap_or_default();
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
                return Ok(out);
            }
        }
    }
    out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
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
}
