//! Local policy gate: authority for what may execute (spec 04).
//!
//! Cloud/model output is advisory until this layer permits it. Policy
//! evaluates *business effects*, never UI gestures. The engine is layered:
//! product hard safety rules first (non-overridable), then organization,
//! workflow, and user/session policy. More specific layers may only narrow,
//! never weaken, hard safety.

pub mod approval;
pub mod capability;
pub mod engine;

pub use approval::{
    Approval, ApprovalConstraints, ApprovalInvalidReason, ApprovalLedger, ApprovalTtl,
    ApprovalValidation,
};
pub use capability::{
    CapabilityCheck, CapabilityGrant, CapabilityRegistry, GrantSource, ResourceScope,
};
pub use engine::{
    evaluate, Decision, DenyReason, DeviceExecutionState, PolicyContext, DEFAULT_RULE_ID,
};
