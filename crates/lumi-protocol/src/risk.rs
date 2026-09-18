//! Canonical risk classes (AGENTS.md, spec 03 §3.2).
//!
//! Risk describes the real-world *effect* of an action, not how difficult
//! it is to perform. Policy units are business effects, never UI gestures.

use serde::{Deserialize, Serialize};

/// Canonical risk classification. Serialization is SCREAMING_SNAKE_CASE to
/// match the v1 wire format. Deserialization of unknown values fails
/// explicitly (spec 01 §1.15); there is no silent `Unknown` coercion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskClass {
    /// No observable side effect (read, query, observe).
    Read,
    /// Writes only inside the local controlled workspace; reversible.
    LocalWrite,
    /// Creates/updates state in an external system; visible to others.
    ExternalWrite,
    /// Sends communication to a person or channel.
    Communication,
    /// Moves data out of a boundary (export, share, download to unmanaged
    /// location).
    DataExport,
    /// Handles or resolves credentials.
    Credential,
    /// Moves or commits money or near-money value.
    Financial,
    /// Accepts terms, consent, or legal commitments.
    LegalConsent,
    /// Deletes or irreversibly destroys state.
    Destructive,
    /// Changes permissions, accounts, or security configuration.
    Admin,
}

impl RiskClass {
    /// True when executing this class can affect anything outside the local
    /// controlled boundary or is consequential enough to always be gated.
    #[must_use]
    pub const fn is_consequential(self) -> bool {
        !matches!(self, Self::Read | Self::LocalWrite)
    }

    /// Coarse ordering used for telemetry and reporting; the canonical
    /// order is also the ascending severity order of the enum declaration.
    #[must_use]
    pub const fn severity_rank(self) -> u8 {
        match self {
            Self::Read => 0,
            Self::LocalWrite => 1,
            Self::DataExport => 2,
            Self::ExternalWrite => 3,
            Self::Communication => 4,
            Self::Credential => 5,
            Self::Admin => 6,
            Self::Financial => 7,
            Self::LegalConsent => 8,
            Self::Destructive => 9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_format_is_screaming_snake() {
        assert_eq!(
            serde_json::to_string(&RiskClass::ExternalWrite).unwrap(),
            "\"EXTERNAL_WRITE\""
        );
        assert_eq!(
            serde_json::to_string(&RiskClass::LegalConsent).unwrap(),
            "\"LEGAL_CONSENT\""
        );
    }

    #[test]
    fn unknown_risk_fails_closed() {
        // Spec 01 §1.15: unknown required enum values MUST fail explicitly.
        assert!(serde_json::from_str::<RiskClass>("\"SPICY\"").is_err());
        assert!(serde_json::from_str::<RiskClass>("\"UNKNOWN\"").is_err());
    }

    #[test]
    fn consequentiality_matches_spec() {
        assert!(!RiskClass::Read.is_consequential());
        assert!(!RiskClass::LocalWrite.is_consequential());
        for risky in [
            RiskClass::ExternalWrite,
            RiskClass::Communication,
            RiskClass::DataExport,
            RiskClass::Credential,
            RiskClass::Financial,
            RiskClass::LegalConsent,
            RiskClass::Destructive,
            RiskClass::Admin,
        ] {
            assert!(risky.is_consequential());
        }
    }
}
