//! Trust language: the five states a step can be in (§23.9).
//! Never collapse "attempted" into "done".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrustLanguage {
    /// The step is in the plan but has not been reached.
    Planned,
    /// The executor ran but the outcome was not verified (e.g. fire-and-
    /// forget). This is NOT success.
    Attempted,
    /// The executor reported success but postconditions have not been
    /// verified yet. This is NOT done.
    Executed,
    /// Postconditions passed — the step is verified successful.
    Verified,
    /// The outcome is unknown. The task is blocked pending external
    /// verification.
    Ambiguous,
    /// The step failed (executor error, verifier failure, policy denial).
    Failed,
    /// The step was cancelled before completion.
    Cancelled,
}

impl TrustLanguage {
    /// True only for verified success (§23.9: never collapse attempted
    /// into done).
    #[must_use]
    pub const fn is_done(self) -> bool {
        matches!(self, Self::Verified)
    }

    /// True when the outcome is unknown and requires human attention.
    #[must_use]
    pub const fn requires_attention(self) -> bool {
        matches!(self, Self::Ambiguous | Self::Failed)
    }

    /// Human-readable label for the UX. These strings are the
    /// trust-language contract: a UI MUST NOT render "done" for anything
    /// except Verified.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Planned => "Planned",
            Self::Attempted => "Attempted",
            Self::Executed => "Executed",
            Self::Verified => "Verified",
            Self::Ambiguous => "Ambiguous — outcome unknown",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempted_is_not_done() {
        assert!(!TrustLanguage::Attempted.is_done());
        assert!(!TrustLanguage::Executed.is_done());
        assert!(!TrustLanguage::Ambiguous.is_done());
        assert!(TrustLanguage::Verified.is_done());
    }

    #[test]
    fn labels_distinguish_states() {
        // §23.9: never collapse attempted into done.
        assert_ne!(
            TrustLanguage::Attempted.label(),
            TrustLanguage::Verified.label()
        );
        assert_ne!(
            TrustLanguage::Executed.label(),
            TrustLanguage::Verified.label()
        );
        assert!(TrustLanguage::Ambiguous.label().contains("unknown"));
    }
}
