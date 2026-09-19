//! Durable task/run state, checkpoints, resume, cancellation, and the
//! retry/recovery controller (specs 02 and 18).
//!
//! Long-running work MUST survive app restarts, network loss, provider
//! failures, approval delays, and crashes without repeating consequential
//! side effects. This crate owns:
//!
//! - the validated task/run state machines (spec 02 §2.2–2.4);
//! - durable checkpoints including pre-action checkpoints (§2.5–2.6);
//! - a side-effect journal so recovery can determine whether a possibly
//!   completed action actually took effect before any retry (§2.7, §18.4);
//! - cooperative cancellation and pause (§2.9–2.10);
//! - bounded retry classification, budgets, and backoff (§2.11–2.12);
//! - the resume procedure (§2.8) and crash-recovery decisions (§18.11).

pub mod cancel;
pub mod checkpoint;
pub mod journal;
pub mod machine;
pub mod retry;
pub mod store;

pub use cancel::CancelToken;
pub use checkpoint::{Checkpoint, PreActionCheckpoint, VerifierPlan};
pub use journal::{AmbiguousResolution, SideEffectJournal, SideEffectRecord, SideEffectStatus};
pub use machine::{TaskStateMachine, TransitionError};
pub use retry::{Backoff, RetryController, RetryDecision, RetryPolicy, RetryState};
pub use store::{
    resume, side_effect_retryable, InMemoryStateStore, JsonStateStore, ResumeAction, ResumeContext,
    StateStore, StoreError,
};
