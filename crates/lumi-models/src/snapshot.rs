//! RouteSnapshot: the per task/turn routing record (spec 09 §9.16).

use serde::{Deserialize, Serialize};

/// Persisted routing decision for one task/turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteSnapshot {
    /// Driver family, e.g. `anthropic`.
    pub provider_driver: String,
    /// The concrete instance used.
    pub provider_instance_id: String,
    pub model: String,
    /// Router version that made the decision.
    pub router_version: String,
    /// Policy version in force at routing time.
    pub policy_version: String,
    /// Required capabilities that were matched.
    pub capability_match: Vec<String>,
    /// Privacy/data-egress classification of the request.
    pub data_classification: DataClassification,
    /// Region/service tier where relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// Privacy classification recorded with the route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataClassification {
    LocalOnly,
    ApprovedRegions,
    CloudAllowed,
}

/// Outcome of validating a persisted snapshot against *current* policy at
/// resume time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotValidation {
    /// Same driver+instance still satisfies current policy: reuse it.
    Reusable,
    /// Current policy no longer admits this route: re-evaluate from the
    /// candidate set. Never silently reroute outside the ORIGINAL
    /// envelope: re-evaluation uses the task's original constraints.
    ReEvaluate { reason: String },
}

impl RouteSnapshot {
    /// Validates a snapshot for resume against the CURRENT configuration
    /// (spec 09 §9.16: reload must not silently widen the envelope; a
    /// tightened policy forces explicit re-evaluation under the original
    /// constraints).
    #[must_use]
    pub fn validate_for_resume(
        &self,
        current_driver: &str,
        current_instance_active: bool,
        current_policy_version: &str,
    ) -> SnapshotValidation {
        if self.provider_driver != current_driver {
            return SnapshotValidation::ReEvaluate {
                reason: "driver removed from configuration".to_owned(),
            };
        }
        if !current_instance_active {
            return SnapshotValidation::ReEvaluate {
                reason: "provider instance revoked or paused".to_owned(),
            };
        }
        if self.policy_version != current_policy_version {
            return SnapshotValidation::ReEvaluate {
                reason: format!(
                    "policy version changed: snapshot {} vs current {current_policy_version}",
                    self.policy_version
                ),
            };
        }
        SnapshotValidation::Reusable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RouteSnapshot {
        RouteSnapshot {
            provider_driver: "anthropic".to_owned(),
            provider_instance_id: "inst-1".to_owned(),
            model: "claude-sonnet".to_owned(),
            router_version: "1".to_owned(),
            policy_version: "1.0.0".to_owned(),
            capability_match: vec!["text".to_owned(), "tool_use".to_owned()],
            data_classification: DataClassification::ApprovedRegions,
            region: Some("us".to_owned()),
        }
    }

    #[test]
    fn snapshot_round_trips() {
        let json = serde_json::to_string(&snapshot()).unwrap();
        let back: RouteSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, snapshot());
    }

    #[test]
    fn unchanged_policy_reuses_snapshot() {
        assert_eq!(
            snapshot().validate_for_resume("anthropic", true, "1.0.0"),
            SnapshotValidation::Reusable
        );
    }

    #[test]
    fn tightening_policy_forces_reevaluation() {
        // Config reload tightened policy: snapshot must NOT be silently
        // reused or widened.
        assert!(matches!(
            snapshot().validate_for_resume("anthropic", true, "2.0.0"),
            SnapshotValidation::ReEvaluate { .. }
        ));
        assert!(matches!(
            snapshot().validate_for_resume("openai", true, "1.0.0"),
            SnapshotValidation::ReEvaluate { .. }
        ));
        assert!(matches!(
            snapshot().validate_for_resume("anthropic", false, "1.0.0"),
            SnapshotValidation::ReEvaluate { .. }
        ));
    }
}
