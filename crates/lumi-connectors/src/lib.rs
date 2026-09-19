//! Connector & extension manifests with load-bearing policy (spec 14).
//!
//! Third-party integrations are capability providers, NEVER authorities
//! (§14.4): they cannot grant capabilities, modify policy, read
//! arbitrary secrets, bypass audit, or approve actions. The manifest is
//! a *load-bearing policy input*:
//!
//! - side-effecting operations normalize to ActionProposals and pass
//!   policy (§14.6);
//! - secret resolution is scoped to the refs the manifest declares
//!   (§14.5) — enforced at the broker call site, not by convention;
//! - network destinations are declared; unexpected targets are denied
//!   by policy (§14.8);
//! - a version that ADDS capabilities requires explicit re-review
//!   (§14.9: updates never silently widen the surface);
//! - organization policy is a CEILING (§14.16): project/user extensions
//!   cannot widen it.

pub mod manifest;
pub mod registry;
pub mod scoped_secrets;

pub use manifest::{
    ConnectorManifest, DataCategory, FilesystemScope, IntegrationKind, ManifestError,
    NetworkDestination, SideEffectDeclaration,
};
pub use registry::{
    ConnectorRecord, ExtensionRegistry, OrgPolicyCeiling, RegistryError, ReviewState,
};
pub use scoped_secrets::{scoped_broker, ScopedSecretBackend};
