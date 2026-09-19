//! Managed/unmanaged local execution hosts (spec 01 §1.5).

use crate::ids::{DeviceId, TenantId};
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

/// Execution host platform. V1 supports macOS and Windows employee
/// deployments; Linux is core-runtime only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Platform {
    Macos,
    Windows,
    LinuxCore,
}

/// Device lifecycle trust state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrustState {
    Unregistered,
    Registered,
    Trusted,
    Revoked,
}

/// Update deployment ring for staged rollouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpdateRing {
    Canary,
    Beta,
    Stable,
}

/// A local execution host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub device_id: DeviceId,
    pub tenant_id: TenantId,
    pub platform: Platform,
    /// CPU architecture, e.g. `aarch64`, `x86_64`.
    pub architecture: String,
    /// Lumi runtime version running on the device.
    pub runtime_version: String,
    /// Employee app version running on the device.
    pub app_version: String,
    pub registered_at: Timestamp,
    pub trust_state: TrustState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<Timestamp>,
    /// Version of the policy document the device enforces.
    pub policy_version: String,
    pub update_ring: UpdateRing,
}

impl Device {
    /// A REVOKED device MUST NOT accept new unattended work (spec 01 §1.5).
    #[must_use]
    pub const fn accepts_unattended_work(&self) -> bool {
        !matches!(
            self.trust_state,
            TrustState::Revoked | TrustState::Unregistered
        )
    }

    /// True when the device belongs to `tenant_id`.
    #[must_use]
    pub fn belongs_to(&self, tenant_id: &TenantId) -> bool {
        &self.tenant_id == tenant_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(state: TrustState) -> Device {
        Device {
            device_id: DeviceId::parse("dev-1").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            platform: Platform::Macos,
            architecture: "aarch64".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            app_version: "0.1.0".to_owned(),
            registered_at: Timestamp::UNIX_EPOCH,
            trust_state: state,
            last_seen_at: None,
            policy_version: "1.0.0".to_owned(),
            update_ring: UpdateRing::Stable,
        }
    }

    #[test]
    fn revoked_devices_do_not_accept_unattended_work() {
        assert!(!device(TrustState::Revoked).accepts_unattended_work());
        assert!(!device(TrustState::Unregistered).accepts_unattended_work());
        assert!(device(TrustState::Registered).accepts_unattended_work());
        assert!(device(TrustState::Trusted).accepts_unattended_work());
    }

    #[test]
    fn serde_rejects_unknown_trust_state() {
        assert!(serde_json::from_str::<TrustState>("\"BLESSED\"").is_err());
        let json = serde_json::to_string(&Platform::Windows).unwrap();
        assert_eq!(json, "\"WINDOWS\"");
    }
}
