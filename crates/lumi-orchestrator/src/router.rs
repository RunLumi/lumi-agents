//! Execution-tier routing (spec 05).
//!
//! The router chooses the least fragile execution tier that can satisfy an
//! action: the highest-semantic *eligible* tier wins (spec 05 §5.4) — a
//! lower tier is never selected merely because a model requested it.
//! Fallback stays within the action's allowed tiers, records its reason,
//! and a vision fallback can never bypass approval for consequential
//! actions (that decision is re-run through policy with the effective
//! tier).

use lumi_protocol::{ActionProposal, Capability, ExecutionTier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Health of one registered executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutorHealth {
    Healthy,
    Degraded,
    Unavailable,
}

/// Measured reliability of one executor (rolling, per capability).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Reliability {
    /// Verified completion rate in `[0,1]` from the eval/telemetry store.
    pub verified_success_rate: f32,
    /// Rolling average end-to-end latency in ms.
    pub avg_latency_ms: u32,
}

impl Default for Reliability {
    fn default() -> Self {
        // No failure data yet: a registry-registered executor is trusted
        // until measurements say otherwise. Only explicitly measured low
        // rates trip the fallback floor.
        Self {
            verified_success_rate: 1.0,
            avg_latency_ms: 0,
        }
    }
}

/// One registered executor able to serve a set of capabilities on its tier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutorDescriptor {
    pub tier: ExecutionTier,
    /// Adapter identity, e.g. `playwright`, `http-connector`.
    pub adapter: String,
    /// Adapter version for audit records.
    pub version: String,
    /// Capabilities this executor can serve on this tier.
    pub capabilities: BTreeSet<Capability>,
    pub health: ExecutorHealth,
    pub reliability: Reliability,
}

impl ExecutorDescriptor {
    #[must_use]
    pub fn new(
        tier: ExecutionTier,
        adapter: &str,
        version: &str,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Self {
        Self {
            tier,
            adapter: adapter.to_owned(),
            version: version.to_owned(),
            capabilities: capabilities.into_iter().collect(),
            health: ExecutorHealth::Healthy,
            reliability: Reliability::default(),
        }
    }

    /// Marks the executor below the reliability bar (fallback trigger,
    /// spec 05 §5.5).
    pub fn with_reliability(mut self, reliability: Reliability) -> Self {
        self.reliability = reliability;
        self
    }

    pub fn with_health(mut self, health: ExecutorHealth) -> Self {
        self.health = health;
        self
    }
}

/// Router policy inputs that can restrict tiers beyond the action.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierPolicy {
    /// Tiers this deployment/tenant forbids entirely (e.g. policy denies
    /// Vision). Denied tiers fail selection rather than fall back.
    pub denied_tiers: BTreeSet<ExecutionTier>,
}

impl TierPolicy {
    #[must_use]
    pub fn denying(tiers: impl IntoIterator<Item = ExecutionTier>) -> Self {
        Self {
            denied_tiers: tiers.into_iter().collect(),
        }
    }
}

/// Why selection failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    /// No registered executor serves this capability at any allowed tier.
    NoExecutorForCapability,
    /// Executors exist but all are unavailable/degraded beyond use.
    AllExecutorsUnavailable,
    /// The only serving tiers are policy-denied. Fails explicitly instead
    /// of falling through to a forbidden tier (spec 05 §5.12).
    TierDenied { tier: ExecutionTier },
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoExecutorForCapability => {
                write!(f, "no executor serves this capability at an allowed tier")
            }
            Self::AllExecutorsUnavailable => write!(f, "all candidate executors are unavailable"),
            Self::TierDenied { tier } => {
                write!(f, "tier {tier:?} is denied by policy; not falling back")
            }
        }
    }
}

impl std::error::Error for SelectionError {}

/// The chosen executor for one action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub tier: ExecutionTier,
    pub adapter: String,
    pub version: String,
    /// Set when the selected tier is a *fallback* from the preferred tier.
    pub fallback_reason: Option<String>,
}

/// Minimum reliability for the highest-semantic tier; below it, fallback
/// MAY occur (spec 05 §5.5).
const RELIABILITY_FLOOR: f32 = 0.7;

/// Selects the executor for `action`.
///
/// Rules (spec 05):
/// - candidates are executors whose tier is in the action's
///   `allowed_tiers` (empty = all tiers allowed), that serve the
///   capability, and that are not policy-denied;
/// - denied tiers fail explicitly — no silent fallback into them;
/// - the highest-semantic (lowest `ExecutionTier`) healthy candidate wins;
/// - a candidate below the reliability floor loses to the next tier,
///   recording a fallback reason;
/// - the model's `preferred_tier` cannot bypass higher-semantic tiers:
///   selection ignores it except for tie-breaking among equals.
///
/// # Errors
/// Returns [`SelectionError`] when no eligible executor exists.
pub fn select(
    action: &ActionProposal,
    executors: &[ExecutorDescriptor],
    tier_policy: &TierPolicy,
) -> Result<Selection, SelectionError> {
    const ALL_TIERS: [ExecutionTier; 5] = [
        ExecutionTier::ConnectorApi,
        ExecutionTier::BrowserSemantic,
        ExecutionTier::NativeSemantic,
        ExecutionTier::AppAdapter,
        ExecutionTier::Vision,
    ];

    let serving: Vec<&ExecutorDescriptor> = executors
        .iter()
        .filter(|e| e.capabilities.contains(&action.capability))
        .collect();
    if serving.is_empty() {
        return Err(SelectionError::NoExecutorForCapability);
    }

    let allowed_tiers: BTreeSet<ExecutionTier> =
        if action.execution_preferences.allowed_tiers.is_empty() {
            ALL_TIERS.into_iter().collect()
        } else {
            action
                .execution_preferences
                .allowed_tiers
                .iter()
                .copied()
                .collect()
        };

    // Semantic preference order (enum order): highest-semantic first.
    let mut ordered: Vec<&ExecutorDescriptor> = serving
        .iter()
        .copied()
        .filter(|e| allowed_tiers.contains(&e.tier))
        .collect();
    ordered.sort_by_key(|e| e.tier);
    if ordered.is_empty() {
        return Err(SelectionError::NoExecutorForCapability);
    }

    // First usable executor at the highest-semantic tier wins; denied
    // tiers fail explicitly mid-scan (no silent fallback into them);
    // unavailable/low-reliability candidates are skipped with the reason
    // recorded.
    let mut fallback_reason: Option<String> = None;
    for executor in &ordered {
        if tier_policy.denied_tiers.contains(&executor.tier) {
            return Err(SelectionError::TierDenied {
                tier: executor.tier,
            });
        }
        match executor.health {
            ExecutorHealth::Unavailable => {
                fallback_reason
                    .get_or_insert_with(|| format!("tier {:?} unavailable", executor.tier));
                continue;
            }
            ExecutorHealth::Healthy
                if executor.reliability.verified_success_rate >= RELIABILITY_FLOOR =>
            {
                return Ok(finish(executor, fallback_reason));
            }
            ExecutorHealth::Healthy | ExecutorHealth::Degraded => {
                fallback_reason.get_or_insert_with(|| {
                    format!(
                        "tier {:?} degraded or below reliability floor",
                        executor.tier
                    )
                });
                continue;
            }
        }
    }

    // Nothing above the floor: fail rather than silently use a weak tier.
    Err(SelectionError::AllExecutorsUnavailable)
}

fn finish(executor: &ExecutorDescriptor, fallback_reason: Option<String>) -> Selection {
    Selection {
        tier: executor.tier,
        adapter: executor.adapter.clone(),
        version: executor.version.clone(),
        fallback_reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumi_protocol::{
        ActionId, AuthenticationStrength, Principal, PrincipalId, PrincipalKind, ResourceRef,
        ResourceType, RiskClass, RunId, Target, TaskId, TenantId, Timestamp,
    };

    fn action(cap: &'static str, tiers: Vec<ExecutionTier>) -> ActionProposal {
        let mut builder = ActionProposal::builder(
            ActionId::parse("a-1").unwrap(),
            TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(Timestamp::UNIX_EPOCH),
                authentication_strength: Some(AuthenticationStrength::Mfa),
            },
            Capability::well_known(cap),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "x".to_owned(),
                sensitivity: None,
            },
            Target::canonical("mailto:c@example.test"),
            "send_customer_email",
            RiskClass::Communication,
        )
        .unwrap();
        builder.execution_preferences.allowed_tiers = tiers;
        builder
    }

    fn executor(
        tier: ExecutionTier,
        adapter: &str,
        cap: &'static str,
        rate: f32,
        health: ExecutorHealth,
    ) -> ExecutorDescriptor {
        ExecutorDescriptor::new(tier, adapter, "1.0.0", [Capability::well_known(cap)])
            .with_reliability(Reliability {
                verified_success_rate: rate,
                avg_latency_ms: 100,
            })
            .with_health(health)
    }

    #[test]
    fn api_preferred_over_browser_when_both_healthy() {
        let executors = vec![
            executor(
                ExecutionTier::BrowserSemantic,
                "playwright",
                lumi_protocol::capabilities::EMAIL_SEND,
                0.95,
                ExecutorHealth::Healthy,
            ),
            executor(
                ExecutionTier::ConnectorApi,
                "http-connector",
                lumi_protocol::capabilities::EMAIL_SEND,
                0.95,
                ExecutorHealth::Healthy,
            ),
        ];
        let selection = select(
            &action(
                lumi_protocol::capabilities::EMAIL_SEND,
                vec![ExecutionTier::ConnectorApi, ExecutionTier::BrowserSemantic],
            ),
            &executors,
            &TierPolicy::default(),
        )
        .unwrap();
        assert_eq!(selection.tier, ExecutionTier::ConnectorApi);
        assert_eq!(selection.adapter, "http-connector");
        assert!(selection.fallback_reason.is_none());
    }

    #[test]
    fn browser_preferred_over_vision() {
        let executors = vec![
            executor(
                ExecutionTier::Vision,
                "vision",
                lumi_protocol::capabilities::EMAIL_SEND,
                0.95,
                ExecutorHealth::Healthy,
            ),
            executor(
                ExecutionTier::BrowserSemantic,
                "playwright",
                lumi_protocol::capabilities::EMAIL_SEND,
                0.95,
                ExecutorHealth::Healthy,
            ),
        ];
        let selection = select(
            &action(
                lumi_protocol::capabilities::EMAIL_SEND,
                vec![ExecutionTier::BrowserSemantic, ExecutionTier::Vision],
            ),
            &executors,
            &TierPolicy::default(),
        )
        .unwrap();
        assert_eq!(selection.tier, ExecutionTier::BrowserSemantic);
    }

    #[test]
    fn model_preference_cannot_force_lower_tier() {
        // The action "prefers" Vision, but a healthy connector exists and
        // is allowed: spec 05 §5.4 — no lower tier on model request.
        let mut a = action(
            lumi_protocol::capabilities::EMAIL_SEND,
            vec![ExecutionTier::ConnectorApi, ExecutionTier::Vision],
        );
        a.execution_preferences.preferred_tier = Some(ExecutionTier::Vision);
        let executors = vec![executor(
            ExecutionTier::ConnectorApi,
            "http-connector",
            lumi_protocol::capabilities::EMAIL_SEND,
            0.95,
            ExecutorHealth::Healthy,
        )];
        let selection = select(&a, &executors, &TierPolicy::default()).unwrap();
        assert_eq!(selection.tier, ExecutionTier::ConnectorApi);
    }

    #[test]
    fn policy_denied_tier_fails_explicitly() {
        let executors = vec![executor(
            ExecutionTier::BrowserSemantic,
            "playwright",
            lumi_protocol::capabilities::EMAIL_SEND,
            0.95,
            ExecutorHealth::Healthy,
        )];
        let err = select(
            &action(
                lumi_protocol::capabilities::EMAIL_SEND,
                vec![ExecutionTier::BrowserSemantic],
            ),
            &executors,
            &TierPolicy::denying([ExecutionTier::BrowserSemantic]),
        )
        .unwrap_err();
        assert_eq!(
            err,
            SelectionError::TierDenied {
                tier: ExecutionTier::BrowserSemantic
            }
        );
    }

    #[test]
    fn degraded_tiers_are_not_silently_used() {
        // Degraded primary with nothing else: explicit failure, not a
        // silent weak selection.
        let executors = vec![executor(
            ExecutionTier::ConnectorApi,
            "http-connector",
            lumi_protocol::capabilities::EMAIL_SEND,
            0.95,
            ExecutorHealth::Degraded,
        )];
        let err = select(
            &action(
                lumi_protocol::capabilities::EMAIL_SEND,
                vec![ExecutionTier::ConnectorApi],
            ),
            &executors,
            &TierPolicy::default(),
        )
        .unwrap_err();
        assert_eq!(err, SelectionError::AllExecutorsUnavailable);
    }

    #[test]
    fn fallback_reason_recorded_when_primary_unavailable() {
        let executors = vec![
            executor(
                ExecutionTier::ConnectorApi,
                "http-connector",
                lumi_protocol::capabilities::EMAIL_SEND,
                0.95,
                ExecutorHealth::Unavailable,
            ),
            executor(
                ExecutionTier::BrowserSemantic,
                "playwright",
                lumi_protocol::capabilities::EMAIL_SEND,
                0.95,
                ExecutorHealth::Healthy,
            ),
        ];
        let selection = select(
            &action(
                lumi_protocol::capabilities::EMAIL_SEND,
                vec![ExecutionTier::ConnectorApi, ExecutionTier::BrowserSemantic],
            ),
            &executors,
            &TierPolicy::default(),
        )
        .unwrap();
        assert_eq!(selection.tier, ExecutionTier::BrowserSemantic);
        assert!(selection.fallback_reason.is_some());
    }

    #[test]
    fn below_reliability_floor_falls_back() {
        let executors = vec![
            executor(
                ExecutionTier::ConnectorApi,
                "http-connector",
                lumi_protocol::capabilities::EMAIL_SEND,
                0.4,
                ExecutorHealth::Healthy,
            ),
            executor(
                ExecutionTier::BrowserSemantic,
                "playwright",
                lumi_protocol::capabilities::EMAIL_SEND,
                0.95,
                ExecutorHealth::Healthy,
            ),
        ];
        let selection = select(
            &action(
                lumi_protocol::capabilities::EMAIL_SEND,
                vec![ExecutionTier::ConnectorApi, ExecutionTier::BrowserSemantic],
            ),
            &executors,
            &TierPolicy::default(),
        )
        .unwrap();
        assert_eq!(selection.tier, ExecutionTier::BrowserSemantic);
        assert!(selection.fallback_reason.is_some());
    }

    #[test]
    fn unsupported_tier_fails() {
        let executors = vec![executor(
            ExecutionTier::BrowserSemantic,
            "playwright",
            lumi_protocol::capabilities::EMAIL_SEND,
            0.95,
            ExecutorHealth::Healthy,
        )];
        // Action allows only a tier nothing serves.
        let err = select(
            &action(
                lumi_protocol::capabilities::EMAIL_SEND,
                vec![ExecutionTier::NativeSemantic],
            ),
            &executors,
            &TierPolicy::default(),
        )
        .unwrap_err();
        assert_eq!(err, SelectionError::NoExecutorForCapability);
    }

    #[test]
    fn no_executor_for_capability_fails() {
        let executors = vec![executor(
            ExecutionTier::BrowserSemantic,
            "playwright",
            lumi_protocol::capabilities::FILES_READ,
            0.95,
            ExecutorHealth::Healthy,
        )];
        let err = select(
            &action(lumi_protocol::capabilities::EMAIL_SEND, vec![]),
            &executors,
            &TierPolicy::default(),
        )
        .unwrap_err();
        assert_eq!(err, SelectionError::NoExecutorForCapability);
    }

    #[test]
    fn all_unavailable_fails() {
        let executors = vec![executor(
            ExecutionTier::ConnectorApi,
            "http-connector",
            lumi_protocol::capabilities::EMAIL_SEND,
            0.95,
            ExecutorHealth::Unavailable,
        )];
        let err = select(
            &action(lumi_protocol::capabilities::EMAIL_SEND, vec![]),
            &executors,
            &TierPolicy::default(),
        )
        .unwrap_err();
        assert_eq!(err, SelectionError::AllExecutorsUnavailable);
    }
}
