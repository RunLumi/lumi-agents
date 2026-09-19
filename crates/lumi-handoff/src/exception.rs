//! Exception card: what blocked, why, safe choices (§23.5).

use serde::{Deserialize, Serialize};

/// A structured exception shown to the user (§23.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionCard {
    /// What blocked the task, in plain language.
    pub what_blocked: String,
    /// Why it happened (category + detail).
    pub why: String,
    /// The safe choices available to the user.
    pub safe_choices: Vec<SafeChoice>,
    /// Consequences of the current state (e.g. "no side effects so far").
    pub consequences: String,
    /// Evidence refs the user can inspect.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    /// Suggested next step.
    pub suggested_next_step: String,
}

/// One safe action the user can take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeChoice {
    pub label: String,
    /// What this choice does.
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exception_card_shows_safe_choices_and_evidence() {
        let card = ExceptionCard {
            what_blocked: "Submit action timed out after clicking Submit".to_owned(),
            why: "The confirmation page did not appear within 30 seconds. \
                  The submission may or may not have been processed."
                .to_owned(),
            safe_choices: vec![
                SafeChoice {
                    label: "Check the website manually".to_owned(),
                    description:
                        "Open the CRM in a browser and verify whether the invoice was submitted"
                            .to_owned(),
                },
                SafeChoice {
                    label: "Retry submission".to_owned(),
                    description: "Only safe if you verified the first attempt did not go through"
                        .to_owned(),
                },
                SafeChoice {
                    label: "Cancel task".to_owned(),
                    description:
                        "No side effects have been confirmed; the task ends without changes"
                            .to_owned(),
                },
            ],
            consequences:
                "Unknown: the invoice may have been submitted. Do NOT retry without checking."
                    .to_owned(),
            evidence_refs: vec![
                "ev-screenshot-1".to_owned(),
                "ev-journal-proposed".to_owned(),
            ],
            suggested_next_step: "Check the CRM portal manually before choosing".to_owned(),
        };
        assert_eq!(card.safe_choices.len(), 3);
        assert!(card.consequences.contains("may have been"));
        assert!(!card.evidence_refs.is_empty());
    }
}
