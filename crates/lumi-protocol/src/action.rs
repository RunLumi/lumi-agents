//! The normalized ActionProposal (spec 03 §3.2).
//!
//! The policy unit is the business effect (`send_customer_email`), never a
//! UI gesture (`click(x=823, y=418)`). Raw gestures MAY exist inside
//! executor-specific plans but never as organization-level policy units.

use crate::canonical::sha256_canonical;
use crate::evidence::EvidenceRequirement;
use crate::ids::{ActionId, RunId, StepId, TaskId, WorkflowId};
use crate::postcondition::Postcondition;
use crate::principal::Principal;
use crate::resource::Capability;
use crate::resource::ResourceRef;
use crate::risk::RiskClass;
use crate::schema::{schema_names, Envelope, ProtocolError};
use crate::tier::ExecutionTier;
use serde::{Deserialize, Serialize};

/// Where an action sends its effect, when distinct from the resource being
/// acted on (e.g. export destination, email recipient).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// Canonical form used for policy matching and digests.
    pub canonical: String,
    /// Human-readable form for approval UX; never used for matching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

impl Target {
    #[must_use]
    pub fn canonical(canonical: impl Into<String>) -> Self {
        Self {
            canonical: canonical.into(),
            display: None,
        }
    }
}

/// What the action is expected to do in the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedEffect {
    /// One-line business-level description.
    pub summary: String,
    /// Will anyone outside the device be able to observe this?
    pub external_visibility: bool,
    /// Can the effect be undone by a defined compensating action?
    pub reversible: bool,
}

/// Idempotency metadata (spec 03 §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Idempotency {
    /// Client-chosen key when semantics require one. MUST be stable across
    /// retries of the same logical action and unique per logical action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub semantics: IdempotencySemantics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdempotencySemantics {
    /// No idempotency guarantee: execution MUST be treated as possibly
    /// ambiguous after any failure.
    #[default]
    None,
    /// The executor sends a client key the remote system deduplicates on.
    ClientKey,
    /// The remote system returns its own operation key to track.
    RemoteKey,
    /// Before any retry, verify the external postcondition.
    VerifyBeforeRetry,
}

/// Execution-tier preferences (spec 03 §3.2, spec 05 §5.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ExecutionPreferences {
    /// Tiers this action MAY be executed on. Empty means the router
    /// chooses among all tiers the capability supports.
    #[serde(default)]
    pub allowed_tiers: Vec<ExecutionTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_tier: Option<ExecutionTier>,
}

/// Raw wire shape of an [`ActionProposal`] including the envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionProposalRaw {
    schema_name: String,
    schema_version: String,
    action_id: ActionId,
    task_id: TaskId,
    run_id: RunId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workflow_id: Option<WorkflowId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    step_id: Option<StepId>,
    principal: Principal,
    capability: Capability,
    resource: ResourceRef,
    target: Target,
    operation: String,
    arguments: serde_json::Value,
    expected_effect: ExpectedEffect,
    risk_class: RiskClass,
    #[serde(default)]
    execution_preferences: ExecutionPreferences,
    #[serde(default)]
    evidence_requirements: Vec<EvidenceRequirement>,
    #[serde(default)]
    postconditions: Vec<Postcondition>,
    #[serde(default)]
    idempotency: Idempotency,
    timeout_ms: u64,
}

/// A normalized, policy-addressable action proposal.
///
/// Once approved, material fields MUST NOT change (spec 03 §3.3); material
/// changes are detected via [`ActionProposal::material_digest`], and any
/// change requires fresh policy evaluation and approval.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActionProposal {
    pub action_id: ActionId,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub workflow_id: Option<WorkflowId>,
    pub step_id: Option<StepId>,
    pub principal: Principal,
    pub capability: Capability,
    pub resource: ResourceRef,
    pub target: Target,
    pub operation: String,
    /// Must be a JSON object.
    pub arguments: serde_json::Value,
    pub expected_effect: ExpectedEffect,
    pub risk_class: RiskClass,
    pub execution_preferences: ExecutionPreferences,
    pub evidence_requirements: Vec<EvidenceRequirement>,
    pub postconditions: Vec<Postcondition>,
    pub idempotency: Idempotency,
    pub timeout_ms: u64,
}

impl ActionProposal {
    /// Envelope carried by serialized forms.
    #[must_use]
    pub fn envelope() -> Envelope {
        Envelope::new(schema_names::ACTION)
    }

    /// Validates structural invariants beyond types.
    ///
    /// # Errors
    /// [`ProtocolError::Malformed`] for non-object arguments, empty
    /// operation/target, empty allowed tiers, or zero timeout.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if !self.arguments.is_object() {
            return Err(ProtocolError::Malformed(
                "arguments must be a JSON object".to_owned(),
            ));
        }
        if self.operation.trim().is_empty() {
            return Err(ProtocolError::Malformed(
                "operation must not be empty".to_owned(),
            ));
        }
        if self.target.canonical.trim().is_empty() {
            return Err(ProtocolError::Malformed(
                "target.canonical must not be empty".to_owned(),
            ));
        }
        if self.timeout_ms == 0 {
            return Err(ProtocolError::Malformed(
                "timeout_ms must be positive".to_owned(),
            ));
        }
        if self.idempotency.semantics == IdempotencySemantics::ClientKey
            && self
                .idempotency
                .key
                .as_deref()
                .is_none_or(|key| key.trim().is_empty())
        {
            return Err(ProtocolError::Malformed(
                "CLIENT_KEY requires a nonempty idempotency key".to_owned(),
            ));
        }
        if !self.execution_preferences.allowed_tiers.is_empty()
            && self
                .execution_preferences
                .preferred_tier
                .is_some_and(|p| !self.execution_preferences.allowed_tiers.contains(&p))
        {
            return Err(ProtocolError::Malformed(
                "preferred_tier must be within allowed_tiers".to_owned(),
            ));
        }
        Ok(())
    }

    /// Digest over all material fields (spec 03 §3.3, spec 04 §4.7).
    ///
    /// Algorithm `lumi-action-digest/v3`: SHA-256 over canonical JSON of
    /// the material projection — capability, resource, target, operation,
    /// arguments, risk, sensitivity, principal identity, idempotency, and
    /// the reversibility/visibility and verification/evidence requirements.
    /// Deliberately *excludes*: execution tier preferences, timeout,
    /// and ids (changing executor tier does
    /// not change the approved business action; changing what it does does).
    #[must_use]
    pub fn material_digest(&self) -> String {
        let material = serde_json::json!({
            "digest_version": "lumi-action-digest/v3",
            "principal": {
                "tenant_id": self.principal.tenant_id,
                "principal_id": self.principal.principal_id,
                "kind": self.principal.kind,
            },
            "capability": self.capability.0,
            "resource": {
                "type": self.resource.resource_type.0,
                "id": self.resource.id,
                "sensitivity": self.resource.sensitivity,
            },
            "risk_class": self.risk_class,
            "idempotency": self.idempotency,
            "postconditions": self.postconditions,
            "evidence_requirements": self.evidence_requirements,
            "target": self.target.canonical,
            "operation": self.operation,
            "arguments": self.arguments,
            "reversible": self.expected_effect.reversible,
            "external_visibility": self.expected_effect.external_visibility,
        });
        sha256_canonical(&material)
    }

    /// Detects material mutation relative to a previously-approved digest
    /// (spec 03 §3.12).
    #[must_use]
    pub fn material_matches_digest(&self, digest: &str) -> bool {
        self.material_digest() == digest
    }

    /// True when this action's effects can be observed outside the device.
    #[must_use]
    pub const fn is_external(&self) -> bool {
        self.expected_effect.external_visibility || self.risk_class.is_consequential()
    }

    /// Serializes with the protocol envelope.
    ///
    /// # Errors
    /// Propagates [`ProtocolError::Malformed`] from [`Self::validate`].
    pub fn to_json(&self) -> Result<String, ProtocolError> {
        self.validate()?;
        let raw = ActionProposalRaw::from_owned(self);
        serde_json::to_string(&raw).map_err(|e| ProtocolError::Malformed(e.to_string()))
    }

    /// Deserializes with strict envelope and invariant checks.
    ///
    /// # Errors
    /// Fails closed on unknown schema versions, unknown enum values, or
    /// invariant violations.
    pub fn from_json(json: &str) -> Result<Self, ProtocolError> {
        let raw: ActionProposalRaw =
            serde_json::from_str(json).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
        Envelope {
            schema_name: raw.schema_name.clone(),
            schema_version: raw.schema_version.clone(),
        }
        .ensure(schema_names::ACTION)?;
        let proposal = ActionProposal::from_raw(raw);
        proposal.validate()?;
        Ok(proposal)
    }

    fn from_raw(raw: ActionProposalRaw) -> Self {
        Self {
            action_id: raw.action_id,
            task_id: raw.task_id,
            run_id: raw.run_id,
            workflow_id: raw.workflow_id,
            step_id: raw.step_id,
            principal: raw.principal,
            capability: raw.capability,
            resource: raw.resource,
            target: raw.target,
            operation: raw.operation,
            arguments: raw.arguments,
            expected_effect: raw.expected_effect,
            risk_class: raw.risk_class,
            execution_preferences: raw.execution_preferences,
            evidence_requirements: raw.evidence_requirements,
            postconditions: raw.postconditions,
            idempotency: raw.idempotency,
            timeout_ms: raw.timeout_ms,
        }
    }

    /// Convenience constructor for fixture/test/workflow builders.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn builder(
        action_id: impl Into<ActionId>,
        task_id: impl Into<TaskId>,
        run_id: impl Into<RunId>,
        principal: Principal,
        capability: Capability,
        resource: ResourceRef,
        target: Target,
        operation: impl Into<String>,
        risk_class: RiskClass,
    ) -> ActionProposalBuilder {
        ActionProposalBuilder {
            inner: Self {
                action_id: action_id.into(),
                task_id: task_id.into(),
                run_id: run_id.into(),
                workflow_id: None,
                step_id: None,
                principal,
                capability,
                resource,
                target,
                operation: operation.into(),
                arguments: serde_json::json!({}),
                expected_effect: ExpectedEffect {
                    summary: String::new(),
                    external_visibility: risk_class.is_consequential(),
                    reversible: false,
                },
                risk_class,
                execution_preferences: ExecutionPreferences::default(),
                evidence_requirements: Vec::new(),
                postconditions: Vec::new(),
                idempotency: Idempotency::default(),
                timeout_ms: 30_000,
            },
        }
    }
}

/// Fluent builder produced by [`ActionProposal::builder`].
pub struct ActionProposalBuilder {
    inner: ActionProposal,
}

impl ActionProposalBuilder {
    #[must_use]
    pub fn arguments(mut self, arguments: serde_json::Value) -> Self {
        self.inner.arguments = arguments;
        self
    }

    #[must_use]
    pub fn expected_effect(mut self, effect: ExpectedEffect) -> Self {
        self.inner.expected_effect = effect;
        self
    }

    #[must_use]
    pub fn execution_preferences(mut self, prefs: ExecutionPreferences) -> Self {
        self.inner.execution_preferences = prefs;
        self
    }

    #[must_use]
    pub fn evidence_requirements(mut self, requirements: Vec<EvidenceRequirement>) -> Self {
        self.inner.evidence_requirements = requirements;
        self
    }

    #[must_use]
    pub fn postconditions(mut self, postconditions: Vec<Postcondition>) -> Self {
        self.inner.postconditions = postconditions;
        self
    }

    #[must_use]
    pub fn idempotency(mut self, idempotency: Idempotency) -> Self {
        self.inner.idempotency = idempotency;
        self
    }

    #[must_use]
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.inner.timeout_ms = timeout_ms;
        self
    }

    #[must_use]
    pub fn workflow_id(mut self, workflow_id: WorkflowId) -> Self {
        self.inner.workflow_id = Some(workflow_id);
        self
    }

    #[must_use]
    pub fn step_id(mut self, step_id: StepId) -> Self {
        self.inner.step_id = Some(step_id);
        self
    }

    /// Builds and validates.
    ///
    /// # Errors
    /// Propagates [`ActionProposal::validate`] errors.
    pub fn build(self) -> Result<ActionProposal, ProtocolError> {
        self.inner.validate()?;
        Ok(self.inner)
    }

    /// Panicking variant for fixtures/tests.
    #[must_use]
    pub fn unwrap(self) -> ActionProposal {
        match self.build() {
            Ok(p) => p,
            Err(e) => panic!("invalid action proposal fixture: {e}"),
        }
    }
}

impl ActionProposalRaw {
    fn from_owned(p: &ActionProposal) -> Self {
        Self {
            schema_name: schema_names::ACTION.to_owned(),
            schema_version: crate::schema::PROTOCOL_VERSION.to_owned(),
            action_id: p.action_id.clone(),
            task_id: p.task_id.clone(),
            run_id: p.run_id.clone(),
            workflow_id: p.workflow_id.clone(),
            step_id: p.step_id.clone(),
            principal: p.principal.clone(),
            capability: p.capability.clone(),
            resource: p.resource.clone(),
            target: p.target.clone(),
            operation: p.operation.clone(),
            arguments: p.arguments.clone(),
            expected_effect: p.expected_effect.clone(),
            risk_class: p.risk_class,
            execution_preferences: p.execution_preferences.clone(),
            evidence_requirements: p.evidence_requirements.clone(),
            postconditions: p.postconditions.clone(),
            idempotency: p.idempotency.clone(),
            timeout_ms: p.timeout_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities;
    use crate::ids::{PrincipalId, TenantId};
    use crate::Timestamp;

    fn principal() -> Principal {
        Principal {
            principal_id: PrincipalId::parse("u-1").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            kind: PrincipalKind::User,
            authenticated_at: Some(Timestamp::UNIX_EPOCH),
            authentication_strength: Some(AuthenticationStrength::Mfa),
        }
    }

    use crate::principal::{AuthenticationStrength, PrincipalKind};

    fn email_action(tier: ExecutionTier) -> ActionProposal {
        ActionProposal::builder(
            ActionId::parse("a-1").unwrap(),
            TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            principal(),
            Capability::well_known(capabilities::EMAIL_SEND),
            ResourceRef {
                resource_type: crate::resource::ResourceType::well_known(
                    crate::resource::ResourceType::EMAIL_MESSAGE,
                ),
                id: "outbound/customer-reply".to_owned(),
                sensitivity: Some(crate::resource::SensitivityLabel::Confidential),
            },
            Target::canonical("mailto:customer@example.test"),
            "send_customer_email",
            RiskClass::Communication,
        )
        .arguments(serde_json::json!({
            "subject": "Your quote",
            "body": "Hello!",
        }))
        .execution_preferences(ExecutionPreferences {
            allowed_tiers: vec![tier, ExecutionTier::BrowserSemantic],
            preferred_tier: Some(tier),
        })
        .unwrap()
    }

    #[test]
    fn material_digest_ignores_tier_but_detects_business_change() {
        let via_connector = email_action(ExecutionTier::ConnectorApi);
        let via_native = ActionProposal {
            execution_preferences: ExecutionPreferences {
                allowed_tiers: vec![ExecutionTier::NativeSemantic],
                preferred_tier: Some(ExecutionTier::NativeSemantic),
            },
            ..email_action(ExecutionTier::NativeSemantic)
        };
        // Tier change with identical business fields keeps the approval
        // valid: policy sees the identical normalized action.
        assert_eq!(
            via_connector.material_digest(),
            via_native.material_digest()
        );

        let wrong_target = ActionProposal {
            target: Target::canonical("mailto:someone-else@example.test"),
            ..email_action(ExecutionTier::ConnectorApi)
        };
        assert_ne!(
            via_connector.material_digest(),
            wrong_target.material_digest(),
            "target mutation must invalidate the digest"
        );
    }

    #[test]
    fn json_round_trip_carries_envelope() {
        let action = email_action(ExecutionTier::ConnectorApi);
        let json = action.to_json().unwrap();
        assert!(json.contains("\"schema_name\":\"lumi.action\""), "{json}");
        assert!(json.contains("\"schema_version\":\"1.0\""));
        let back = ActionProposal::from_json(&json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn unknown_schema_version_fails_closed() {
        let action = email_action(ExecutionTier::ConnectorApi);
        let json = action.to_json().unwrap();
        let mutated = json.replace("\"schema_version\":\"1.0\"", "\"schema_version\":\"9.9\"");
        let err = ActionProposal::from_json(&mutated).unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnsupportedSchemaVersion { .. }
        ));
    }

    #[test]
    fn unknown_risk_class_fails_explicitly() {
        let action = email_action(ExecutionTier::ConnectorApi);
        let json = action.to_json().unwrap();
        let mutated = json.replace("COMMUNICATION", "TELEPORTATION");
        assert!(ActionProposal::from_json(&mutated).is_err());
    }

    #[test]
    fn non_object_arguments_are_malformed() {
        let action = ActionProposal {
            arguments: serde_json::json!("not-an-object"),
            ..email_action(ExecutionTier::ConnectorApi)
        };
        assert!(matches!(
            action.validate(),
            Err(ProtocolError::Malformed(_))
        ));
    }

    #[test]
    fn preferred_tier_must_be_allowed() {
        let action = ActionProposal {
            execution_preferences: ExecutionPreferences {
                allowed_tiers: vec![ExecutionTier::ConnectorApi],
                preferred_tier: Some(ExecutionTier::Vision),
            },
            ..email_action(ExecutionTier::ConnectorApi)
        };
        assert!(action.validate().is_err());
    }

    #[test]
    fn external_actions_are_flagged() {
        let email = email_action(ExecutionTier::ConnectorApi);
        assert!(email.is_external());
        let read = ActionProposal::builder(
            ActionId::parse("a-2").unwrap(),
            TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            principal(),
            Capability::well_known(capabilities::BROWSER_READ),
            ResourceRef {
                resource_type: crate::resource::ResourceType::well_known(
                    crate::resource::ResourceType::BROWSER_ORIGIN,
                ),
                id: "https://example.test".to_owned(),
                sensitivity: None,
            },
            Target::canonical("https://example.test"),
            "read_page",
            RiskClass::Read,
        )
        .unwrap();
        assert!(!read.is_external());
    }
}
