//! Tenant: the security and data-isolation boundary (spec 01 §1.3).

use crate::ids::{OrganizationId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Where task data may be processed/stored. Constraint survives provider
/// fallback (spec 0.5 invariant 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataEgressPolicy {
    /// No data may leave the local device. Cloud providers are forbidden.
    LocalOnly,
    /// Data may leave the device only to providers inside approved regions.
    ApprovedRegions,
    /// Cloud processing is allowed within the provider allowlist.
    CloudAllowed,
}

/// Evidence/audit retention bounds (spec 01 §1.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Retention duration in days for operational evidence.
    pub evidence_retention_days: u32,
    /// Retention duration in days for audit events. Audit events SHOULD be
    /// append-only; deletion is by retention policy, not by edit.
    pub audit_retention_days: u32,
}

/// Tenant definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tenant {
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    /// Data-egress constraint applied to every task under this tenant.
    pub data_egress_policy: DataEgressPolicy,
    /// Providers this tenant permits (advisory names matched against
    /// provider adapter identities). Empty means "no providers allowed".
    pub provider_allowlist: BTreeSet<String>,
    pub retention: RetentionPolicy,
    /// Version of the composed organization policy in force.
    pub organization_policy_version: String,
    /// Device classes allowed to execute tenant work, e.g. `managed`,
    /// `byod`. Empty means unmanaged devices are denied.
    pub allowed_device_classes: BTreeSet<String>,
    /// Destination for tenant audit export, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_destination: Option<String>,
}

impl Tenant {
    /// True when `provider` may receive tenant data.
    #[must_use]
    pub fn provider_allowed(&self, provider: &str) -> bool {
        match self.data_egress_policy {
            DataEgressPolicy::LocalOnly => false,
            DataEgressPolicy::ApprovedRegions | DataEgressPolicy::CloudAllowed => {
                self.provider_allowlist.contains(provider)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::OrganizationId;

    fn tenant(egress: DataEgressPolicy) -> Tenant {
        Tenant {
            tenant_id: TenantId::parse("t-acme").unwrap(),
            organization_id: OrganizationId::parse("org-acme").unwrap(),
            data_egress_policy: egress,
            provider_allowlist: BTreeSet::from(["anthropic".to_owned()]),
            retention: RetentionPolicy {
                evidence_retention_days: 30,
                audit_retention_days: 365,
            },
            organization_policy_version: "1.0.0".to_owned(),
            allowed_device_classes: BTreeSet::from(["managed".to_owned()]),
            audit_destination: None,
        }
    }

    #[test]
    fn local_only_blocks_all_providers() {
        let t = tenant(DataEgressPolicy::LocalOnly);
        assert!(!t.provider_allowed("anthropic"));
        assert!(!t.provider_allowed("local-ollama"));
    }

    #[test]
    fn allowlist_is_enforced_outside_local_only() {
        let t = tenant(DataEgressPolicy::CloudAllowed);
        assert!(t.provider_allowed("anthropic"));
        assert!(!t.provider_allowed("openai"));
    }

    #[test]
    fn serde_round_trip() {
        let t = tenant(DataEgressPolicy::ApprovedRegions);
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("\"LOCAL_ONLY\"") || json.contains("\"APPROVED_REGIONS\""));
        let back: Tenant = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}
