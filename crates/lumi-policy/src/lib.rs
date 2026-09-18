//! Local policy gate. Cloud/model output is advisory until this layer permits it.

use lumi_protocol::{ActionRequest, ExecutionTier, RiskLevel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    RequireApproval { reason: &'static str },
    Deny { reason: &'static str },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultPolicy;

impl DefaultPolicy {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn evaluate(&self, request: &ActionRequest) -> Decision {
        if request.risk == RiskLevel::Destructive {
            return Decision::RequireApproval {
                reason: "destructive actions require explicit human approval",
            };
        }

        if request.risk == RiskLevel::ExternalSideEffect {
            return Decision::RequireApproval {
                reason: "externally visible side effects require explicit approval by default",
            };
        }

        if request.tier == ExecutionTier::Vision && request.risk != RiskLevel::ReadOnly {
            return Decision::RequireApproval {
                reason: "non-read-only vision actions are too fragile for silent execution",
            };
        }

        Decision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(tier: ExecutionTier, risk: RiskLevel) -> ActionRequest {
        ActionRequest::new("a-1", "wf-1", tier, risk, "fixture", "test")
    }

    #[test]
    fn read_only_semantic_action_is_allowed() {
        assert_eq!(
            DefaultPolicy::new().evaluate(&request(
                ExecutionTier::NativeSemantic,
                RiskLevel::ReadOnly
            )),
            Decision::Allow
        );
    }

    #[test]
    fn external_side_effect_requires_approval() {
        assert!(matches!(
            DefaultPolicy::new().evaluate(&request(
                ExecutionTier::BrowserSemantic,
                RiskLevel::ExternalSideEffect
            )),
            Decision::RequireApproval { .. }
        ));
    }

    #[test]
    fn non_read_only_vision_requires_approval() {
        assert!(matches!(
            DefaultPolicy::new().evaluate(&request(
                ExecutionTier::Vision,
                RiskLevel::Reversible
            )),
            Decision::RequireApproval { .. }
        ));
    }

    #[test]
    fn destructive_action_requires_approval() {
        assert!(matches!(
            DefaultPolicy::new().evaluate(&request(
                ExecutionTier::ConnectorApi,
                RiskLevel::Destructive
            )),
            Decision::RequireApproval { .. }
        ));
    }
}
