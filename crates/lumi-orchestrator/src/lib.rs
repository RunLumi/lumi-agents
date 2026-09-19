//! The local execution orchestrator: the vertical-slice runtime.
//!
//! Wires the full invariant end to end (AGENTS.md):
//!
//! ```text
//! intent -> task -> normalized ActionProposal
//!   -> policy (deny-by-default capability gate)
//!   -> scoped approval when required (digest-bound, single-use)
//!   -> pre-action checkpoint + side-effect journal
//!   -> executor
//!   -> postcondition verification
//!   -> audit event with evidence refs
//!   -> verified result / ambiguity / exception
//! ```
//!
//! Executors receive already-authorized actions only. "Done" is never
//! self-reported: a step is verified-successful only when its
//! postconditions pass through the audit crate's verifier.

pub mod orch;
pub mod router;

pub use orch::{Orchestrator, OrchestratorConfig, StepOutcome};
pub use router::{
    select, ExecutorDescriptor, ExecutorHealth, Reliability, Selection, SelectionError, TierPolicy,
};
