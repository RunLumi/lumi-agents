//! Evidence requirements and references (spec 11 §11.4).
//!
//! Evidence is minimal data used to explain or verify an action/outcome.
//! The preferred order is structured state first, screenshots last.

use crate::ids::EvidenceId;
use serde::{Deserialize, Serialize};

/// What evidence an action requires, in preference order (spec 11 §11.4):
/// prefer structured remote/local state; full screenshots only when
/// necessary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceRequirement {
    /// Structured remote/local state captured at verification time.
    StructuredState,
    /// A durable resource id/reference created or mutated by the action.
    ResourceReference,
    /// Content checksum.
    Checksum,
    /// Before/after field diff.
    Diff,
    /// Bounded log excerpt.
    LogExcerpt,
    /// A cropped/selected screenshot (not continuous capture).
    SelectiveScreenshot,
    /// Full screenshot; only when nothing else can prove the claim.
    FullScreenshot,
}

impl EvidenceRequirement {
    /// Evidence-quality rank; lower is stronger/more preferred.
    #[must_use]
    pub const fn preference_rank(self) -> u8 {
        match self {
            Self::StructuredState => 0,
            Self::ResourceReference => 1,
            Self::Checksum => 2,
            Self::Diff => 3,
            Self::LogExcerpt => 4,
            Self::SelectiveScreenshot => 5,
            Self::FullScreenshot => 6,
        }
    }
}

/// Reference to a persisted evidence record. Records themselves live in the
/// audit ledger (spec 11); the protocol only carries references so action
/// messages stay small and secret-free.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub evidence_id: EvidenceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<crate::resource::SensitivityLabel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_order_matches_spec() {
        let ranked = [
            EvidenceRequirement::StructuredState,
            EvidenceRequirement::ResourceReference,
            EvidenceRequirement::Checksum,
            EvidenceRequirement::Diff,
            EvidenceRequirement::LogExcerpt,
            EvidenceRequirement::SelectiveScreenshot,
            EvidenceRequirement::FullScreenshot,
        ];
        for pair in ranked.windows(2) {
            assert!(pair[0].preference_rank() < pair[1].preference_rank());
        }
    }

    #[test]
    fn evidence_ref_serde() {
        let r = EvidenceRef {
            evidence_id: crate::ids::EvidenceId::parse("ev-1").unwrap(),
            sensitivity: Some(crate::resource::SensitivityLabel::Internal),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"ev-1\""));
        assert_eq!(serde_json::from_str::<EvidenceRef>(&json).unwrap(), r);
    }
}
