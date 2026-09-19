//! The durable Project record (spec 26 §26.3).
//!
//! A Project is a durable, policy-bounded working context rooted in one
//! ExecutionEnvironment. The record carries stable identity, authorized
//! filesystem roots, discovered metadata, and instruction provenance.
//! It never carries secrets and never grants authority by itself: the
//! root boundary is enforced at every filesystem/shell/git operation.

use crate::error::ProjectError;
use lumi_protocol::{EnvironmentId, Principal, ProjectId, TenantId, Timestamp};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Access mode of one authorized root (spec 26 §26.26).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootAccess {
    ReadWrite,
    ReadOnly,
}

/// One explicitly authorized filesystem root. Sibling directories are
/// never inferred (§26.5, §26.26).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizedRoot {
    /// Stable root identifier within the project.
    pub root_id: String,
    /// Canonical absolute path.
    pub path: PathBuf,
    pub access: RootAccess,
}

/// Detected source type from top-level manifests (§26.3 "project type").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectedSource {
    Rust,
    Node,
    Python,
    Go,
    Generic,
}

/// Kind of a discovered project instruction file (§26.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionKind {
    AgentsMd,
    Readme,
    Contributor,
    Other,
}

/// Provenance for one project instruction source (§26.8): instructions
/// are observations about how the project works. They never grant
/// authority; provenance makes their origin inspectable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructionSource {
    /// Path relative to the primary root.
    pub path: String,
    pub kind: InstructionKind,
    pub sha256: String,
    pub read_at: Timestamp,
}

/// Search/index freshness for the project (§26.3 "indexing state").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexingState {
    NotIndexed,
    Stale,
    Indexed { updated_at: Timestamp },
}

/// Minimal Git discovery recorded at open time (§26.16). Deeper Git
/// metadata is read on demand by the Git capability, not stored here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMetadata {
    pub is_repository: bool,
    pub discovered_at: Timestamp,
}

/// Durable project record (§26.3). Identity survives application
/// restart and is independent of display name and filesystem path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub project_id: ProjectId,
    pub tenant_id: TenantId,
    pub owner: Principal,
    pub execution_environment_id: EnvironmentId,
    pub display_name: String,
    /// Canonical absolute path of the primary authorized root.
    pub primary_root: PathBuf,
    pub authorized_roots: Vec<AuthorizedRoot>,
    pub created_at: Timestamp,
    pub last_opened_at: Timestamp,
    pub detected_source: DetectedSource,
    /// Capability snapshot advertised for this project (§26.32).
    pub capability_snapshot: Vec<String>,
    /// Reference to the project policy/config document (§26.23). A
    /// project config may narrow behavior; it can never widen
    /// organization/user authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_reference: Option<String>,
    /// Discovered instruction files with provenance (§26.8).
    pub instruction_sources: Vec<InstructionSource>,
    pub indexing_state: IndexingState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_metadata: Option<GitMetadata>,
}

/// Largest instruction file hashed at open time (§26.13 read bounds);
/// anything larger is not an instruction source for v1 discovery.
pub const INSTRUCTION_SCAN_LIMIT_BYTES: u64 = 1024 * 1024;

impl ProjectRecord {
    /// Structural invariants enforced on every load (fail closed on a
    /// corrupt record).
    ///
    /// # Errors
    /// [`ProjectError::InvalidRecord`] when the record violates them.
    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.authorized_roots.is_empty() {
            return Err(ProjectError::InvalidRecord(
                "no authorized roots".to_owned(),
            ));
        }
        if !self
            .authorized_roots
            .iter()
            .any(|r| r.path == self.primary_root)
        {
            return Err(ProjectError::InvalidRecord(
                "primary root is not among authorized roots".to_owned(),
            ));
        }
        let mut ids: Vec<&str> = self
            .authorized_roots
            .iter()
            .map(|r| r.root_id.as_str())
            .collect();
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(ProjectError::InvalidRecord("duplicate root ids".to_owned()));
        }
        if self.display_name.trim().is_empty() {
            return Err(ProjectError::InvalidRecord("empty display name".to_owned()));
        }
        Ok(())
    }

    /// The authorized root containing `path`, if any.
    #[must_use]
    pub fn root_for(&self, path: &Path) -> Option<&AuthorizedRoot> {
        self.authorized_roots
            .iter()
            .find(|r| path.starts_with(&r.path))
    }
}

/// Request for the Open Folder flow (spec 26 §26.4).
#[derive(Debug, Clone)]
pub struct OpenFolderRequest {
    pub tenant_id: TenantId,
    pub owner: Principal,
    pub execution_environment_id: EnvironmentId,
    /// User-selected folder; canonicalized before use.
    pub path: PathBuf,
    /// Optional display name; defaults to the folder name.
    pub display_name: Option<String>,
    /// Optional project policy/config reference (§26.23).
    pub policy_reference: Option<String>,
}

/// Result of an Open Folder request: the project is either created or
/// reopened under its existing stable identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOutcome {
    Created(ProjectRecord),
    Reopened(ProjectRecord),
}

impl OpenOutcome {
    #[must_use]
    pub const fn record(&self) -> &ProjectRecord {
        match self {
            Self::Created(r) | Self::Reopened(r) => r,
        }
    }
}

/// Detects the source type from top-level manifests only (bounded
/// discovery; never walks the tree).
///
/// # Errors
/// Never; falls back to [`DetectedSource::Generic`].
#[must_use]
pub fn detect_source(root: &Path) -> DetectedSource {
    let markers = [
        ("Cargo.toml", DetectedSource::Rust),
        ("package.json", DetectedSource::Node),
        ("pyproject.toml", DetectedSource::Python),
        ("go.mod", DetectedSource::Go),
    ];
    for (file, source) in markers {
        if root.join(file).is_file() {
            return source;
        }
    }
    DetectedSource::Generic
}

/// Scans top-level instruction files and records provenance (§26.8).
/// Only `AGENTS.md` and `README.md` at the primary root are read; each
/// is capped at [`INSTRUCTION_SCAN_LIMIT_BYTES`].
///
/// # Errors
/// Never; unreadable or oversized files are simply not recorded.
pub fn scan_instructions(root: &Path, now: Timestamp) -> Vec<InstructionSource> {
    let candidates = [
        ("AGENTS.md", InstructionKind::AgentsMd),
        ("README.md", InstructionKind::Readme),
    ];
    let mut found = Vec::new();
    for (name, kind) in candidates {
        let path = root.join(name);
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() || meta.len() > INSTRUCTION_SCAN_LIMIT_BYTES {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        found.push(InstructionSource {
            path: name.to_owned(),
            kind,
            sha256: lumi_protocol::canonical::sha256_hex(&bytes),
            read_at: now,
        });
    }
    found
}

/// Detects a Git repository at the root via `.git` presence (§26.16).
/// No git binary is invoked at open time.
#[must_use]
pub fn detect_git(root: &Path, now: Timestamp) -> Option<GitMetadata> {
    let dotgit = root.join(".git");
    if dotgit.is_dir() || dotgit.is_file() {
        Some(GitMetadata {
            is_repository: true,
            discovered_at: now,
        })
    } else {
        None
    }
}

/// Capabilities advertised for a freshly opened project (§26.32). Only
/// capabilities that exist in the runtime are advertised; Git/clone
/// entries appear once those capabilities ship.
#[must_use]
pub fn capability_snapshot(is_repository: bool) -> Vec<String> {
    let mut caps = vec![
        "project_open_folder".to_owned(),
        "project_recent".to_owned(),
        "files_read".to_owned(),
        "files_write".to_owned(),
        "shell_host_bounded".to_owned(),
    ];
    if is_repository {
        caps.push("git_read".to_owned());
    }
    caps
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::ids::PrincipalId;
    use lumi_protocol::principal::{AuthenticationStrength, PrincipalKind};

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
            "lumi-project-record-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detect_source_matches_top_level_manifests() {
        let dir = unique_dir("source");
        std::fs::write(dir.join("package.json"), b"{}").unwrap();
        assert_eq!(detect_source(&dir), DetectedSource::Node);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_source_falls_back_to_generic() {
        let dir = unique_dir("generic");
        assert_eq!(detect_source(&dir), DetectedSource::Generic);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_instructions_hashes_present_files_only() {
        let dir = unique_dir("instructions");
        std::fs::write(dir.join("AGENTS.md"), b"be careful").unwrap();
        std::fs::write(dir.join("README.md"), b"# demo").unwrap();
        let now = Timestamp::from_epoch(0, 0).unwrap();
        let found = scan_instructions(&dir, now);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].kind, InstructionKind::AgentsMd);
        assert_eq!(found[0].path, "AGENTS.md");
        assert!(!found[0].sha256.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_instructions_skips_oversized_files() {
        let dir = unique_dir("big");
        let big = std::fs::File::create(dir.join("README.md")).unwrap();
        big.set_len(INSTRUCTION_SCAN_LIMIT_BYTES + 1).unwrap();
        drop(big);
        let found = scan_instructions(&dir, Timestamp::from_epoch(0, 0).unwrap());
        assert!(found.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_git_requires_dotgit() {
        let dir = unique_dir("git");
        assert!(detect_git(&dir, Timestamp::from_epoch(0, 0).unwrap()).is_none());
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        let meta = detect_git(&dir, Timestamp::from_epoch(1, 0).unwrap()).unwrap();
        assert!(meta.is_repository);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn capability_snapshot_lists_existing_capabilities_only() {
        let caps = capability_snapshot(false);
        assert!(caps.contains(&"project_open_folder".to_owned()));
        assert!(!caps.iter().any(|c| c.starts_with("git_")));
        assert!(capability_snapshot(true).contains(&"git_read".to_owned()));
    }

    #[test]
    fn validate_rejects_rootless_and_mismatched_primary() {
        let now = Timestamp::from_epoch(0, 0).unwrap();
        let base = ProjectRecord {
            project_id: ProjectId::parse("p-1").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            owner: principal(),
            execution_environment_id: EnvironmentId::parse("env-1").unwrap(),
            display_name: "demo".to_owned(),
            primary_root: PathBuf::from("/tmp/a"),
            authorized_roots: vec![],
            created_at: now,
            last_opened_at: now,
            detected_source: DetectedSource::Generic,
            capability_snapshot: vec![],
            policy_reference: None,
            instruction_sources: vec![],
            indexing_state: IndexingState::NotIndexed,
            git_metadata: None,
        };
        assert!(base.validate().is_err());

        let mut bad_primary = base.clone();
        bad_primary.authorized_roots = vec![AuthorizedRoot {
            root_id: "primary".to_owned(),
            path: PathBuf::from("/tmp/b"),
            access: RootAccess::ReadWrite,
        }];
        assert!(bad_primary.validate().is_err());
    }
}
