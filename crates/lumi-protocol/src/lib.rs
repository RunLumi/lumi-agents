//! Stable vocabulary shared by the Lumi planner, policy engine, executors,
//! verifiers, audit ledger, workflow packs, and control-plane sync.
//!
//! This crate implements [spec 01] (core domain model) and [spec 03]
//! (action/observation protocol). It is deliberately dependency-light and
//! contains no execution, authorization, or I/O logic: models may *propose*
//! values from this vocabulary, but nothing here grants authority.
//!
//! Normative invariants enforced by this crate:
//!
//! - Persisted protocol messages carry `schema_name` / `schema_version` and
//!   unknown versions or enum values fail explicitly (never coerce).
//! - Timestamps are UTC RFC 3339.
//! - IDs are opaque strings and never encode secrets.
//! - Secret values are referenced, never embedded ([`SecretRef`]).
//! - Payloads support field-level redaction before egress or long-term audit.
//! - Material action changes are detectable via [`ActionProposal::material_digest`].
//!
//! [spec 01]: ../../docs/specs/v1/01-core-domain-model.md
//! [spec 03]: ../../docs/specs/v1/03-action-observation-protocol.md

pub mod action;
pub mod artifact;
pub mod budget;
pub mod canonical;
pub mod device;
pub mod error;
pub mod evidence;
pub mod ids;
pub mod observation;
pub mod postcondition;
pub mod principal;
pub mod project;
pub mod redaction;
pub mod resource;
pub mod result;
pub mod risk;
pub mod run;
pub mod schema;
pub mod task;
pub mod tenant;
pub mod tier;
pub mod timestamp;

pub use action::{
    ActionProposal, ExecutionPreferences, ExpectedEffect, Idempotency, IdempotencySemantics, Target,
};
pub use artifact::{
    Artifact, ArtifactProvenance, ArtifactType, PublicationState, ValidationStatus,
};
pub use budget::{Budget, ConsumedBudget};
pub use device::{Device, Platform, TrustState, UpdateRing};
pub use error::{ErrorEnvelope, FailureCategory, RecoveryAction, RetryClass};
pub use evidence::{EvidenceRef, EvidenceRequirement};
pub use ids::{
    ActionId, ApprovalId, ArtifactId, ConnectorInstanceId, DeviceId, EnvironmentId, EvidenceId,
    OrganizationId, PrincipalId, ProjectId, ProviderRequestId, RunId, StepId, TaskId, TenantId,
    UserId, WorkflowId, WorkflowVersion,
};
pub use observation::{
    Confidence, ConfidenceKind, Observation, ObservationKind, ObservationSource,
};
pub use postcondition::{Postcondition, PostconditionCheck, PostconditionId};
pub use principal::{AuthenticationStrength, Principal, PrincipalKind};
pub use project::{ProjectTaskBinding, WorkspaceKind};
pub use redaction::{redact_value, RedactionRule, SecretRef, REDACTED_MARKER};
pub use resource::Capability;
pub use resource::{capabilities, Resource, ResourceRef, ResourceType, SensitivityLabel};
pub use result::{ExecutionResult, ExecutionStatus, Grounding};
pub use risk::RiskClass;
pub use run::{Run, RunState};
pub use schema::{schema_names, ProtocolError, PROTOCOL_VERSION};
pub use task::{PrivacyConstraint, Task, TaskMode, TaskStatus};
pub use tenant::{DataEgressPolicy, RetentionPolicy, Tenant};
pub use tier::{ExecutionTier, ObservationSurface};
pub use timestamp::Timestamp;
