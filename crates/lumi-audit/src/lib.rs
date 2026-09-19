//! Audit, evidence, and verification (spec 11).
//!
//! The audit crate makes Lumi able to explain what happened and to prove
//! task success without collecting surveillance data:
//!
//! - [`AuditEvent`] + [`AuditLedger`]: append-only, hash-chained event log
//!   with tenant-scoped reads and JSONL persistence.
//! - [`EvidenceStore`]: minimal evidence records — redacted on append,
//!   tenant-scoped, retention-purged, screenshots only when the action
//!   explicitly required them.
//! - [`PostconditionVerifier`]: deterministic postcondition evaluation.
//!   Executor SUCCESS + verifier FAILED means the step is NOT verified
//!   successful; missing verification capability is AMBIGUOUS, never PASSED.

pub mod event;
pub mod evidence;
pub mod ledger;
pub mod verify;

pub use event::{ActionEventDetails, AuditEvent, AuditEventKind, AuditScope, PolicyOutcome};
pub use evidence::{EvidenceError, EvidenceKind, EvidenceRecord, EvidenceRequest, EvidenceStore};
pub use ledger::{AuditLedger, ChainVerification, JsonlAuditLog};
pub use verify::{
    safe_workspace_path, workspace_file_checksum, FixtureEnvironment, PostconditionVerifier,
    UnavailableEnvironment, VerificationEnvironment, VerificationOutcome, VerificationStatus,
};
