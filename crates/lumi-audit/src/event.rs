//! Structured audit events with tamper-evident chaining (spec 11 §11.2,
//! §11.3, §11.11, §11.12).
//!
//! Every material action generates one event; the ledger is append-only
//! and each event commits to the previous one via a SHA-256 hash chain, so
//! silent history edits are detectable. Corrections are follow-up events,
//! never overwrites.

use crate::verify::VerificationStatus;
use lumi_protocol::{
    canonical::sha256_hex, schema_names, ActionId, ApprovalId, ErrorEnvelope, ExecutionStatus,
    ExecutionTier, FailureCategory, Principal, ResourceRef, RiskClass, RunId, StepId, Target,
    TaskId, TenantId, Timestamp, WorkflowId, WorkflowVersion, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};

/// The run-scoped identity every audit event carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditScope {
    pub tenant_id: TenantId,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub workflow: Option<(WorkflowId, WorkflowVersion)>,
}

/// How the action was resolved by policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PolicyOutcome {
    Allowed { rule_id: String },
    Denied { rule_id: String, reason: String },
    ApprovalRequired { rule_id: String, reason: String },
}

/// The full record of one material action attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionEventDetails {
    pub action_id: ActionId,
    pub step_id: Option<StepId>,
    pub principal: Principal,
    pub capability: String,
    pub operation: String,
    pub resource: ResourceRef,
    pub target: Target,
    pub risk_class: RiskClass,
    pub execution_tier: Option<ExecutionTier>,
    pub adapter: Option<String>,
    pub adapter_version: Option<String>,
    pub policy: PolicyOutcome,
    pub approval_id: Option<ApprovalId>,
    /// The material digest this event's action carried. Approval linkage
    /// (spec 11 §11.11) is via approval_id + digest.
    pub action_digest: String,
    pub execution_status: Option<ExecutionStatus>,
    pub verification: VerificationStatus,
    pub failure: Option<ErrorEnvelope>,
    pub failure_category: Option<FailureCategory>,
}

/// Event taxonomy. Material action events and lifecycle events share one
/// ledger so the chain covers everything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AuditEventKind {
    /// A material action attempt and everything known about its outcome.
    /// Boxed to keep the other variants cheap to move.
    Action(Box<ActionEventDetails>),
    /// A human approved a specific action digest (spec 11 §11.11).
    ApprovalGranted {
        approval_id: ApprovalId,
        approver: Principal,
        action_digest: String,
        action_id: ActionId,
    },
    /// A previously issued approval was invalidated (expiry, mutation,
    /// revocation, consumption).
    ApprovalInvalidated {
        approval_id: ApprovalId,
        action_digest: String,
        reason: String,
    },
    /// Task/run lifecycle transition.
    Lifecycle {
        run_id: RunId,
        from: String,
        to: String,
        reason: Option<String>,
    },
    /// A postcondition was evaluated (standalone verification events).
    Verified {
        action_id: Option<ActionId>,
        postcondition_id: String,
        verification: VerificationStatus,
        detail: String,
    },
}

/// Raw wire shape used for envelope checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEventRaw {
    schema_name: String,
    schema_version: String,
    event_id: String,
    timestamp: Timestamp,
    tenant_id: TenantId,
    task_id: TaskId,
    run_id: RunId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workflow_id: Option<WorkflowId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workflow_version: Option<WorkflowVersion>,
    #[serde(flatten)]
    kind: AuditEventKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    evidence_refs: Vec<String>,
    /// Hash of the previous event in the ledger (`"genesis"` for the
    /// first event).
    prev_event_hash: String,
    /// SHA-256 over the canonical JSON of all fields above.
    event_hash: String,
}

/// One immutable audit record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: Timestamp,
    pub tenant_id: TenantId,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub workflow_id: Option<WorkflowId>,
    pub workflow_version: Option<WorkflowVersion>,
    pub kind: AuditEventKind,
    pub evidence_refs: Vec<String>,
    pub prev_event_hash: String,
    pub event_hash: String,
}

impl AuditEvent {
    /// Builds an event, computing its chain hash over the canonical JSON
    /// of every field (chain hash covers payload, not just metadata).
    #[must_use]
    pub fn new(
        scope: AuditScope,
        kind: AuditEventKind,
        evidence_refs: Vec<String>,
        prev_event_hash: &str,
        timestamp: Timestamp,
    ) -> Self {
        let AuditScope {
            tenant_id,
            task_id,
            run_id,
            workflow,
        } = scope;
        let (workflow_id, workflow_version) = match workflow {
            Some((id, version)) => (Some(id), Some(version)),
            None => (None, None),
        };
        let for_hash = AuditEventRaw {
            schema_name: schema_names::AUDIT_EVENT.to_owned(),
            schema_version: PROTOCOL_VERSION.to_owned(),
            event_id: String::new(),
            timestamp,
            tenant_id: tenant_id.clone(),
            task_id: task_id.clone(),
            run_id: run_id.clone(),
            workflow_id: workflow_id.clone(),
            workflow_version: workflow_version.clone(),
            kind: kind.clone(),
            evidence_refs: evidence_refs.clone(),
            prev_event_hash: prev_event_hash.to_owned(),
            event_hash: String::new(),
        };
        let event_hash = sha256_hex(
            lumi_protocol::canonical::canonical_json(
                &serde_json::to_value(&for_hash).expect("audit event is JSON-serializable"),
            )
            .as_bytes(),
        );
        Self {
            event_id: format!("evt-{event_hash}"),
            timestamp,
            tenant_id,
            task_id,
            run_id,
            workflow_id,
            workflow_version,
            kind,
            evidence_refs,
            prev_event_hash: prev_event_hash.to_owned(),
            event_hash,
        }
    }

    /// Verifies this event's stored hash matches its content.
    #[must_use]
    pub fn hash_is_valid(&self) -> bool {
        let for_hash = AuditEventRaw {
            schema_name: schema_names::AUDIT_EVENT.to_owned(),
            schema_version: PROTOCOL_VERSION.to_owned(),
            event_id: String::new(),
            timestamp: self.timestamp,
            tenant_id: self.tenant_id.clone(),
            task_id: self.task_id.clone(),
            run_id: self.run_id.clone(),
            workflow_id: self.workflow_id.clone(),
            workflow_version: self.workflow_version.clone(),
            kind: self.kind.clone(),
            evidence_refs: self.evidence_refs.clone(),
            prev_event_hash: self.prev_event_hash.clone(),
            event_hash: String::new(),
        };
        sha256_hex(
            lumi_protocol::canonical::canonical_json(
                &serde_json::to_value(&for_hash).expect("audit event is JSON-serializable"),
            )
            .as_bytes(),
        ) == self.event_hash
    }

    /// Serializes with the protocol envelope.
    ///
    /// # Errors
    /// Serialization of an event is infallible for in-memory events;
    /// errors indicate a bug.
    pub fn to_json(&self) -> Result<String, lumi_protocol::ProtocolError> {
        let raw = AuditEventRaw {
            schema_name: schema_names::AUDIT_EVENT.to_owned(),
            schema_version: PROTOCOL_VERSION.to_owned(),
            event_id: self.event_id.clone(),
            timestamp: self.timestamp,
            tenant_id: self.tenant_id.clone(),
            task_id: self.task_id.clone(),
            run_id: self.run_id.clone(),
            workflow_id: self.workflow_id.clone(),
            workflow_version: self.workflow_version.clone(),
            kind: self.kind.clone(),
            evidence_refs: self.evidence_refs.clone(),
            prev_event_hash: self.prev_event_hash.clone(),
            event_hash: self.event_hash.clone(),
        };
        serde_json::to_string(&raw)
            .map_err(|e| lumi_protocol::ProtocolError::Malformed(e.to_string()))
    }

    /// Deserializes with envelope validation and hash verification.
    ///
    /// # Errors
    /// Fails closed on unknown schema versions or hash mismatch.
    pub fn from_json(json: &str) -> Result<Self, lumi_protocol::ProtocolError> {
        let raw: AuditEventRaw = serde_json::from_str(json)
            .map_err(|e| lumi_protocol::ProtocolError::Malformed(e.to_string()))?;
        lumi_protocol::schema::Envelope {
            schema_name: raw.schema_name.clone(),
            schema_version: raw.schema_version.clone(),
        }
        .ensure(schema_names::AUDIT_EVENT)?;
        let event = Self {
            event_id: raw.event_id,
            timestamp: raw.timestamp,
            tenant_id: raw.tenant_id,
            task_id: raw.task_id,
            run_id: raw.run_id,
            workflow_id: raw.workflow_id,
            workflow_version: raw.workflow_version,
            kind: raw.kind,
            evidence_refs: raw.evidence_refs,
            prev_event_hash: raw.prev_event_hash,
            event_hash: raw.event_hash,
        };
        if !event.hash_is_valid() {
            return Err(lumi_protocol::ProtocolError::Malformed(
                "audit event hash mismatch (content modified after append)".to_owned(),
            ));
        }
        Ok(event)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use lumi_protocol::{
        AuthenticationStrength, Capability, PrincipalId, ResourceRef, ResourceType, Target,
    };

    pub fn fixture_principal(tenant: &str) -> Principal {
        Principal {
            principal_id: PrincipalId::parse("u-fixture").unwrap(),
            tenant_id: TenantId::parse(tenant).unwrap(),
            kind: PrincipalKind::User,
            authenticated_at: Some(Timestamp::UNIX_EPOCH),
            authentication_strength: Some(AuthenticationStrength::Mfa),
        }
    }

    use lumi_protocol::PrincipalKind;

    pub fn action_event(
        tenant: &str,
        digest: &str,
        verification: VerificationStatus,
    ) -> AuditEvent {
        AuditEvent::new(
            AuditScope {
                tenant_id: TenantId::parse(tenant).unwrap(),
                task_id: TaskId::parse("task-1").unwrap(),
                run_id: RunId::parse("run-1").unwrap(),
                workflow: None,
            },
            AuditEventKind::Action(Box::new(ActionEventDetails {
                action_id: ActionId::parse("a-1").unwrap(),
                step_id: None,
                principal: fixture_principal(tenant),
                capability: Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND).0,
                operation: "send_customer_email".to_owned(),
                resource: ResourceRef {
                    resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                    id: "outbound/x".to_owned(),
                    sensitivity: None,
                },
                target: Target::canonical("mailto:c@example.test"),
                risk_class: RiskClass::Communication,
                execution_tier: Some(ExecutionTier::ConnectorApi),
                adapter: Some("http-connector/1".to_owned()),
                adapter_version: Some("1.0.0".to_owned()),
                policy: PolicyOutcome::Allowed {
                    rule_id: "lumi.policy.default/v1".to_owned(),
                },
                approval_id: None,
                action_digest: digest.to_owned(),
                execution_status: Some(ExecutionStatus::Success),
                verification,
                failure: None,
                failure_category: None,
            })),
            vec![],
            "genesis",
            Timestamp::UNIX_EPOCH,
        )
    }

    #[test]
    fn event_hash_is_deterministic_and_content_bound() {
        let e1 = action_event("t-1", "digest-a", VerificationStatus::Passed);
        let e2 = action_event("t-1", "digest-a", VerificationStatus::Passed);
        assert_eq!(e1.event_hash, e2.event_hash);
        let e3 = action_event("t-1", "digest-b", VerificationStatus::Passed);
        assert_ne!(e1.event_hash, e3.event_hash);
        assert!(e1.hash_is_valid());
    }

    #[test]
    fn json_round_trip_rejects_content_modification() {
        let event = action_event("t-1", "digest-a", VerificationStatus::Passed);
        let json = event.to_json().unwrap();
        assert!(
            json.contains("\"schema_name\":\"lumi.audit-event\""),
            "{json}"
        );
        let back = AuditEvent::from_json(&json).unwrap();
        assert_eq!(back.event_hash, event.event_hash);

        // Tampering with any content invalidates the event.
        let tampered = json.replace("send_customer_email", "send_spam_email");
        assert!(AuditEvent::from_json(&tampered).is_err());
    }
}
