//! Execution tiers and observation surfaces (spec 03 §3.5, spec 05 §5.2).
//!
//! Two distinct vocabularies, deliberately not merged:
//!
//! - [`ExecutionTier`] is the semantic-preference ladder for *actions*
//!   (connector > browser > native > app adapter > vision).
//! - [`ObservationSurface`] is where an *observation* came from, including
//!   orthogonal surfaces (shell, files, artifacts) that are not tiers.

use serde::{Deserialize, Serialize};

/// Ordered from most deterministic/semantic to least. Ord follows the
/// preference order (spec 05 §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionTier {
    /// Connector/API integration (most preferred).
    ConnectorApi,
    /// Browser semantic automation (DOM/accessibility locators).
    BrowserSemantic,
    /// Native desktop semantic automation (AX/UIA).
    NativeSemantic,
    /// App-specific deterministic adapter.
    AppAdapter,
    /// Vision/coordinate fallback (least preferred).
    Vision,
}

impl ExecutionTier {
    /// Human-readable lowercase name matching observation-surface style.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConnectorApi => "connector",
            Self::BrowserSemantic => "browser",
            Self::NativeSemantic => "native",
            Self::AppAdapter => "app",
            Self::Vision => "vision",
        }
    }

    /// True when this tier is the vision/coordinate fallback.
    #[must_use]
    pub const fn is_vision(self) -> bool {
        matches!(self, Self::Vision)
    }
}

/// Where an observation originated. Includes orthogonal surfaces that are
/// not part of the execution-tier ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObservationSurface {
    Connector,
    Browser,
    Native,
    App,
    Vision,
    Shell,
    Files,
    Artifact,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_order_matches_spec_05() {
        assert!(ExecutionTier::ConnectorApi < ExecutionTier::BrowserSemantic);
        assert!(ExecutionTier::BrowserSemantic < ExecutionTier::NativeSemantic);
        assert!(ExecutionTier::NativeSemantic < ExecutionTier::AppAdapter);
        assert!(ExecutionTier::AppAdapter < ExecutionTier::Vision);
    }

    #[test]
    fn wire_names() {
        assert_eq!(
            serde_json::to_string(&ExecutionTier::ConnectorApi).unwrap(),
            "\"CONNECTOR_API\""
        );
        assert_eq!(
            serde_json::to_string(&ObservationSurface::Shell).unwrap(),
            "\"SHELL\""
        );
    }

    #[test]
    fn tier_names_align_with_observation_surfaces() {
        for tier in [
            ExecutionTier::ConnectorApi,
            ExecutionTier::BrowserSemantic,
            ExecutionTier::NativeSemantic,
            ExecutionTier::AppAdapter,
            ExecutionTier::Vision,
        ] {
            let surface = tier.as_str();
            assert!(
                ["connector", "browser", "native", "app", "vision"].contains(&surface),
                "tier name {surface} has no observation surface"
            );
        }
    }
}
