//! Semantic target description and delivery modes (spec 07 §7.4, §7.20).

use serde::{Deserialize, Serialize};

/// Semantic target: application/window/role/name plus an optional stable
/// property. Coordinates are NOT workflow identity and never appear here
/// (§7.4); they may only appear as executor-internal fallback data behind
/// an explicitly authorized [`DeliveryMode::GlobalInput`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticTarget {
    /// Application identity (bundle id on macOS, process/AUMID on
    /// Windows).
    pub application: String,
    /// Window title or accessible window name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    /// Control role, e.g. `button`, `text_field`, `menu_item` (AX role /
    /// UIA control type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Accessible name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional stable property/value pair for disambiguation, e.g.
    /// `("identifier", "save-draft")`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stable_property: Option<(String, String)>,
}

impl SemanticTarget {
    /// Identity key used for audit and wrong-window prevention: the
    /// application + window the target belongs to.
    #[must_use]
    pub fn context_key(&self) -> String {
        format!(
            "{}::{}",
            self.application,
            self.window.as_deref().unwrap_or("*")
        )
    }
}

/// How an operation is delivered (§7.20). Escalation from semantic to
/// global input is NEVER implicit: it must be requested by execution
/// preference AND re-authorized by policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    /// Semantic, app-scoped, background-capable delivery (default; AX /
    /// UIA without focus steal).
    #[default]
    SemanticBackground,
    /// Semantic delivery requiring foreground focus.
    SemanticForeground,
    /// Unrestricted global keyboard/mouse synthesis. Requires an explicit
    /// execution preference and fresh policy evaluation; the executor
    /// refuses otherwise.
    GlobalInput,
}

impl DeliveryMode {
    /// True when the mode may synthesize global input.
    #[must_use]
    pub const fn is_global_input(self) -> bool {
        matches!(self, Self::GlobalInput)
    }

    /// True when the delivery promises to preserve the physical cursor
    /// and current focus (background semantic delivery).
    #[must_use]
    pub const fn preserves_focus_and_cursor(self) -> bool {
        matches!(self, Self::SemanticBackground)
    }
}
