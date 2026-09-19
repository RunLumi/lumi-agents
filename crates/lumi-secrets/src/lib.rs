//! Secrets broker: OS-keychain-backed secret storage with zeroize-on-drop
//! values and audit hooks (AGENTS.md secrets rules, spec 08/17 context).
//!
//! Invariants:
//!
//! - Secret VALUES never appear in prompts, audit logs, traces,
//!   screenshots, or fixtures. Only [`SecretRef`] names do.
//! - [`SecretValue`] redacts its `Debug`/`Display` output and zeroizes
//!   its memory on drop, so a value living beyond a call is a compile-time
//!   visible `expose()` call — and a dropped one is zeroed memory.
//! - Every broker operation records an audit event containing the
//!   reference name, operation, outcome, and purpose — never the value.
//! - Deletion (revocation) removes the secret; later resolutions fail.
//!
//! The OS backend uses the platform keychain (macOS Keychain /
//! Windows Credential Manager) via the `keyring` crate on those
//! platforms; tests use an in-memory backend with the same semantics.

pub mod broker;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod keyring_backend;
pub mod memory_backend;
pub mod value;

pub use broker::{
    SecretAuditEvent, SecretAuditOperation, SecretBackend, SecretBroker, SecretError,
};
pub use memory_backend::InMemorySecretBackend;
pub use value::SecretValue;

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use keyring_backend::KeyringBackend;
