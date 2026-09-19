//! Project registry and authority errors (spec 26 §26.30).
//!
//! Every failure mode is explicit and fail-closed: nothing here widens
//! scope or recreates state to keep going.

use std::fmt;

/// Why a project operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    /// The selected folder does not exist.
    RootMissing { path: String },
    /// The selected path exists but is not a directory.
    RootNotADirectory { path: String },
    /// The project folder could not be proven to be the same resource
    /// (marker missing or replaced). Deliberate relink is required.
    IdentityUnproven { path: String },
    /// Filesystem failure.
    Io(String),
    /// Serialization failure or corrupt durable document.
    Serde(String),
    /// The durable document was written by a newer schema.
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    /// The transaction lock already exists; operator recovery required.
    TransactionBusy(String),
    /// Compare-before-save generation mismatch.
    StaleWriter { expected: u64, actual: u64 },
    /// Generation counter exhausted.
    GenerationExhausted,
    /// The requested project is not in the registry.
    NotFound { project_id: String },
    /// The requested authorized root is not in the registry.
    RootNotFound { root_id: String },
    /// The path is already registered as another project's root.
    RootAlreadyRegistered { project_id: String, path: String },
    /// The path is registered to a project whose identity is no longer
    /// provable there (folder replaced/moved). The desktop must offer an
    /// explicit choice: relink that project elsewhere, remove it from
    /// the registry, or cancel — never a silent re-create.
    PathClaimedByUnprovenProject { project_id: String, path: String },
    /// A candidate root overlaps an existing authorized root.
    OverlappingRoots { path: String },
    /// Relink was requested but the current root is still healthy.
    RelinkUnnecessary { path: String },
    /// The request path resolves outside every authorized root.
    OutsideProjectRoots { requested: String },
    /// The record violates structural invariants.
    InvalidRecord(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootMissing { path } => write!(f, "project root does not exist: {path}"),
            Self::RootNotADirectory { path } => write!(f, "project root is not a directory: {path}"),
            Self::IdentityUnproven { path } => write!(
                f,
                "cannot prove {path} is the same project resource; deliberate relink required"
            ),
            Self::Io(e) => write!(f, "project io error: {e}"),
            Self::Serde(e) => write!(f, "project state serialization error: {e}"),
            Self::UnsupportedSchemaVersion { found, supported } => {
                write!(f, "project state schema version {found} unsupported (supported: {supported})")
            }
            Self::TransactionBusy(path) => write!(
                f,
                "project state transaction lock already held at {path}; operator recovery required"
            ),
            Self::StaleWriter { expected, actual } => {
                write!(f, "stale project writer: expected generation {expected}, found {actual}")
            }
            Self::GenerationExhausted => write!(f, "project state generation exhausted"),
            Self::NotFound { project_id } => write!(f, "project not found: {project_id}"),
            Self::RootNotFound { root_id } => write!(f, "authorized root not found: {root_id}"),
            Self::RootAlreadyRegistered { project_id, path } => {
                write!(f, "root {path} already registered by project {project_id}")
            }
            Self::PathClaimedByUnprovenProject { project_id, path } => write!(
                f,
                "path {path} is registered as project {project_id}, but its identity can no longer be proven there; relink or remove that project first"
            ),
            Self::OverlappingRoots { path } => {
                write!(f, "candidate root {path} overlaps an existing authorized root")
            }
            Self::RelinkUnnecessary { path } => {
                write!(f, "relink unnecessary: root {path} is still healthy")
            }
            Self::OutsideProjectRoots { requested } => {
                write!(f, "path resolves outside every authorized project root: {requested}")
            }
            Self::InvalidRecord(why) => write!(f, "invalid project record: {why}"),
        }
    }
}

impl std::error::Error for ProjectError {}
