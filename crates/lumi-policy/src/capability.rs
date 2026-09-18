//! Capability grants and the grant registry (spec 04 §4.3).
//!
//! Capability answers "may this principal request this class of
//! operation?"; policy answers "may this specific action execute?"; approval
//! answers "did a human authorize this exact action?". These remain distinct.
//! A grant is permission to *request* — it never authorizes execution.

use lumi_protocol::{Capability, Principal, ResourceType, TenantId, Timestamp};
use std::collections::BTreeSet;

/// Scope of resources a grant covers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResourceScope {
    /// Resource types covered. Empty = no types (fail closed).
    pub resource_types: BTreeSet<ResourceType>,
    /// Canonical-id prefixes covered (e.g. `crm/quotes/Q-`). Empty set
    /// means all ids within the covered types.
    pub id_prefixes: BTreeSet<String>,
}

impl ResourceScope {
    #[must_use]
    pub fn all_of(types: impl IntoIterator<Item = ResourceType>) -> Self {
        Self {
            resource_types: types.into_iter().collect(),
            id_prefixes: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn matches(&self, resource_type: &ResourceType, resource_id: &str) -> bool {
        if !self.resource_types.contains(resource_type) {
            return false;
        }
        if self.id_prefixes.is_empty() {
            return true;
        }
        self.id_prefixes
            .iter()
            .any(|prefix| resource_id.starts_with(prefix.as_str()))
    }
}

/// Where a grant came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantSource {
    OrganizationPolicy,
    WorkflowPack,
    UserDelegation,
    DeviceDefault,
}

/// Permission for a principal to request a capability within a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGrant {
    pub grant_id: String,
    /// Tenant the grant lives in. A grant never crosses tenants.
    pub tenant_id: TenantId,
    /// Principal allowed to hold requests. `None` = every authenticated
    /// principal in the tenant (still subject to policy).
    pub principal_id: Option<lumi_protocol::PrincipalId>,
    pub capability: Capability,
    pub resource_scope: ResourceScope,
    /// Canonical-target prefixes this grant may address. Empty = no target
    /// restriction beyond the resource scope.
    pub target_prefixes: BTreeSet<String>,
    pub expires_at: Option<Timestamp>,
    pub source: GrantSource,
}

/// Result of a capability check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityCheck {
    Granted { grant_id: String },
    Denied { reason: &'static str },
}

/// Registry of capability grants. Lookup is fail-closed: no matching
/// grant means denied.
#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    grants: Vec<CapabilityGrant>,
}

impl CapabilityRegistry {
    #[must_use]
    pub fn new(grants: Vec<CapabilityGrant>) -> Self {
        Self { grants }
    }

    /// Registers a grant. Returns the registry for chaining.
    pub fn grant(mut self, grant: CapabilityGrant) -> Self {
        self.grants.push(grant);
        self
    }

    /// Checks whether `principal` may request `capability` on
    /// (`resource_type`, `resource_id`, `target`).
    #[must_use]
    pub fn check(
        &self,
        principal: &Principal,
        capability: &Capability,
        resource_type: &ResourceType,
        resource_id: &str,
        target: &str,
        now: Timestamp,
    ) -> CapabilityCheck {
        for grant in &self.grants {
            if grant.tenant_id != principal.tenant_id {
                continue;
            }
            if let Some(expected) = &grant.principal_id {
                if expected != &principal.principal_id {
                    continue;
                }
            }
            if grant.capability != *capability {
                continue;
            }
            if let Some(expires_at) = grant.expires_at {
                if now > expires_at {
                    continue;
                }
            }
            if !grant.resource_scope.matches(resource_type, resource_id) {
                continue;
            }
            if !grant.target_prefixes.is_empty()
                && !grant
                    .target_prefixes
                    .iter()
                    .any(|prefix| target.starts_with(prefix.as_str()))
            {
                continue;
            }
            return CapabilityCheck::Granted {
                grant_id: grant.grant_id.clone(),
            };
        }
        CapabilityCheck::Denied {
            reason: "no capability grant covers this principal/scope (deny-by-default)",
        }
    }

    /// True when the capability family is known to this registry at all
    /// (any grant references it). Used by the hard-safety layer to fail
    /// closed on unknown consequential capabilities.
    #[must_use]
    pub fn knows_family(&self, capability: &Capability) -> bool {
        self.grants.iter().any(|g| g.capability.0 == capability.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::{AuthenticationStrength, PrincipalId, PrincipalKind};

    fn principal(tenant: &str, id: &str) -> Principal {
        Principal {
            principal_id: PrincipalId::parse(id).unwrap(),
            tenant_id: TenantId::parse(tenant).unwrap(),
            kind: PrincipalKind::User,
            authenticated_at: Some(Timestamp::UNIX_EPOCH),
            authentication_strength: Some(AuthenticationStrength::Mfa),
        }
    }

    fn email_grant(tenant: &str) -> CapabilityGrant {
        CapabilityGrant {
            grant_id: "g-email".to_owned(),
            tenant_id: TenantId::parse(tenant).unwrap(),
            principal_id: None,
            capability: Capability::well_known(lumi_protocol::capabilities::EMAIL_DRAFT_CREATE),
            resource_scope: ResourceScope::all_of([ResourceType::well_known(
                ResourceType::EMAIL_DRAFT,
            )]),
            target_prefixes: BTreeSet::new(),
            expires_at: None,
            source: GrantSource::OrganizationPolicy,
        }
    }

    #[test]
    fn grant_allows_matching_request() {
        let registry = CapabilityRegistry::default().grant(email_grant("t-1"));
        let check = registry.check(
            &principal("t-1", "u-1"),
            &Capability::well_known(lumi_protocol::capabilities::EMAIL_DRAFT_CREATE),
            &ResourceType::well_known(ResourceType::EMAIL_DRAFT),
            "outbound/x",
            "mailto:a@b.test",
            Timestamp::UNIX_EPOCH,
        );
        assert!(matches!(check, CapabilityCheck::Granted { .. }));
    }

    #[test]
    fn no_grant_is_denied() {
        let registry = CapabilityRegistry::default();
        let check = registry.check(
            &principal("t-1", "u-1"),
            &Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            &ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            "x",
            "mailto:a@b.test",
            Timestamp::UNIX_EPOCH,
        );
        assert!(matches!(check, CapabilityCheck::Denied { .. }));
    }

    #[test]
    fn cross_tenant_grant_does_not_apply() {
        let registry = CapabilityRegistry::default().grant(email_grant("t-1"));
        let check = registry.check(
            &principal("t-2", "u-1"),
            &Capability::well_known(lumi_protocol::capabilities::EMAIL_DRAFT_CREATE),
            &ResourceType::well_known(ResourceType::EMAIL_DRAFT),
            "outbound/x",
            "mailto:a@b.test",
            Timestamp::UNIX_EPOCH,
        );
        assert!(matches!(check, CapabilityCheck::Denied { .. }));
    }

    #[test]
    fn expired_grant_is_denied() {
        let mut grant = email_grant("t-1");
        grant.expires_at = Some(Timestamp::from_epoch(100, 0).unwrap());
        let registry = CapabilityRegistry::default().grant(grant);
        let check = registry.check(
            &principal("t-1", "u-1"),
            &Capability::well_known(lumi_protocol::capabilities::EMAIL_DRAFT_CREATE),
            &ResourceType::well_known(ResourceType::EMAIL_DRAFT),
            "outbound/x",
            "mailto:a@b.test",
            Timestamp::from_epoch(101, 0).unwrap(),
        );
        assert!(matches!(check, CapabilityCheck::Denied { .. }));
    }

    #[test]
    fn target_prefix_restrictions_bind() {
        let mut grant = email_grant("t-1");
        grant.target_prefixes = BTreeSet::from(["mailto:*@customer.example.test".to_owned()]);
        // Prefix matching is literal; policy authors use explicit prefixes.
        grant.target_prefixes = BTreeSet::from(["mailto:support@customer.example.test".to_owned()]);
        let registry = CapabilityRegistry::default().grant(grant);
        let allowed = registry.check(
            &principal("t-1", "u-1"),
            &Capability::well_known(lumi_protocol::capabilities::EMAIL_DRAFT_CREATE),
            &ResourceType::well_known(ResourceType::EMAIL_DRAFT),
            "outbound/x",
            "mailto:support@customer.example.test",
            Timestamp::UNIX_EPOCH,
        );
        assert!(matches!(allowed, CapabilityCheck::Granted { .. }));
        let denied = registry.check(
            &principal("t-1", "u-1"),
            &Capability::well_known(lumi_protocol::capabilities::EMAIL_DRAFT_CREATE),
            &ResourceType::well_known(ResourceType::EMAIL_DRAFT),
            "outbound/x",
            "mailto:ceo@customer.example.test",
            Timestamp::UNIX_EPOCH,
        );
        assert!(matches!(denied, CapabilityCheck::Denied { .. }));
    }
}
