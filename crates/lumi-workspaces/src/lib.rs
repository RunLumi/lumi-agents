//! Controlled local files, sandboxed shell, and artifacts (spec 08).
//!
//! Every task using local files or shell operates inside an explicit
//! workspace root (§8.2). Paths are canonically resolved and traversal or
//! symlink escapes fail closed (§8.4). Destructive file actions are
//! reversible by default — permanent deletion is DESTRUCTIVE risk (§8.5).
//! Shell execution runs with a cleared environment (allowlist only), a
//! deadline, bounded output capture, and an honestly-declared isolation
//! class (§8.6–8.7): v1 ships HOST_BOUNDED execution and refuses
//! workflow policies that demand stronger isolation rather than
//! pretending. Artifacts carry provenance, checksums, validations, and a
//! lifecycle where publication is strictly separated from generation
//! (§8.11–8.13).

pub mod artifacts;
pub mod files;
pub mod paths;
pub mod shell;
pub mod workspace;

pub use artifacts::{
    ArtifactError, ArtifactLifecycle, ArtifactProvenance, ArtifactRecord, ArtifactStore,
    BuiltInValidator, ValidationOutcome,
};
pub use files::{FileOp, FileOpError, FileOpRecord, WorkspaceFiles};
pub use paths::{resolve_in_workspace, PathError};
pub use shell::{
    run_spec, IsolationClass, NetworkPolicy, ShellOutcome, ShellSandbox, ShellSpec, ShellStatus,
};
pub use workspace::{CleanupPolicy, Workspace, WorkspaceMetadata};
