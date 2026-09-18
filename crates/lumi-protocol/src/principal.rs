//! Principals: who or what is requesting authority (spec 01 §1.4).

use crate::ids::{PrincipalId, TenantId};
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// Kind of principal requesting authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrincipalKind {
    /// A human user.
    User,
    /// A workflow definition acting on behalf of its tenant.
    Workflow,
    /// A scheduler/trigger.
    Schedule,
    /// A service integration.
    Service,
    /// An administrative principal.
    Admin,
    /// The runtime itself.
    System,
}

/// How strongly the principal's identity was established.
///
/// Unauthenticated principals fail closed for consequential policy decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthenticationStrength {
    /// Identity could not be established. Never sufficient for authority.
    Unauthenticated,
    /// Possession of a registered device.
    DevicePossession,
    /// Shared knowledge factor.
    Password,
    /// Federated single sign-on.
    Sso,
    /// Multi-factor authentication.
    Mfa,
    /// Hardware-backed key (e.g. platform secure enclave).
    HardwareKey,
}

/// Who or what is requesting authority.
///
/// Model/provider identity is *not* an authorization principal: models
/// propose, they never act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub principal_id: PrincipalId,
    pub tenant_id: TenantId,
    pub kind: PrincipalKind,
    /// When the principal authenticated; `None` for runtime-internal
    /// principals where authentication is implicit (e.g. SYSTEM).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authenticated_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authentication_strength: Option<AuthenticationStrength>,
}

impl Principal {
    /// True when the principal is authenticated strongly enough to hold
    /// authority. `SYSTEM` principals are authenticated by construction
    /// (they are the local runtime).
    #[must_use]
    pub fn is_authenticated(&self) -> bool {
        if self.kind == PrincipalKind::System {
            return true;
        }
        matches!(
            self.authentication_strength,
            Some(AuthenticationStrength::DevicePossession)
                | Some(AuthenticationStrength::Password)
                | Some(AuthenticationStrength::Sso)
                | Some(AuthenticationStrength::Mfa)
                | Some(AuthenticationStrength::HardwareKey)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(kind: PrincipalKind, strength: Option<AuthenticationStrength>) -> Principal {
        Principal {
            principal_id: PrincipalId::parse("p-1").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            kind,
            authenticated_at: Some(Timestamp::now()),
            authentication_strength: strength,
        }
    }

    #[test]
    fn system_is_authenticated_without_explicit_strength() {
        assert!(principal(PrincipalKind::System, None).is_authenticated());
        assert!(!principal(PrincipalKind::User, None).is_authenticated());
        assert!(!principal(
            PrincipalKind::User,
            Some(AuthenticationStrength::Unauthenticated)
        )
        .is_authenticated());
        assert!(
            principal(PrincipalKind::User, Some(AuthenticationStrength::Mfa)).is_authenticated()
        );
    }

    #[test]
    fn serde_uses_screaming_snake() {
        let json = serde_json::to_string(&PrincipalKind::Workflow).unwrap();
        assert_eq!(json, "\"WORKFLOW\"");
        let back: PrincipalKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PrincipalKind::Workflow);
    }

    #[test]
    fn unknown_kind_fails_closed() {
        let err = serde_json::from_str::<PrincipalKind>("\"GALACTIC_EMPEROR\"").unwrap_err();
        assert!(err.to_string().contains("unknown variant"), "{err}");
    }
}
