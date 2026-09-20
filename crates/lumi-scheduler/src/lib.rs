//! Scheduler, background work & triggers (spec 13).
//!
//! Bounded unattended work from schedules and events. Invariants:
//!
//! - Every trigger normalizes into a task-creation request carrying
//!   tenant, principal, template ref, payload refs (§13.10: references,
//!   not raw sensitive content), timestamp, deduplication key, and the
//!   policy context (§13.3).
//! - Duplicate triggers never create duplicate consequential work
//!   (§13.4): the deduplication key + window is enforced before any run
//!   starts.
//! - Unattended execution requires a REVOCABLE LEASE (§13.5) binding
//!   device + workflow + capability scope + expiry + budget. A revoked
//!   lease stops admission AND halts a run at its next gate check.
//! - Every background task carries deadline + model/action budget +
//!   retry budget + cancellation path + audit (§13.6).
//! - Missed schedules follow an explicit catch-up policy — default
//!   RUN_ONCE, never a burst of duplicate side effects (§13.12).
//! - Timezone/DST semantics are explicit on the schedule (§13.11).
//! - Quiet hours may suppress non-critical starts (§13.8).
//! - Device-unavailable tasks wait or reroute WITHOUT weakening
//!   locality/privacy constraints (§13.9): a local-only task can never
//!   fail over to the cloud.
//! - Approval-gated steps behave exactly as in the foreground: the task
//!   parks in WAITING_APPROVAL and no material action proceeds (§13.13).
//!
//! The scheduler itself is a TASK CREATOR, not an authority: it normalizes
//! triggers into the same task/run requests the foreground uses, and all
//! execution still flows through Orchestrator::execute_step.

pub mod cron;
pub mod lease;
pub mod scheduler;
pub mod trigger;

pub use lease::{LeaseRegistry, LeaseRevocation, UnattendedLease, DEVICE_LEASE_SERVICE};
pub use scheduler::{
    CatchUpPolicy, QuietHours, Schedule, ScheduleAdmission, ScheduleId, Scheduler, SchedulerError,
};
pub use trigger::{
    DeduplicationLedger, DeduplicationWindow, DeviceAvailability, TriggerKind, TriggerRequest,
    TriggerSource,
};
