//! Desktop shell backend API (spec 15).
//!
//! The desktop app is a viewport onto the orchestrator's durable state.
//! It does NOT own policy, credentials, or executor state. Every
//! operation exposed here maps to an existing orchestrator/policy/audit
//! call — there is no parallel authority path.
//!
//! The Tauri frontend calls these as IPC commands. The backend keeps no
//! UI state: all state comes from the orchestrator, ledger, and store.

pub mod api;
pub mod browser_tools;
pub mod chrome_tools;
pub mod computer_tools;
pub mod connections;
pub mod engagement;
pub mod engagement_runner;
pub mod permissions;
pub mod projects;
pub mod provider_storage;
pub mod runner;
pub mod runtime;

pub use api::{
    ConnectionState, DesktopBackend, EconomicsSummary, EvidenceSummaryEntry, ExecutionState,
    IssuedApproval, KillSwitchState, OperationsSnapshot, PendingApproval,
};
pub use connections::{
    connect_connection, disconnect_connection, list_connections, ConnectionRecord, ConnectionsStore,
};
pub use permissions::{PermissionCheckResult, PermissionState, PermissionStatus};
pub use projects::{
    FileContent, FileContentBase64, MemorySubmission, OpenedProject, ProjectOverview,
    ProjectService, ProjectSummary,
};
pub use runner::{
    gated_file_save, is_runnable, run_desktop_task, run_desktop_task_with_provider, FileSaveOp,
    ProviderSession,
};
pub use runtime::{DesktopRuntime, ProjectTaskSpec};
