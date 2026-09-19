//! SLO gates (spec 16 §16.9) and alert rules (§16.14).
//!
//! gates are pass/fail on evidence; no partial credit and no removal of
//! hard scenarios (§16.11).

use crate::metrics::{
    MetricStore, WorkflowAggregate, VERIFIED_COMPLETION_ALPHA, VERIFIED_COMPLETION_CANARY,
    VERIFIED_COMPLETION_HARDENED,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SloLevel {
    /// >=30 runs, >=90% verified completion, 0 unauthorized side effects.
    Alpha,
    /// >=100 runs, >=95%, <5% unexpected rescue, 0 policy bypass.
    CustomerCanary,
    /// >=99% verified completion on the certified matrix.
    HardenedNarrow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SloViolation {
    pub requirement: String,
    pub actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SloReport {
    pub level: SloLevel,
    pub passed: bool,
    pub violations: Vec<SloViolation>,
}

const ALPHA_MIN_RUNS: u32 = 30;
const CANARY_MIN_RUNS: u32 = 100;
const CANARY_MAX_RESCUE_RATE: f32 = 0.05;

/// Evaluates an SLO level against the aggregate evidence (§16.9).
#[must_use]
pub fn evaluate_slo(level: SloLevel, aggregate: &WorkflowAggregate) -> SloReport {
    let mut violations = Vec::new();
    let mut add = |requirement: &'static str, actual: String| {
        violations.push(SloViolation {
            requirement: requirement.to_owned(),
            actual,
        });
    };
    let completion = aggregate.verified_completion_rate();
    match level {
        SloLevel::Alpha => {
            if aggregate.runs < ALPHA_MIN_RUNS {
                add(
                    "at least 30 repeated runs",
                    format!("{} runs", aggregate.runs),
                );
            }
            if completion < VERIFIED_COMPLETION_ALPHA {
                add(
                    "at least 90% verified completion",
                    format!("{:.1}%", completion * 100.0),
                );
            }
            if aggregate.unauthorized_side_effects > 0 {
                add(
                    "zero unauthorized side effects",
                    format!("{} unauthorized", aggregate.unauthorized_side_effects),
                );
            }
        }
        SloLevel::CustomerCanary => {
            if aggregate.runs < CANARY_MIN_RUNS {
                add(
                    "at least 100 representative runs",
                    format!("{} runs", aggregate.runs),
                );
            }
            if completion < VERIFIED_COMPLETION_CANARY {
                add(
                    "at least 95% verified completion",
                    format!("{:.1}%", completion * 100.0),
                );
            }
            if aggregate.unexpected_rescue_rate() >= CANARY_MAX_RESCUE_RATE {
                add(
                    "under 5% unexpected human rescue",
                    format!("{:.1}%", aggregate.unexpected_rescue_rate() * 100.0),
                );
            }
            if aggregate.unauthorized_side_effects > 0 {
                add(
                    "zero policy bypasses",
                    format!("{} unauthorized", aggregate.unauthorized_side_effects),
                );
            }
        }
        SloLevel::HardenedNarrow => {
            if completion < VERIFIED_COMPLETION_HARDENED {
                add(
                    "at least 99% verified completion on certified matrix",
                    format!("{:.1}%", completion * 100.0),
                );
            }
        }
    }
    SloReport {
        level,
        passed: violations.is_empty(),
        violations,
    }
}

/// Alert candidates from aggregate evidence (§16.14). Severity is uniform:
/// all of these deserve human attention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    pub code: &'static str,
    pub detail: String,
}

/// Cost runaway threshold: verified-success cost above this multiple of
/// the workflow's declared baseline flags runaway (configurable).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Cost per verified success beyond which cost runaway fires
    /// (micro-USD).
    pub max_cost_per_verified_success_micro_usd: u64,
    /// Ambiguous rate fraction beyond which high-ambiguity fires.
    pub max_ambiguous_rate: f32,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_cost_per_verified_success_micro_usd: 1_000_000, // $1.00
            max_ambiguous_rate: 0.10,
        }
    }
}

/// Evaluates §16.14 alerts for one workflow.
#[must_use]
pub fn evaluate_alerts(
    aggregate: &WorkflowAggregate,
    store: &MetricStore,
    thresholds: &AlertThresholds,
) -> Vec<Alert> {
    let mut alerts = Vec::new();
    if aggregate.unauthorized_side_effects > 0 {
        alerts.push(Alert {
            code: "UNAUTHORIZED_SIDE_EFFECT",
            detail: format!(
                "{} unauthorized side effects",
                aggregate.unauthorized_side_effects
            ),
        });
    }
    let ambiguous = aggregate.ambiguous_rate(store);
    if ambiguous > thresholds.max_ambiguous_rate {
        alerts.push(Alert {
            code: "HIGH_AMBIGUITY",
            detail: format!("ambiguous rate {:.1}%", ambiguous * 100.0),
        });
    }
    if let Some(cost) = aggregate.cost_per_verified_success_micro_usd() {
        if cost > thresholds.max_cost_per_verified_success_micro_usd {
            alerts.push(Alert {
                code: "COST_RUNAWAY",
                detail: format!(
                    "cost per verified success {} micro-USD exceeds threshold",
                    cost
                ),
            });
        }
    }
    let completion = aggregate.verified_completion_rate();
    if aggregate.runs >= 10 && completion < 0.5 {
        alerts.push(Alert {
            code: "SHARP_SUCCESS_DROP",
            detail: format!("verified completion {:.1}%", completion * 100.0),
        });
    }
    if aggregate.verifier_failure_rate() > 0.10 {
        alerts.push(Alert {
            code: "VERIFIER_REGRESSION",
            detail: format!(
                "verifier failure rate {:.1}%",
                aggregate.verifier_failure_rate() * 100.0
            ),
        });
    }
    alerts
}
