//! Session state, permissions, and runtime generations (spec 07 §7.6,
//! §7.7, §7.18).

use serde::{Deserialize, Serialize};

/// Runtime generation: bumped on driver/runtime restart, rebind, or
/// permission-host change. Session handles embed the generation they were
/// created under; a handle whose generation does not match the current
/// one MUST fail closed (§7.18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct RuntimeGeneration(pub u64);

impl RuntimeGeneration {
    /// Bumps to the next generation (driver/runtime restart or rebind).
    pub fn bump(&mut self) -> Self {
        self.0 += 1;
        *self
    }
}

/// A session handle bound to a generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionHandle {
    pub session_id: String,
    pub generation: RuntimeGeneration,
}

impl SessionHandle {
    /// Validates against the current generation; stale handles fail
    /// closed.
    #[must_use]
    pub const fn is_current(&self, current: RuntimeGeneration) -> bool {
        self.generation.0 == current.0
    }
}

/// Desktop session state the executor MUST detect before acting (§7.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionState {
    /// Interactive session available.
    Unlocked,
    /// Locked workstation: unattended desktop automation must fail
    /// explicitly.
    Locked,
    /// Disconnected remote desktop.
    Disconnected,
    /// Windows UAC secure desktop: user input is not synthesizable.
    UacSecureDesktop,
}

impl SessionState {
    /// True when native execution can proceed.
    #[must_use]
    pub const fn supports_automation(self) -> bool {
        matches!(self, Self::Unlocked)
    }
}

/// OS permissions required for native execution and their detected state
/// (§7.6). Detection is read-only: the executor NEVER grants permissions
/// or escalates; onboarding belongs to the desktop UX.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRequirement {
    /// e.g. `macos.accessibility`, `macos.screen_recording`,
    /// `windows.uia`.
    pub capability: String,
    pub state: PermissionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PermissionState {
    Granted,
    NotGranted,
    /// Previously granted and later revoked: treated as a hard failure.
    Revoked,
}

impl PermissionState {
    /// True when execution may proceed for this permission.
    #[must_use]
    pub const fn allows_execution(self) -> bool {
        matches!(self, Self::Granted)
    }
}

/// Overall permission gate: all requirements must be Granted.
#[must_use]
pub fn permissions_allow(requirements: &[PermissionRequirement]) -> bool {
    !requirements.is_empty() && requirements.iter().all(|r| r.state.allows_execution())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_generation_handles_fail_closed() {
        let mut generation = RuntimeGeneration::default();
        let handle = SessionHandle {
            session_id: "s1".to_owned(),
            generation,
        };
        assert!(handle.is_current(generation));
        // Driver restart bumps the generation: old handles are stale.
        let bumped = generation.bump();
        assert!(!handle.is_current(bumped));
    }

    #[test]
    fn locked_and_secure_desktops_block_automation() {
        assert!(SessionState::Unlocked.supports_automation());
        assert!(!SessionState::Locked.supports_automation());
        assert!(!SessionState::Disconnected.supports_automation());
        assert!(!SessionState::UacSecureDesktop.supports_automation());
    }

    #[test]
    fn permission_gate_requires_all_granted() {
        let granted = PermissionRequirement {
            capability: "macos.accessibility".to_owned(),
            state: PermissionState::Granted,
        };
        let revoked = PermissionRequirement {
            capability: "macos.screen_recording".to_owned(),
            state: PermissionState::Revoked,
        };
        assert!(permissions_allow(std::slice::from_ref(&granted)));
        assert!(!permissions_allow(&[granted, revoked]));
        assert!(!permissions_allow(&[]));
    }
}
