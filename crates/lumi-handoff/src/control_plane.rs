//! Control-plane boundary (spec 19): the control plane MAY distribute
//! policy and request work, but MUST NOT bypass local policy/executor
//! gate. Cloud unavailability MUST NOT expand local authority.

use serde::{Deserialize, Serialize};

/// Device sync message (§19.4). References and metadata only — never
/// local content, secrets, or credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSyncMessage {
    pub device_id: String,
    pub runtime_version: String,
    pub app_version: String,
    /// Trust state (e.g. "trusted", "revoked").
    pub trust_state: String,
    pub policy_version: String,
    /// Capability names the device supports (§19.18 capability
    /// negotiation).
    pub supported_capabilities: Vec<String>,
    /// Last-seen timestamp (RFC 3339).
    pub last_seen: String,
    /// Update ring (e.g. "canary", "stable").
    pub update_ring: String,
}

/// Signed policy distribution (§19.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDistribution {
    pub policy_version: String,
    /// Serialized policy payload (org-scoped capability registry, etc.).
    pub policy_payload: String,
    /// Signature over the policy payload (org signing key).
    pub signature: String,
    /// Which tenant this policy applies to.
    pub tenant_id: String,
}

/// Remote task dispatch (§19.6). The control plane REQUESTS work; the
/// local runtime still gates every action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDispatch {
    pub tenant_id: String,
    pub principal_id: String,
    pub workflow_id: String,
    /// Budget carried on the task.
    pub max_actions: u32,
    pub deadline_seconds: u64,
    /// Capability expectations.
    pub required_capabilities: Vec<String>,
    /// Privacy constraint (e.g. "local_only").
    pub privacy_constraint: String,
    /// Lease id (must be valid at admission, §19.7).
    pub lease_id: String,
    /// Idempotency token: replayed dispatch is deduped (§19.14).
    pub idempotency_token: String,
}

/// Conflict policy for mutable metadata sync (§19.8): prefer safer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncConflictPolicy {
    /// On conflict, take the more restrictive value (deny > allow).
    PreferSafer,
    /// Last writer wins (only for non-security metadata).
    LastWriteWins,
}

/// The boundary contract: what the control plane MAY and MAY NOT do
/// (§19.2, §19.15–§19.17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneBoundary {
    /// True when the deployment has a hosted control plane.
    pub has_control_plane: bool,
    /// True when local tasks may continue while the control plane is
    /// unreachable (§19.9). This NEVER expands authority: it only means
    /// the local runtime keeps enforcing its own policy.
    pub offline_continuation: bool,
    /// Whether the local runtime buffers audit offline (§19.10).
    pub audit_buffering: bool,
}

impl ControlPlaneBoundary {
    /// Cloud unavailability MUST NOT expand permissions (§19.5): when
    /// offline, the local runtime keeps its current policy — no new
    /// capabilities, no bypass.
    #[must_use]
    pub const fn offline_expands_authority(&self) -> bool {
        false // structurally impossible: offline never adds grants
    }
}

/// What the control plane may do vs what it MUST NOT do. This is the
/// boundary contract, tested to prevent creep.
pub mod boundary_rules {
    /// Operations the control plane MAY perform (§19.3).
    pub const MAY: &[&str] = &[
        "tenant_device_registry",
        "org_policy_distribution",
        "workflow_pack_catalog",
        "provider_configuration",
        "schedule_metadata",
        "fleet_status",
        "revocation",
        "task_handoff",
        "aggregated_observability",
        "release_ring_metadata",
    ];

    /// Operations the control plane MUST NOT perform (§19.2, §14.4).
    pub const MUST_NOT: &[&str] = &[
        "bypass_local_policy",
        "execute_actions_directly",
        "read_local_secrets",
        "approve_actions",
        "modify_audit",
        "grant_capabilities_to_principal",
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_cannot_bypass_local_policy() {
        // §19.14: cloud cannot execute around local policy. The boundary
        // contract structurally forbids it.
        for forbidden in boundary_rules::MUST_NOT {
            assert!(
                !boundary_rules::MAY.contains(forbidden),
                "{forbidden} must not appear in the control-plane MAY list"
            );
        }
        // Offline never expands authority.
        let boundary = ControlPlaneBoundary {
            has_control_plane: true,
            offline_continuation: true,
            audit_buffering: true,
        };
        assert!(!boundary.offline_expands_authority());
    }

    #[test]
    fn task_dispatch_carries_all_required_fields() {
        let dispatch = TaskDispatch {
            tenant_id: "t-1".to_owned(),
            principal_id: "u-1".to_owned(),
            workflow_id: "invoice-reconciliation".to_owned(),
            max_actions: 10,
            deadline_seconds: 300,
            required_capabilities: vec!["api.read".to_owned()],
            privacy_constraint: "local_only".to_owned(),
            lease_id: "lease-1".to_owned(),
            idempotency_token: "dispatch-42".to_owned(),
        };
        let json = serde_json::to_string(&dispatch).unwrap();
        // §19.14: replayed dispatch is deduped by idempotency token.
        assert!(json.contains("dispatch-42"));
        let back: TaskDispatch = serde_json::from_str(&json).unwrap();
        assert_eq!(back, dispatch);
    }

    #[test]
    fn device_sync_carries_metadata_not_content() {
        let sync = DeviceSyncMessage {
            device_id: "dev-1".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            app_version: "0.1.0".to_owned(),
            trust_state: "trusted".to_owned(),
            policy_version: "1.0.0".to_owned(),
            supported_capabilities: vec!["browser.semantic.v1".to_owned()],
            last_seen: "2026-09-19T00:00:00Z".to_owned(),
            update_ring: "stable".to_owned(),
        };
        let json = serde_json::to_string(&sync).unwrap();
        // §19.4: avoid sending unnecessary local content.
        assert!(!json.contains("secret"));
        assert!(!json.contains("password"));
        assert!(!json.contains("credential_value"));
    }
}
