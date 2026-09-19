//! Release certification checks (spec 21, issue #11).
//!
//! Every release gate is a typed check that passes or fails with
//! evidence. The release is blocked until all required checks pass.
//! These gates are load-bearing: the readiness report and CI pipeline
//! call them, not a human reading a checklist.

pub mod checks;

pub use checks::{
    run_certification, CertificationReport, CheckResult, SigningConfig, UpdaterConfig,
};
