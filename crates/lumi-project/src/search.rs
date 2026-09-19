//! Bounded project search (spec 26 §26.11–26.12).
//!
//! v1 retrieval is deliberately simple and reliable: filename/path
//! search and exact text search with strict budgets. Sensitive and
//! dependency paths are excluded by policy (ignore rules do not equal
//! security policy, but these exclusions serve both); the filesystem
//! stays authoritative and no index is built.

use crate::error::ProjectError;
use crate::record::ProjectRecord;
use crate::roots::resolve_in_project;
use std::path::{Path, PathBuf};

/// Default result bound for search calls.
pub const MAX_RESULTS: usize = 200;

/// Largest file considered by text search.
pub const MAX_TEXT_FILE_BYTES: u64 = 1_024 * 1024;

/// Directories never searched or listed for context (dependency trees,
/// build output, VCS internals, Lumi state).
pub const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".lumi",
    ".lumi-trash",
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    "vendor",
    ".next",
    ".cache",
];

/// File patterns excluded from search/indexing (§26.12 sensitive-path
/// policy). Ignored from search does not mean unreadable: direct reads
/// still go through project root policy.
pub const EXCLUDED_FILE_PATTERNS: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    "id_rsa",
    "id_ed25519",
    ".pem",
    ".p12",
    ".pfx",
    ".key",
    ".pem.",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// Substring match against the file/folder name.
    FileName,
    /// Case-insensitive exact-substring text search inside files.
    Text,
}

/// One search hit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SearchHit {
    /// Project-relative path.
    pub path: String,
    /// Line number (1-based) for text matches.
    pub line: Option<usize>,
    /// The matching line, trimmed to a usable length.
    pub snippet: Option<String>,
}

/// Why a search was refused or truncated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    UnsafePath(ProjectError),
    Io(String),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsafePath(e) => write!(f, "unsafe path: {e}"),
            Self::Io(e) => write!(f, "search failed: {e}"),
        }
    }
}

impl std::error::Error for SearchError {}

impl From<ProjectError> for SearchError {
    fn from(e: ProjectError) -> Self {
        Self::UnsafePath(e)
    }
}

/// Searches the project's primary root (§26.11: local-first, bounded).
pub fn search(
    project: &ProjectRecord,
    query: &str,
    mode: SearchMode,
    max_results: usize,
) -> Result<Vec<SearchHit>, SearchError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let root = resolve_in_project(project, Path::new("."))?.resolved;
    let mut ctx = WalkCtx {
        root: root.clone(),
        query: query.trim().to_owned(),
        mode,
        max_results: max_results.min(MAX_RESULTS),
        max_visited: 10_000, // hard stop on absurd trees
        visited: 0,
        hits: Vec::new(),
    };
    walk(&mut ctx, &root)?;
    Ok(ctx.hits)
}

struct WalkCtx {
    root: PathBuf,
    query: String,
    mode: SearchMode,
    max_results: usize,
    max_visited: usize,
    visited: usize,
    hits: Vec<SearchHit>,
}

fn walk(ctx: &mut WalkCtx, dir: &Path) -> Result<(), SearchError> {
    if ctx.hits.len() >= ctx.max_results || ctx.visited >= ctx.max_visited {
        return Ok(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(SearchError::Io(e.to_string())),
    };
    for entry in entries.flatten() {
        ctx.visited += 1;
        if ctx.hits.len() >= ctx.max_results || ctx.visited >= ctx.max_visited {
            return Ok(());
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            if EXCLUDED_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(ctx, &path)?;
            continue;
        }
        if is_excluded_file(&name) {
            continue;
        }
        match ctx.mode {
            SearchMode::FileName => {
                if name.to_lowercase().contains(&ctx.query.to_lowercase()) {
                    ctx.hits.push(SearchHit {
                        path: relative_to(&ctx.root, &path),
                        line: None,
                        snippet: None,
                    });
                }
            }
            SearchMode::Text => {
                if meta.len() > MAX_TEXT_FILE_BYTES {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&path) else {
                    continue;
                };
                if bytes.get(..8_192).unwrap_or(&bytes).contains(&0) {
                    continue; // binary
                }
                let Ok(text) = String::from_utf8(bytes) else {
                    continue;
                };
                let query_lower = ctx.query.to_lowercase();
                for (i, line) in text.lines().enumerate() {
                    if line.to_lowercase().contains(&query_lower) {
                        ctx.hits.push(SearchHit {
                            path: relative_to(&ctx.root, &path),
                            line: Some(i + 1),
                            snippet: Some(line.trim().chars().take(200).collect()),
                        });
                        if ctx.hits.len() >= ctx.max_results {
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn is_excluded_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    EXCLUDED_FILE_PATTERNS
        .iter()
        .any(|pattern| lower.starts_with(pattern) || lower.ends_with(pattern))
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

/// Lists a bounded directory tree (§26.7 discovery substrate; used by
/// the Files surface).
pub fn list_dir(
    project: &ProjectRecord,
    path: &Path,
    max_entries: usize,
) -> Result<Vec<ListedEntry>, SearchError> {
    let root = resolve_in_project(project, Path::new("."))?.resolved;
    let resolved = resolve_in_project(project, path)?;
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&resolved.resolved) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(SearchError::Io(e.to_string())),
    };
    for entry in entries.flatten() {
        if out.len() >= max_entries {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.metadata().map(|m| m.is_dir()).unwrap_or(false)
            && EXCLUDED_DIRS.contains(&name.as_str())
        {
            continue;
        }
        let meta = entry.metadata().ok();
        let rel = entry
            .path()
            .strip_prefix(&root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| name.clone());
        out.push(ListedEntry {
            name: name.clone(),
            path: rel,
            is_dir: meta.as_ref().is_some_and(|m| m.is_dir()),
            size: meta.as_ref().map(|m| m.len()),
        });
    }
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(out)
}

/// One listed filesystem entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ListedEntry {
    pub name: String,
    /// Project-relative path.
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{AuthorizedRoot, DetectedSource, IndexingState, RootAccess};
    use lumi_protocol::ids::{EnvironmentId, PrincipalId, ProjectId};
    use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
    use lumi_protocol::{TenantId, Timestamp};

    fn unique_dir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-project-search-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }

    fn project_at(root: PathBuf) -> ProjectRecord {
        let now = Timestamp::from_epoch(0, 0).unwrap();
        let record = ProjectRecord {
            project_id: ProjectId::parse("p-search").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            owner: Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(now),
                authentication_strength: Some(AuthenticationStrength::DevicePossession),
            },
            execution_environment_id: EnvironmentId::parse("env-1").unwrap(),
            display_name: "search".to_owned(),
            primary_root: root.clone(),
            authorized_roots: vec![AuthorizedRoot {
                root_id: "primary".to_owned(),
                path: root,
                access: RootAccess::ReadWrite,
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
    fn filename_search_finds_and_skips_excluded_dirs() {
        let root = unique_dir("names");
        std::fs::create_dir_all(root.join("src/orders")).unwrap();
        std::fs::write(root.join("src/orders/sync.ts"), b"x").unwrap();
        std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        std::fs::write(root.join("node_modules/pkg/sync.ts"), b"x").unwrap();

        let hits = search(
            &project_at(root.clone()),
            "sync.ts",
            SearchMode::FileName,
            100,
        )
        .unwrap();
        assert_eq!(hits.len(), 1, "node_modules must be excluded");
        assert_eq!(hits[0].path.replace('\\', "/"), "src/orders/sync.ts");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn text_search_finds_lines_with_snippets() {
        let root = unique_dir("text");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/app.rs"), "fn main() {}\n// TODO fix later\n").unwrap();
        let hits = search(&project_at(root.clone()), "todo", SearchMode::Text, 100).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, Some(2));
        assert!(hits[0]
            .snippet
            .as_deref()
            .is_some_and(|s| s.contains("TODO")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn sensitive_files_are_excluded_from_search() {
        let root = unique_dir("sensitive");
        std::fs::write(root.join(".env"), "SECRET=1\n").unwrap();
        std::fs::write(root.join("server.key"), "----\n").unwrap();
        std::fs::write(root.join("readme.md"), "SECRET=1\n").unwrap();

        let hits = search(&project_at(root.clone()), "secret", SearchMode::Text, 100).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "readme.md");

        let key_hits = search(
            &project_at(root.clone()),
            "server",
            SearchMode::FileName,
            100,
        )
        .unwrap();
        assert!(key_hits.is_empty(), "keys must never be searched");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn binary_and_empty_queries_are_handled() {
        let root = unique_dir("binary");
        std::fs::write(root.join("blob.bin"), [0u8, 1, 2]).unwrap();
        assert!(search(&project_at(root.clone()), "", SearchMode::Text, 100)
            .unwrap()
            .is_empty());
        let hits = search(&project_at(root.clone()), "1", SearchMode::Text, 100).unwrap();
        assert!(hits.is_empty(), "binary files are not text-searched");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn list_dir_returns_sorted_bounded_entries() {
        let root = unique_dir("list");
        std::fs::create_dir_all(root.join("zdir")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/x")).unwrap();
        std::fs::write(root.join("a.txt"), b"a").unwrap();
        let listed = list_dir(&project_at(root.clone()), Path::new("."), 50).unwrap();
        let names: Vec<&str> = listed.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["zdir", "a.txt"],
            "dirs first, node_modules excluded"
        );
        std::fs::remove_dir_all(&root).ok();
    }
}
