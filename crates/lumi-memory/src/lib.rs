//! Context, memory, and retrieval separation (spec 10).
//!
//! Five context classes that MUST NOT be treated as interchangeable:
//!
//! 1. WORKING_CONTEXT — short-lived task material (this crate:
//!    [`WorkingContext`], dropped with the task).
//! 2. DURABLE_MEMORY — explicitly retained facts with owner, purpose,
//!    provenance, retention, and deletion behavior ([`MemoryStore`]).
//! 3. WORKFLOW_STATE — operational state; lives in `lumi-state`.
//! 4. EVIDENCE_HISTORY — accountability data; lives in `lumi-audit` and
//!    MUST NOT automatically enter model memory (no implicit promotion —
//!    tested).
//! 5. RETRIEVAL_INDEX — searchable metadata with tenant boundaries,
//!    deletion propagation, and sensitivity labels ([`RetrievalIndex`]).
//!
//! Retrieved content is DATA, not authority: retrieval results carry an
//! untrusted marker so agent loops treat them like any other content
//! (§10.8). Secrets must never be stored as memory: values of type
//! `SecretRef`-shaped or marked do-not-persist are refused at write time
//! (§10.10).

pub mod compaction;
pub mod memory;
pub mod retrieval;
pub mod working;

pub use compaction::compact;
pub use memory::{
    MemoryDeletion, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope, MemoryStore,
    RetentionDuration,
};
pub use retrieval::{IndexEntry, RetrievalIndex, RetrievalResult};
pub use working::WorkingContext;
