//! Native desktop executor: Lumi-owned DesktopDriver contract with Cua
//! as the initial upstream (spec 07).
//!
//! Architecture invariants:
//!
//! - Workflow definitions never call upstream tool names; they describe
//!   **semantic targets** (application/window/role/name), never
//!   coordinates (§7.2, §7.4).
//! - OS/API success is not success: DELIVERED requires the declared
//!   effect oracle to observe the intended state change (§7.15).
//! - Sessions/handles are bound to a runtime generation; stale handles
//!   fail closed (§7.18).
//! - No silent fallback from semantic delivery to global input (§7.20):
//!   escalation requires an explicit execution preference and a fresh
//!   policy check.
//! - The bundled upstream binary is pinned: exact version, checksum,
//!   license, update owner (§7.10) — never "latest".

pub mod cua;
pub mod driver;
pub mod executor;
pub mod session;
pub mod target;

pub use cua::{CuaUpstreamConfig, PinnedBinary};
pub use driver::{DesktopDriver, EffectOracle, NativeOperation, NativeOutcome, RefusalReason};
pub use executor::NativeExecutor;
pub use session::{
    PermissionRequirement, PermissionState, RuntimeGeneration, SessionHandle, SessionState,
};
pub use target::{DeliveryMode, SemanticTarget};
