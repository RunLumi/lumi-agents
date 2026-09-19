//! The model router (spec 09 §9.7–9.8, §9.16).
//!
//! Routing order (first decisive constraint wins):
//! 1. tenant data policy (local-only etc.);
//! 2. local/cloud constraint;
//! 3. provider allowlist;
//! 4. region/data residency;
//! 5. capability match;
//! 6. measured reliability;
//! 7. latency;
//! 8. cost (price table).
//!
//! Fallback candidates are drawn from the *already policy-filtered* set,
//! so fallback can never violate local-only, allowlist, region, or
//! capability constraints (spec 09 §9.8). Privacy outranks cost by
//! construction: eligibility filters run before any cost comparison.

use crate::capabilities::ModelCapability;
use crate::error::ModelError;
use crate::instance::InstanceStatus;
use crate::request::ModelRequest;
use crate::snapshot::{DataClassification, RouteSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The routing-relevant view of tenant policy (spec 09 §9.7 inputs 1–4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantModelPolicy {
    pub policy_version: String,
    /// Force on-device execution only.
    pub local_only: bool,
    /// Providers this tenant permits (empty = none).
    pub provider_allowlist: BTreeSet<String>,
    /// Regions data may be processed in, when constrained.
    pub allowed_regions: Option<BTreeSet<String>>,
}

impl TenantModelPolicy {
    #[must_use]
    pub fn local_only(policy_version: impl Into<String>) -> Self {
        Self {
            policy_version: policy_version.into(),
            local_only: true,
            provider_allowlist: BTreeSet::new(),
            allowed_regions: None,
        }
    }

    #[must_use]
    pub fn cloud(
        policy_version: impl Into<String>,
        allowlist: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            policy_version: policy_version.into(),
            local_only: false,
            provider_allowlist: allowlist.into_iter().map(Into::into).collect(),
            allowed_regions: None,
        }
    }

    fn admits(&self, candidate: &Candidate) -> bool {
        // 1+2. Data policy / local-cloud constraint.
        if self.local_only && !candidate.local {
            return false;
        }
        // 3. Provider allowlist (local endpoints bypass vendor allowlists:
        // the constraint is about data destination, not vendor).
        if !self.local_only && !self.provider_allowlist.contains(&candidate.provider_name) {
            return false;
        }
        // 4. Region/residency.
        if let Some(allowed) = &self.allowed_regions {
            if !candidate.regions.iter().any(|r| allowed.contains(r)) {
                return false;
            }
        }
        true
    }
}

/// One candidate route the router may pick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub instance_id: String,
    pub driver: String,
    pub provider_name: String,
    pub model: String,
    pub local: bool,
    pub regions: Vec<String>,
    /// Measured verified success rate `[0,1]` (spec 09 §9.10).
    pub verified_success_rate: f32,
    /// Rolling average latency in ms.
    pub avg_latency_ms: u32,
    /// Blended price micro-USD per 1K tokens.
    pub price_per_1k_micro_usd: u64,
}

impl Candidate {
    #[must_use]
    pub fn new(instance_id: &str, driver: &str, provider_name: &str, model: &str) -> Self {
        Self {
            instance_id: instance_id.to_owned(),
            driver: driver.to_owned(),
            provider_name: provider_name.to_owned(),
            model: model.to_owned(),
            local: false,
            regions: Vec::new(),
            verified_success_rate: 1.0,
            avg_latency_ms: 0,
            price_per_1k_micro_usd: 0,
        }
    }

    pub fn local(mut self) -> Self {
        self.local = true;
        self.regions = vec!["local".to_owned()];
        self
    }

    pub fn with_regions(mut self, regions: &[&str]) -> Self {
        self.regions = regions.iter().map(|s| (*s).to_owned()).collect();
        self
    }

    pub fn with_measurements(mut self, rate: f32, latency_ms: u32) -> Self {
        self.verified_success_rate = rate;
        self.avg_latency_ms = latency_ms;
        self
    }

    pub fn with_price(mut self, micro_usd_per_1k: u64) -> Self {
        self.price_per_1k_micro_usd = micro_usd_per_1k;
        self
    }
}

/// The chosen route plus policy-legal fallbacks.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteChoice {
    pub primary: Candidate,
    /// Ordered fallbacks within the same privacy envelope (spec 09 §9.8).
    pub fallbacks: Vec<Candidate>,
}

/// Why routing failed for ALL candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    NoCandidate { reason: String },
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCandidate { reason } => write!(f, "no model route: {reason}"),
        }
    }
}

impl std::error::Error for RouteError {}

/// Full routing: eligibility (constraints 1–5) then selection (6–8).
///
/// `model_supports` consults the instance catalog for declared model
/// capabilities; unsupported capability = not eligible (never degraded).
///
/// # Errors
/// [`RouteError`] when nothing eligible supports the request.
pub fn route(
    candidates: &[Candidate],
    policy: &TenantModelPolicy,
    request: &ModelRequest,
    required: &BTreeSet<ModelCapability>,
    model_supports: impl Fn(&Candidate, ModelCapability) -> bool,
) -> Result<(RouteChoice, RouteSnapshot), RouteError> {
    let _ = request;
    let mut eligible: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| policy.admits(c))
        .filter(|c| required.iter().all(|cap| model_supports(c, *cap)))
        .collect();
    if eligible.is_empty() {
        return Err(RouteError::NoCandidate {
            reason: if policy.local_only {
                "no local candidate supports the required capabilities within tenant data policy"
                    .to_owned()
            } else {
                "no allowlisted candidate supports the required capabilities".to_owned()
            },
        });
    }
    // Selection: reliability, then latency, then price.
    eligible.sort_by(|a, b| {
        b.verified_success_rate
            .partial_cmp(&a.verified_success_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.avg_latency_ms.cmp(&b.avg_latency_ms))
            .then(a.price_per_1k_micro_usd.cmp(&b.price_per_1k_micro_usd))
    });
    let primary = eligible[0].clone();
    let fallbacks = eligible[1..].iter().map(|c| (*c).clone()).collect();
    let choice = RouteChoice { primary, fallbacks };

    let classification = match policy {
        TenantModelPolicy {
            local_only: true, ..
        } => DataClassification::LocalOnly,
        TenantModelPolicy {
            allowed_regions: Some(_),
            ..
        } => DataClassification::ApprovedRegions,
        TenantModelPolicy {
            allowed_regions: None,
            local_only: false,
            ..
        } => DataClassification::CloudAllowed,
    };
    let snapshot = RouteSnapshot {
        provider_driver: choice.primary.driver.clone(),
        provider_instance_id: choice.primary.instance_id.clone(),
        model: choice.primary.model.clone(),
        router_version: env!("CARGO_PKG_VERSION").to_owned(),
        policy_version: policy.policy_version.clone(),
        capability_match: required.iter().map(|c| format!("{c:?}")).collect(),
        data_classification: classification,
        region: choice.primary.regions.first().cloned(),
    };
    Ok((choice, snapshot))
}

/// Guards executing on a chosen route: the instance must still admit work
/// (spec 09 §9.19 revocation test).
pub fn ensure_admission(status: InstanceStatus) -> Result<(), ModelError> {
    if status == InstanceStatus::Active {
        Ok(())
    } else {
        Err(ModelError::new(
            lumi_protocol::FailureCategory::SecurityViolation,
            "provider instance does not admit new work",
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ModelRequest {
        ModelRequest::text(
            "req-1",
            crate::request::ModelRole::Planner,
            "any-model",
            "plan this",
        )
    }

    #[test]
    fn local_only_excludes_cloud_even_when_cheaper() {
        let policy = TenantModelPolicy::local_only("1.0.0");
        let candidates = vec![
            Candidate::new("i1", "openai", "openai", "gpt-4o-mini").with_price(1),
            Candidate::new("i2", "openai-compatible", "ollama", "llama3").local(),
        ];
        let required = BTreeSet::from([ModelCapability::Text]);
        let (choice, snapshot) =
            route(&candidates, &policy, &request(), &required, |_, _| true).unwrap();
        assert_eq!(choice.primary.instance_id, "i2");
        assert!(choice.fallbacks.is_empty(), "no cloud fallback may remain");
        assert_eq!(snapshot.data_classification, DataClassification::LocalOnly);
    }

    #[test]
    fn allowlist_blocks_unlisted_providers() {
        let policy = TenantModelPolicy::cloud("1.0.0", ["anthropic"]);
        let candidates = vec![
            Candidate::new("i1", "openai", "openai", "gpt-4o-mini"),
            Candidate::new("i2", "anthropic", "anthropic", "claude"),
        ];
        let required = BTreeSet::from([ModelCapability::Text]);
        let (choice, _) = route(&candidates, &policy, &request(), &required, |_, _| true).unwrap();
        assert_eq!(choice.primary.provider_name, "anthropic");
    }

    #[test]
    fn capability_miss_excludes_candidate() {
        let policy = TenantModelPolicy::cloud("1.0.0", ["openai"]);
        let candidates = vec![
            Candidate::new("i-text", "openai", "openai", "text-model"),
            Candidate::new("i-vision", "openai", "openai", "vision-model"),
        ];
        let required = BTreeSet::from([ModelCapability::Vision]);
        let (choice, _) = route(&candidates, &policy, &request(), &required, |c, cap| {
            c.instance_id == "i-vision" && cap == ModelCapability::Vision
        })
        .unwrap();
        assert_eq!(choice.primary.instance_id, "i-vision");
    }

    #[test]
    fn region_constraint_enforced() {
        let mut policy = TenantModelPolicy::cloud("1.0.0", ["openai"]);
        policy.allowed_regions = Some(BTreeSet::from(["eu".to_owned()]));
        let candidates = vec![
            Candidate::new("i-us", "openai", "openai", "m").with_regions(&["us"]),
            Candidate::new("i-eu", "openai", "openai", "m").with_regions(&["eu"]),
        ];
        let required = BTreeSet::new();
        let (choice, snapshot) =
            route(&candidates, &policy, &request(), &required, |_, _| true).unwrap();
        assert_eq!(choice.primary.instance_id, "i-eu");
        assert_eq!(snapshot.region.as_deref(), Some("eu"));
        assert_eq!(
            snapshot.data_classification,
            DataClassification::ApprovedRegions
        );
    }

    #[test]
    fn reliability_outranks_price() {
        let policy = TenantModelPolicy::cloud("1.0.0", ["openai"]);
        let candidates = vec![
            Candidate::new("i-cheap", "openai", "openai", "m")
                .with_measurements(0.8, 500)
                .with_price(1),
            Candidate::new("i-solid", "openai", "openai", "m")
                .with_measurements(0.99, 900)
                .with_price(50),
        ];
        let required = BTreeSet::new();
        let (choice, _) = route(&candidates, &policy, &request(), &required, |_, _| true).unwrap();
        assert_eq!(choice.primary.instance_id, "i-solid");
        // Fallback is the still-eligible cheaper one.
        assert_eq!(choice.fallbacks.len(), 1);
        assert_eq!(choice.fallbacks[0].instance_id, "i-cheap");
    }

    #[test]
    fn no_candidate_names_the_constraint() {
        let policy = TenantModelPolicy::local_only("1.0.0");
        let candidates = vec![Candidate::new("i1", "openai", "openai", "m")];
        let required = BTreeSet::new();
        let err = route(&candidates, &policy, &request(), &required, |_, _| true).unwrap_err();
        assert!(err.to_string().contains("local"), "{err}");
    }
}
