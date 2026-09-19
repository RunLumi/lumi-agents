//! UX, handoff & control contracts (spec 23) + control-plane boundary
//! (spec 19).
//!
//! The UI is a VIEWPORT onto the orchestrator's durable state — it does
//! not own policy, credentials, or executor state. These types define
//! what the UI shows and what operations the user can perform.
//!
//! Trust language (§23.9): planned / attempted / executed / verified /
//! ambiguous are DISTINCT. Never collapse "attempted" into "done".

pub mod approval;
pub mod completion;
pub mod control_plane;
pub mod exception;
pub mod progress;
pub mod task_view;
pub mod trust;

pub use approval::ApprovalCard;
pub use completion::CompletionSummary;
pub use control_plane::{
    ControlPlaneBoundary, DeviceSyncMessage, PolicyDistribution, SyncConflictPolicy, TaskDispatch,
};
pub use exception::ExceptionCard;
pub use progress::{ProgressStep, ProgressView};
pub use task_view::{TaskCreationRequest, TaskPhase};
pub use trust::TrustLanguage;
