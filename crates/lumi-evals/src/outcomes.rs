//! Canonical executor-outcome classification (spec 16 §16.16).
//!
//! A declared REFUSED may be a passing result when the tested contract
//! requires safe refusal (e.g. adversarial suites). NO_EFFECT and
//! AMBIGUOUS MUST NOT count as successful delivery.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StepOutcomeClass {
    /// Effect took place and was verified.
    Delivered,
    /// The action was safely refused (passes only when the scenario
    /// contract requires refusal — see [`RefusalContract`]).
    Refused,
    /// No observable effect. Never a success.
    NoEffect,
    /// Outcome unknown. Never a success.
    Ambiguous,
    /// Failed with a canonical category. Never a success.
    Error,
    /// Cancelled before/during execution. Never a success.
    Cancelled,
}

/// Whether the scenario treats refusal as success (adversarial suites).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RefusalContract {
    #[serde(default)]
    pub refusal_counts_as_pass: bool,
}

impl StepOutcomeClass {
    /// Counts toward verified completion only under the scenario's
    /// refusal contract (spec 16 §16.16).
    #[must_use]
    pub const fn counts_as_verified_success(self, refusals: &RefusalContract) -> bool {
        match self {
            Self::Delivered => true,
            Self::Refused => refusals.refusal_counts_as_pass,
            Self::NoEffect | Self::Ambiguous | Self::Error | Self::Cancelled => false,
        }
    }

    /// True when the outcome must be investigated as a delivery failure.
    #[must_use]
    pub const fn is_delivery_failure(self) -> bool {
        matches!(self, Self::NoEffect | Self::Ambiguous | Self::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_effect_and_ambiguous_never_count_as_success() {
        let contract = RefusalContract::default();
        assert!(StepOutcomeClass::Delivered.counts_as_verified_success(&contract));
        assert!(!StepOutcomeClass::NoEffect.counts_as_verified_success(&contract));
        assert!(!StepOutcomeClass::Ambiguous.counts_as_verified_success(&contract));
        assert!(!StepOutcomeClass::Error.counts_as_verified_success(&contract));
        assert!(!StepOutcomeClass::Cancelled.counts_as_verified_success(&contract));
        assert!(!StepOutcomeClass::Refused.counts_as_verified_success(&contract));
    }

    #[test]
    fn refusal_passes_only_when_contract_requires_it() {
        let adversarial = RefusalContract {
            refusal_counts_as_pass: true,
        };
        assert!(StepOutcomeClass::Refused.counts_as_verified_success(&adversarial));
    }
}
