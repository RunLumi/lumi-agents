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
pub mod permissions;
pub mod projects;
pub mod runtime;

pub use api::{
    ConnectionState, DesktopBackend, EconomicsSummary, EvidenceSummaryEntry, ExecutionState,
    IssuedApproval, KillSwitchState, OperationsSnapshot, PendingApproval,
};
pub use permissions::{PermissionCheckResult, PermissionState, PermissionStatus};
pub use projects::{FileContent, OpenedProject, ProjectOverview, ProjectService, ProjectSummary};
pub use runtime::DesktopRuntime;
