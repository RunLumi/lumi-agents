//! Browser executor contract and Playwright worker client (spec 06).
//!
//! The worker process runs OUTSIDE the privileged policy core: it holds no
//! policy, no secret store, no approval authority (spec 06 §6.3). It
//! receives an already-authorized, closed set of operations and returns
//! normalized observations and failures. Page content is untrusted data —
//! it is never interpreted as instructions (spec 06 §6.11); this is
//! structural: operations are a closed enum, and page text only ever
//! appears inside observation payloads.
//!
//! Locator preference (spec 06 §6.5): test id → role/name → label →
//! text → CSS, encoded in [`Locator`] so workflow definitions cannot
//! reach for brittle selectors first.

pub mod executor;
pub mod protocol;
pub mod worker;

pub use executor::{browser_executor_error, BrowserExecutor};
pub use protocol::{
    BrowserOp, ExpectTarget, Locator, ProfileMode, ReadResult, SubmitResult, TargetSelector,
    TracePolicy, WorkerRequest, WorkerResponse, WorkerResult,
};
pub use worker::{BrowserWorkerConfig, BrowserWorkerHandle, WorkerError};
