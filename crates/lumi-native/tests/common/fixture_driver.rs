//! Deterministic fixture DesktopDriver for certification tests
//! (spec 07 §7.14, §7.21).
//!
//! The fixture models a macOS-style AX environment with scripted state so
//! every certification scenario is reproducible on any platform. Windows
//! UIA fixtures reuse the same scenarios with Windows identifiers
//! (§7.14: "macOS and Windows deterministic fixtures").

use lumi_native::driver::{
    CollateralReport, DesktopDriver, EffectOracle, NativeError, NativeOperation, NativeOutcome,
    NativeResult, RefusalReason,
};
use lumi_native::session::{
    PermissionRequirement, PermissionState, RuntimeGeneration, SessionHandle, SessionState,
};
use lumi_native::target::DeliveryMode;
use lumi_protocol::FailureCategory;
use std::cell::RefCell;

/// Scripted fixture state, mutable per scenario.
#[derive(Debug, Clone)]
pub struct FixtureDesktop {
    /// Current desktop session state.
    pub session: SessionState,
    /// The driver's current runtime generation (bumped on restart/rebind).
    pub current_generation: RuntimeGeneration,
    /// Permission table.
    pub permissions: Vec<PermissionRequirement>,
    /// The "focused" app::window context.
    pub focused_context: String,
    /// Semantic target values, keyed `app::window::name`.
    pub values: std::cell::RefCell<std::collections::BTreeMap<String, String>>,
    /// A modal dialog blocks interaction when set.
    pub modal_dialog: Option<String>,
    /// Simulate an OS call that "succeeds" but has no effect.
    pub silent_failure: bool,
    /// Simulate an interruption with unknown effect.
    pub interrupted: bool,
    /// Requests are recorded for collateral/cancel assertions.
    pub operations: RefCell<Vec<String>>,
}

impl FixtureDesktop {
    #[must_use]
    pub fn macos_notes() -> Self {
        Self {
            session: SessionState::Unlocked,
            current_generation: RuntimeGeneration::default(),
            permissions: vec![PermissionRequirement {
                capability: "macos.accessibility".to_owned(),
                state: PermissionState::Granted,
            }],
            focused_context: "com.acme.notes::Untitled".to_owned(),
            values: std::cell::RefCell::new(std::collections::BTreeMap::from([(
                "com.acme.notes::Untitled::note-body".to_owned(),
                String::new(),
            )])),
            modal_dialog: None,
            silent_failure: false,
            interrupted: false,
            operations: RefCell::new(Vec::new()),
        }
    }

    /// Windows UIA variant of the same scenario (§7.14).
    #[must_use]
    pub fn windows_notepad() -> Self {
        Self {
            session: SessionState::Unlocked,
            current_generation: RuntimeGeneration::default(),
            permissions: vec![PermissionRequirement {
                capability: "windows.uia".to_owned(),
                state: PermissionState::Granted,
            }],
            focused_context: "Notepad.exe::Untitled - Notepad".to_owned(),
            values: std::cell::RefCell::new(std::collections::BTreeMap::from([(
                "Notepad.exe::Untitled - Notepad::editor".to_owned(),
                String::new(),
            )])),
            modal_dialog: None,
            silent_failure: false,
            interrupted: false,
            operations: RefCell::new(Vec::new()),
        }
    }
}

impl DesktopDriver for FixtureDesktop {
    fn name(&self) -> &'static str {
        "fixture-driver"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn session_state(&self, handle: &SessionHandle) -> Result<SessionState, NativeError> {
        if !handle.is_current(self.current_generation) {
            return Err(NativeError::new(
                FailureCategory::UpstreamDriver,
                "stale generation handle",
            ));
        }
        Ok(self.session)
    }

    fn permissions(
        &self,
        _handle: &SessionHandle,
    ) -> Result<Vec<PermissionRequirement>, NativeError> {
        Ok(self.permissions.clone())
    }

    fn execute(
        &self,
        handle: &SessionHandle,
        operation: NativeOperation,
        oracle: EffectOracle<'_>,
    ) -> Result<NativeResult, NativeError> {
        if !handle.is_current(self.current_generation) {
            return Err(NativeError::new(
                FailureCategory::UpstreamDriver,
                "stale generation handle",
            ));
        }
        self.operations.borrow_mut().push(format!("{operation:?}"));

        let (target, mode) = match &operation {
            NativeOperation::FocusWindow { target } => (target, DeliveryMode::SemanticBackground),
            NativeOperation::ReadValue { target } => (target, DeliveryMode::SemanticBackground),
            NativeOperation::Click { target, mode } => (target, *mode),
            NativeOperation::SetValue { target, .. } => (target, DeliveryMode::SemanticBackground),
            NativeOperation::PressAction { target, .. } => {
                (target, DeliveryMode::SemanticBackground)
            }
        };

        // Wrong-window prevention (§7.14/§7.21): the target's context must
        // match the current focused context, else refuse pre-mutation.
        if mode != DeliveryMode::GlobalInput
            && !self.focused_context.starts_with(
                &target
                    .context_key()
                    .split("::")
                    .next()
                    .map_or_else(String::new, |app| format!("{app}::")),
            )
        {
            let expected_app = target.context_key();
            if !self
                .focused_context
                .contains(expected_app.split("::").next().unwrap_or_default())
            {
                return Ok(NativeResult {
                    outcome: NativeOutcome::Refused,
                    failure: None,
                    refusal: Some(RefusalReason::WrongTargetContext),
                    oracle_observation: Some(self.focused_context.clone()),
                    collateral: Default::default(),
                });
            }
        }

        // Modal dialog blocks interaction (§7.14).
        if let Some(dialog) = &self.modal_dialog {
            return Ok(NativeResult {
                outcome: NativeOutcome::Error,
                failure: Some(FailureCategory::NativeSession),
                refusal: None,
                oracle_observation: Some(format!("modal dialog blocks interaction: {dialog}")),
                collateral: Default::default(),
            });
        }

        if self.silent_failure {
            // OS call "succeeded" but oracle sees no change (§7.21).
            return Ok(NativeResult {
                outcome: NativeOutcome::NoEffect,
                failure: None,
                refusal: None,
                oracle_observation: None,
                collateral: Default::default(),
            });
        }

        if self.interrupted {
            return Ok(NativeResult {
                outcome: NativeOutcome::Ambiguous,
                failure: None,
                refusal: None,
                oracle_observation: None,
                collateral: Default::default(),
            });
        }

        // Apply the operation to fixture state.
        if let NativeOperation::SetValue { target, value } = &operation {
            let key = format!(
                "{}::{}::{}",
                target.application,
                target.window.as_deref().unwrap_or(""),
                target.name.as_deref().unwrap_or("")
            );
            self.values.borrow_mut().insert(key, value.clone());
        }

        // Oracle decides DELIVERED vs NO_EFFECT (§7.15).
        match oracle() {
            Some(observation) => Ok(NativeResult {
                outcome: NativeOutcome::Delivered,
                failure: None,
                refusal: None,
                oracle_observation: Some(observation),
                collateral: CollateralReport {
                    // Global input takes focus and moves the cursor; the
                    // collateral oracle records the truth (§7.17).
                    focus_preserved: Some(!mode.is_global_input()),
                    cursor_preserved: mode.preserves_focus_and_cursor().then_some(true),
                    no_input_leak: Some(!mode.is_global_input()),
                    clipboard_unchanged: Some(true),
                },
            }),
            None => Ok(NativeResult {
                outcome: NativeOutcome::NoEffect,
                failure: None,
                refusal: None,
                oracle_observation: None,
                collateral: Default::default(),
            }),
        }
    }
}
