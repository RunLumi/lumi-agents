//! The Lumi-owned DesktopDriver contract (spec 07 §7.2, §7.15, §7.17).

use crate::session::{SessionHandle, SessionState};
use crate::target::{DeliveryMode, SemanticTarget};
use lumi_protocol::FailureCategory;
use serde::{Deserialize, Serialize};

/// Canonical native executor outcomes (§7.15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NativeOutcome {
    /// The declared effect oracle observed the intended state change.
    /// An OS/API success alone is NOT sufficient.
    Delivered,
    /// Stable refusal before any mutation (unsupported route, policy
    /// refusal). Must cause no side effects (§7.21).
    Refused,
    /// The operation ran without error but the oracle did not observe
    /// the intended change. NOT a success.
    NoEffect,
    /// The effect could not be confirmed (e.g. save interrupted).
    Ambiguous,
    /// Failed with a canonical category.
    Error,
    /// Cancelled before or during execution.
    Cancelled,
}

impl NativeOutcome {
    /// True only for verified delivery (§16.16 alignment).
    #[must_use]
    pub const fn counts_as_delivery(self) -> bool {
        matches!(self, Self::Delivered)
    }
}

/// Why an operation was refused (stable codes, §7.15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalReason {
    /// Semantic target unavailable and no authorized fallback exists.
    NoSemanticTarget,
    /// Global-input delivery not authorized for this action.
    GlobalInputNotAuthorized,
    /// Route unsupported on this platform/driver build.
    UnsupportedRoute,
    /// Wrong window/application context for the target.
    WrongTargetContext,
}

/// One native operation, semantically described.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "operation")]
pub enum NativeOperation {
    /// Bring the target window into scope (never silently: the target's
    /// context is validated first).
    FocusWindow { target: SemanticTarget },
    /// Read the target's accessible value/text (verification path).
    ReadValue { target: SemanticTarget },
    /// Click a control.
    Click {
        target: SemanticTarget,
        mode: DeliveryMode,
    },
    /// Set a text field's value (preferred over keystrokes when AX/UIA
    /// supports it: no global input, no focus requirement).
    SetValue {
        target: SemanticTarget,
        value: String,
    },
    /// Press a control-scoped keyboard action (e.g. Return on a dialog's
    /// default button) within the app-scoped context.
    PressAction {
        target: SemanticTarget,
        action: String,
    },
}

/// Result of one operation including outcome + oracle evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeResult {
    pub outcome: NativeOutcome,
    /// Canonical failure category when outcome is Error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<FailureCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<RefusalReason>,
    /// What the effect oracle observed (e.g. post-action value, window
    /// state). Evidence, not pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oracle_observation: Option<String>,
    /// Collateral-effect checks (§7.17): focus preserved, cursor
    /// preserved, no input leakage.
    #[serde(default)]
    pub collateral: CollateralReport,
}

/// Collateral-effect oracles (§7.17): correct mutation with unacceptable
/// collateral is not a certified success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CollateralReport {
    /// Active/focused application remained the promised one.
    pub focus_preserved: Option<bool>,
    /// Physical pointer/cursor preserved (background delivery promise).
    pub cursor_preserved: Option<bool>,
    /// No input leaked to a non-target app/window.
    pub no_input_leak: Option<bool>,
    /// Clipboard unchanged by the operation.
    pub clipboard_unchanged: Option<bool>,
}

impl CollateralReport {
    /// All recorded checks passed? None (not checked) does not fail.
    #[must_use]
    pub fn all_pass(&self) -> bool {
        [
            self.focus_preserved,
            self.cursor_preserved,
            self.no_input_leak,
            self.clipboard_unchanged,
        ]
        .iter()
        .all(|c| c.unwrap_or(true))
    }
}

/// The effect oracle: observes the intended executor-level state change
/// (§7.15). Returning `None` means the oracle cannot confirm the effect.
pub type EffectOracle<'a> = &'a dyn Fn() -> Option<String>;

/// The Lumi-owned desktop driver interface. Upstream engines (Cua now,
/// direct AX/UIA drivers later) implement this; workflow code never sees
/// them (§7.2, §7.11).
pub trait DesktopDriver {
    /// Driver identity for audit/certification, e.g. `cua-driver`.
    fn name(&self) -> &'static str;

    /// Driver version (pinned build).
    fn version(&self) -> &'static str;

    /// Detects current desktop session state (read-only).
    fn session_state(&self, handle: &SessionHandle) -> Result<SessionState, NativeError>;

    /// Detects OS permissions (read-only; never grants).
    fn permissions(
        &self,
        handle: &SessionHandle,
    ) -> Result<Vec<crate::session::PermissionRequirement>, NativeError>;

    /// Executes one semantic operation with its effect oracle.
    fn execute(
        &self,
        handle: &SessionHandle,
        operation: NativeOperation,
        oracle: EffectOracle<'_>,
    ) -> Result<NativeResult, NativeError>;
}

/// Driver-level errors (operation failures travel in
/// [`NativeResult::failure`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeError {
    pub category: FailureCategory,
    pub message: String,
}

impl NativeError {
    #[must_use]
    pub fn new(category: FailureCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.category.as_str(), self.message)
    }
}

impl std::error::Error for NativeError {}
