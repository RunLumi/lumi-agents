//! Durable Projects and root-bounded filesystem authority (spec 26).
//!
//! A Project is a durable, policy-bounded working context rooted in one
//! ExecutionEnvironment (`ExecutionEnvironment -> Project -> Task ->
//! Run -> Action`, §26.2). This crate implements the v1 Project
//! contract: the durable registry ([`ProjectStore`]), the project
//! record with authorized roots and instruction provenance
//! ([`ProjectRecord`]), and root-bounded path resolution
//! ([`resolve_in_project`]).
//!
//! Invariants enforced here:
//!
//! - Project identity is stable and independent of display name and
//!   path, proven on disk by a `.lumi/project.json` marker (§26.3).
//! - A missing or moved root is health to surface, never a reason to
//!   recreate or re-point state (§26.4). Relinking is deliberate.
//! - Authorized roots are the default boundary; siblings, home
//!   directories, and credential stores are never inferred (§26.5,
//!   §26.26). Resolution fails closed on traversal and symlink escape.
//! - Project instruction files are recorded as provenance only; they do
//!   not and cannot widen authority (§26.8).

pub mod changeset;
pub mod discovery;
pub mod error;
pub mod files;
pub mod git;
pub mod memory;
pub mod record;
pub mod roots;
pub mod search;
pub mod store;
pub mod validation;

pub use changeset::{
    checksum, text_patch, ChangeEntry, ChangeKind, ChangeSet, ChangeSetStore, ChangeSource,
    CommandRecord,
};
pub use discovery::{discover, source_label, ProjectDiscovery, ProposedCommand};
pub use error::ProjectError;
pub use files::{FileOpsError, MutationOutcome, ProjectFiles, MAX_PATCH_BYTES, MAX_READ_BYTES};
pub use git::{
    clone_repository, normalize_path, CommitInfo, FileStatus, GitError, GitRepo, GitStatus,
    WorktreeEntry, GIT_MAX_OUTPUT_BYTES, GIT_TIMEOUT_MS,
};
pub use memory::{Invalidation, MemoryKind, MemoryProvenance, MemoryRecord, ProjectMemoryStore};
pub use record::{
    capability_snapshot, detect_git, detect_source, scan_instructions, AuthorizedRoot,
    DetectedSource, GitMetadata, IndexingState, InstructionKind, InstructionSource,
    OpenFolderRequest, OpenOutcome, ProjectRecord, RootAccess, INSTRUCTION_SCAN_LIMIT_BYTES,
};
pub use roots::{resolve_in_project, ResolvedInProject};
pub use search::{list_dir, search, ListedEntry, SearchError, SearchHit, SearchMode, MAX_RESULTS};
pub use store::{PersistedProjects, ProjectStore, RootHealth, PROJECTS_SCHEMA_VERSION};
pub use validation::{run_validation, ValidationRecord, ValidationStatus};
