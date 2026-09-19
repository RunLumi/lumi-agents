//! OS permission detection and onboarding status (spec 15: the app shows
//! permission state but never grants or escalates).

use serde::{Deserialize, Serialize};

/// Per-permission detection result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionStatus {
    /// Capability name (e.g. `macos.accessibility`).
    pub capability: String,
    /// Human-readable description for the onboarding UX.
    pub description: String,
    /// Current state.
    pub state: PermissionState,
    /// Whether the feature requires this permission to function.
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PermissionState {
    Granted,
    NotGranted,
    Revoked,
    /// Cannot be detected programmatically; user must check manually.
    RequiresManualCheck,
}

impl PermissionState {
    /// True when execution may proceed.
    #[must_use]
    pub const fn allows_execution(self) -> bool {
        matches!(self, Self::Granted)
    }
}

/// Aggregate permission check for the desktop app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionCheckResult {
    pub permissions: Vec<PermissionStatus>,
    /// True when ALL required permissions are granted.
    pub all_granted: bool,
    /// The platform (for rendering the correct onboarding instructions).
    pub platform: String,
}

/// Convenience constructor for macOS permission sets.
#[must_use]
pub fn macos_permissions(accessibility: bool, screen_recording: bool) -> PermissionCheckResult {
    PermissionCheckResult {
        permissions: vec![
            PermissionStatus {
                capability: "macos.accessibility".to_owned(),
                description: "Required to read and control applications via Accessibility"
                    .to_owned(),
                state: if accessibility {
                    PermissionState::Granted
                } else {
                    PermissionState::NotGranted
                },
                required: true,
            },
            PermissionStatus {
                capability: "macos.screen_recording".to_owned(),
                description: "Required for selective screenshot evidence".to_owned(),
                state: if screen_recording {
                    PermissionState::Granted
                } else {
                    PermissionState::RequiresManualCheck
                },
                required: false, // optional: only needed for screenshot evidence
            },
        ],
        all_granted: accessibility,
        platform: "macos".to_owned(),
    }
}

/// Convenience constructor for Windows permission sets.
#[must_use]
pub fn windows_permissions(uia_available: bool) -> PermissionCheckResult {
    PermissionCheckResult {
        permissions: vec![PermissionStatus {
            capability: "windows.uia".to_owned(),
            description: "UI Automation for native app control".to_owned(),
            state: if uia_available {
                PermissionState::Granted
            } else {
                PermissionState::RequiresManualCheck
            },
            required: true,
        }],
        all_granted: uia_available,
        platform: "windows".to_owned(),
    }
}

/// Type alias for the platform permissions module (spec 15 onboarding).
pub type PlatformPermissions = PermissionCheckResult;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_onboarding_detects_missing() {
        let result = macos_permissions(false, false);
        assert!(!result.all_granted);
        assert_eq!(result.permissions.len(), 2);
        assert!(!result.permissions[0].state.allows_execution());
    }

    #[test]
    fn all_granted_when_permissions_present() {
        let result = windows_permissions(true);
        assert!(result.all_granted);
    }
}
