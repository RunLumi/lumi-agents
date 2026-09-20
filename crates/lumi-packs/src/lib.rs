//! Workflow packs: reusable production automation units (spec 12).
//!
//! A pack is a versioned automation product: manifest, schemas, step
//! graph, policy requirements, postconditions, exception routes, evidence
//! policy, eval hooks, compatibility matrix, and economics baseline. Work
//! mode discovers; workflow mode (packs) hardens, governs, measures, and
//! repeats.
//!
//! Isolation (spec 12 §12.13): customer data and secrets never live in a
//! pack. Steps carry argument *templates* with `$input.field` references
//! resolved at run time from schema-validated inputs; credentials appear
//! only as [`lumi_protocol::SecretRef`]s resolved by the runtime.

pub mod admission;

pub mod manifest;
pub mod role;
pub mod runner;
pub mod step;

pub use manifest::{
    CompatibilityMatrix, EconomicBaseline, PackManifest, PackStatus, PrivacyClassification,
    SchemaField, SchemaFieldSet, ValueSchema,
};
pub use role::{
    RoleAuthorityCeiling, RolePack, RoleQueue, RoleWorkItem, RoleWorkItemField, RoleWorkItemSchema,
    WorkflowPackRef, ROLE_SCHEMA_NAME, ROLE_SCHEMA_VERSION,
};
pub use runner::{prepare_run, resolve_value, PackRunError, PreparedPackRun};
pub use step::{ApprovalRule, ExceptionRoute, ExceptionTarget, PackStep, WorkflowPack};
