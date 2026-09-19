//! Bounded project discovery (spec 26 §26.7, §26.14).
//!
//! Discovery answers "how does this project work?" without reading the
//! whole tree: top-level manifests, lockfiles, test/lint/format/CI
//! configuration, and likely entry points, all under file-count and
//! depth budgets. Discovered commands are PROPOSALS for validation —
//! executing them still passes the shell policy boundary (§26.14:
//! project content never bypasses policy).

use crate::record::{DetectedSource, ProjectRecord};
use std::path::Path;

/// A validation command Lumi proposes for this project. A proposal is
/// not an authorization: execution goes through the shell policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedCommand {
    /// Stable role: "test", "build", "lint", "format", "typecheck".
    pub role: &'static str,
    /// The command as it would run from the project root.
    pub command: String,
    /// Where the convention was found (manifest/config path, relative).
    pub evidence: String,
}

/// Result of one bounded discovery pass.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectDiscovery {
    /// Top-level manifests found (relative paths).
    pub manifests: Vec<String>,
    /// Lockfiles found (relative paths).
    pub lockfiles: Vec<String>,
    /// Validation command proposals derived from manifests/config.
    pub proposed_commands: Vec<ProposedCommand>,
    /// CI configuration files found.
    pub ci_configs: Vec<String>,
    /// Likely entry points (relative paths).
    pub entry_points: Vec<String>,
    /// Whether discovery hit its budget before finishing.
    pub truncated: bool,
}

/// Manifests that pin an ecosystem, with the commands they conventionally
/// expose. Nothing here is executed by discovery itself.
const MANIFEST_CONVENTIONS: &[(&str, &str, &str, &str)] = &[
    // (file, source-kind, role, command)
    ("Cargo.toml", "Rust", "test", "cargo test"),
    ("Cargo.toml", "Rust", "build", "cargo build"),
    ("Cargo.toml", "Rust", "lint", "cargo clippy"),
    ("Cargo.toml", "Rust", "format", "cargo fmt --check"),
    ("package.json", "Node", "test", "npm test"),
    ("package.json", "Node", "build", "npm run build"),
    ("package.json", "Node", "lint", "npm run lint"),
    ("pyproject.toml", "Python", "test", "pytest"),
    ("pyproject.toml", "Python", "lint", "ruff check ."),
    (
        "pyproject.toml",
        "Python",
        "format",
        "ruff format --check .",
    ),
    ("go.mod", "Go", "test", "go test ./..."),
    ("go.mod", "Go", "build", "go build ./..."),
];

const LOCKFILES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "poetry.lock",
    "uv.lock",
    "go.sum",
];

const CI_CONFIGS: &[&str] = &[
    ".github/workflows",
    ".gitlab-ci.yml",
    ".circleci",
    "azure-pipelines.yml",
];

const ENTRY_POINT_CANDIDATES: &[&str] = &[
    "src/main.rs",
    "src/lib.rs",
    "src/main.ts",
    "src/index.ts",
    "main.go",
    "index.js",
    "main.py",
    "app.py",
    "package.json",
];

/// Discovery budgets (§26.7: bounded by file count and depth).
const MAX_TOP_LEVEL_ENTRIES: usize = 200;
const MAX_CI_SCAN_DEPTH: usize = 2;

/// Performs bounded discovery for a project (top level + one CI level).
///
/// # Errors
/// [`crate::ProjectError::RootMissing`] when the root is gone; I/O
/// errors other than missing entries abort the pass.
pub fn discover(project: &ProjectRecord) -> Result<ProjectDiscovery, crate::error::ProjectError> {
    let root = &project.primary_root;
    if !root.exists() {
        return Err(crate::error::ProjectError::RootMissing {
            path: root.display().to_string(),
        });
    }

    let mut discovery = ProjectDiscovery::default();
    let mut top_level: Vec<String> = Vec::new();
    let entries =
        std::fs::read_dir(root).map_err(|e| crate::error::ProjectError::Io(e.to_string()))?;
    for entry in entries.flatten().take(MAX_TOP_LEVEL_ENTRIES) {
        if let Ok(name) = entry.file_name().into_string() {
            top_level.push(name);
        }
    }

    for (file, _, role, command) in MANIFEST_CONVENTIONS {
        if top_level.iter().any(|name| name == file)
            && !discovery
                .proposed_commands
                .iter()
                .any(|p| p.role == *role && p.command == *command)
        {
            discovery.manifests.push((*file).to_owned());
            discovery.proposed_commands.push(ProposedCommand {
                role,
                command: (*command).to_owned(),
                evidence: (*file).to_owned(),
            });
        }
    }

    for lock in LOCKFILES {
        if top_level.iter().any(|name| name == lock) {
            discovery.lockfiles.push((*lock).to_owned());
        }
    }

    for candidate in ENTRY_POINT_CANDIDATES {
        if top_level.iter().any(|name| name == candidate) {
            discovery.entry_points.push((*candidate).to_owned());
        }
    }

    // CI configs: `.github/workflows` is a directory; the rest are files.
    for ci in CI_CONFIGS {
        let path = root.join(ci);
        if path.is_dir() || path.is_file() {
            discovery.ci_configs.push((*ci).to_owned());
        }
    }

    // A workspace-level Cargo.toml with a [workspace] section proposes
    // workspace-wide validation; detection is textual on the one file
    // discovery is allowed to read (bounded, top-level only).
    let cargo = root.join("Cargo.toml");
    if let Ok(text) = std::fs::read_to_string(&cargo) {
        if text.contains("[workspace]") {
            for proposal in &mut discovery.proposed_commands {
                if proposal.role == "test" {
                    proposal.command = "cargo test --workspace".to_owned();
                }
            }
        }
    }

    discovery.truncated =
        top_level.len() >= MAX_TOP_LEVEL_ENTRIES && !walk_depth_ok(root, 0, MAX_CI_SCAN_DEPTH);
    Ok(discovery)
}

fn walk_depth_ok(dir: &Path, depth: usize, max_depth: usize) -> bool {
    if depth > max_depth {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return true;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() && !walk_depth_ok(&entry.path(), depth + 1, max_depth) {
            return false;
        }
    }
    true
}

/// Maps a detected source type onto the discovery result for reporting.
#[must_use]
pub const fn source_label(source: DetectedSource) -> &'static str {
    match source {
        DetectedSource::Rust => "Rust",
        DetectedSource::Node => "Node",
        DetectedSource::Python => "Python",
        DetectedSource::Go => "Go",
        DetectedSource::Generic => "Generic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{AuthorizedRoot, IndexingState, RootAccess};
    use lumi_protocol::ids::{EnvironmentId, PrincipalId, ProjectId};
    use lumi_protocol::principal::{AuthenticationStrength, Principal, PrincipalKind};
    use lumi_protocol::{TenantId, Timestamp};
    use std::path::PathBuf;

    fn unique_dir(tag: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "lumi-project-disc-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }

    fn project_at(root: PathBuf) -> ProjectRecord {
        let now = Timestamp::from_epoch(0, 0).unwrap();
        let record = ProjectRecord {
            project_id: ProjectId::parse("p-disc").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            owner: Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(now),
                authentication_strength: Some(AuthenticationStrength::DevicePossession),
            },
            execution_environment_id: EnvironmentId::parse("env-1").unwrap(),
            display_name: "disc".to_owned(),
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
    fn rust_workspace_discovery_proposes_workspace_test_command() {
        let root = unique_dir("rust");
        std::fs::write(
            root.join("Cargo.toml"),
            b"[package]\nname=\"d\"\n\n[workspace]\nmembers=[\"crates/x\"]\n",
        )
        .unwrap();
        std::fs::write(root.join("Cargo.lock"), b"").unwrap();
        std::fs::create_dir_all(root.join(".github/workflows")).unwrap();
        std::fs::write(root.join(".github/workflows/ci.yml"), b"on: push\n").unwrap();

        let discovery = discover(&project_at(root.clone())).unwrap();
        assert!(discovery.manifests.contains(&"Cargo.toml".to_owned()));
        assert!(discovery.lockfiles.contains(&"Cargo.lock".to_owned()));
        assert!(discovery
            .ci_configs
            .contains(&".github/workflows".to_owned()));
        let test = discovery
            .proposed_commands
            .iter()
            .find(|c| c.role == "test")
            .unwrap();
        assert_eq!(
            test.command, "cargo test --workspace",
            "workspace-aware proposal"
        );
        assert_eq!(test.evidence, "Cargo.toml");
        assert!(!discovery.truncated);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn node_discovery_proposes_npm_commands() {
        let root = unique_dir("node");
        std::fs::write(root.join("package.json"), b"{\"name\":\"d\"}").unwrap();
        std::fs::write(root.join("package-lock.json"), b"{}").unwrap();
        let discovery = discover(&project_at(root.clone())).unwrap();
        let roles: Vec<&str> = discovery.proposed_commands.iter().map(|c| c.role).collect();
        assert!(roles.contains(&"test"));
        assert!(roles.contains(&"build"));
        assert!(roles.contains(&"lint"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_project_discovers_nothing_and_never_invents_commands() {
        let root = unique_dir("empty");
        let discovery = discover(&project_at(root.clone())).unwrap();
        assert!(discovery.manifests.is_empty());
        assert!(discovery.proposed_commands.is_empty());
        assert!(discovery.entry_points.is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn discovery_fails_closed_on_missing_root() {
        let root = unique_dir("gone");
        let project = project_at(root.clone());
        std::fs::remove_dir_all(&root).unwrap();
        let err = discover(&project).unwrap_err();
        assert!(matches!(
            err,
            crate::error::ProjectError::RootMissing { .. }
        ));
    }
}
