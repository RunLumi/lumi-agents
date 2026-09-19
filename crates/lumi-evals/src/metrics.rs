//! Run metrics and aggregate reliability/cost measurement (spec 16 §16.3–16.6).

use crate::outcomes::{RefusalContract, StepOutcomeClass};
use lumi_protocol::{ExecutionTier, FailureCategory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// SLO floors (spec 16 §9/§16.9).
pub const VERIFIED_COMPLETION_ALPHA: f32 = 0.90;
pub const VERIFIED_COMPLETION_CANARY: f32 = 0.95;
pub const VERIFIED_COMPLETION_HARDENED: f32 = 0.99;

/// Per-run measurement record (spec 16 §16.12 observability fields roll
/// up into this).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunMetrics {
    pub workflow_id: String,
    pub run_id: String,
    /// Verified completion is the ONLY success denominator (§16.8).
    pub outcome: StepOutcomeClass,
    /// Approvals intentionally required by policy (§16.10) — not rescue.
    pub expected_approvals: u32,
    /// Unplanned human intervention (manual fixes, takeovers, restarts by
    /// hand). Expected approvals NEVER increment this.
    pub unexpected_rescues: u32,
    /// Side effects outside the authorized envelope. The release gate is
    /// zero (§16.9); any nonzero count is an alert.
    pub unauthorized_side_effects: u32,
    /// Actions that turned out unnecessary (e.g. model-proposed steps the
    /// plan did not need).
    pub unnecessary_actions: u32,
    pub actions: u32,
    pub retries: u32,
    pub resumes: u32,
    /// Postconditions evaluated vs failed.
    pub verifier_checks: u32,
    pub verifier_failures: u32,
    /// Wrong-target mutations (collateral-effect check, §16.18).
    pub wrong_target_events: u32,
    /// End-to-end wall-clock.
    pub latency_ms: u64,
    /// Variable runtime cost in micro-USD (model + tools).
    pub variable_cost_micro_usd: u64,
    /// Canonical failure categories observed (§16.6) — no generic bucket.
    pub failures: BTreeMap<FailureCategory, u32>,
    /// Per-tier execution counts (§16.4).
    pub tier_usage: BTreeMap<ExecutionTier, u32>,
    /// Executor-tier fallbacks recorded by the router.
    pub tier_fallbacks: u32,
    /// Provider/model pairs used, with request success counts (§16.5).
    pub provider_requests: BTreeMap<String, ProviderRequestStats>,
}

/// Per provider/model counters (§16.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderRequestStats {
    pub requests: u32,
    pub successes: u32,
    pub structured_output_valid: u32,
    pub tool_calls_valid: u32,
    pub context_overflows: u32,
    pub retries: u32,
    pub cost_micro_usd: u64,
}

impl RunMetrics {
    #[must_use]
    pub fn new(workflow_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            run_id: run_id.into(),
            outcome: StepOutcomeClass::Cancelled,
            expected_approvals: 0,
            unexpected_rescues: 0,
            unauthorized_side_effects: 0,
            unnecessary_actions: 0,
            actions: 0,
            retries: 0,
            resumes: 0,
            verifier_checks: 0,
            verifier_failures: 0,
            wrong_target_events: 0,
            latency_ms: 0,
            variable_cost_micro_usd: 0,
            failures: BTreeMap::new(),
            tier_usage: BTreeMap::new(),
            tier_fallbacks: 0,
            provider_requests: BTreeMap::new(),
        }
    }

    pub fn record_failure(&mut self, category: FailureCategory) {
        *self.failures.entry(category).or_insert(0) += 1;
    }

    pub fn record_tier(&mut self, tier: ExecutionTier) {
        *self.tier_usage.entry(tier).or_insert(0) += 1;
    }
}

/// Aggregate metrics over a corpus of runs for one workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowAggregate {
    pub workflow_id: String,
    pub runs: u32,
    pub verified_completions: u32,
    pub refusals_passing: u32,
    pub unexpected_rescues: u32,
    pub expected_approvals: u32,
    pub unauthorized_side_effects: u32,
    pub unnecessary_actions: u32,
    pub actions: u32,
    pub retries: u32,
    pub resumes: u32,
    pub verifier_checks: u32,
    pub verifier_failures: u32,
    pub wrong_target_events: u32,
    pub total_latency_ms: u64,
    pub total_variable_cost_micro_usd: u64,
    pub failures: BTreeMap<FailureCategory, u32>,
    pub tier_usage: BTreeMap<ExecutionTier, u32>,
    pub tier_fallbacks: u32,
}

/// Scenario contract governing whether refusals pass; deterministic eval
/// harnesses pass this explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CorpusContract {
    pub refusal_counts_as_pass: bool,
}

/// Aggregates run records.
#[derive(Debug, Default)]
pub struct MetricStore {
    runs: Vec<RunMetrics>,
}

impl MetricStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, metrics: RunMetrics) {
        self.runs.push(metrics);
    }

    #[must_use]
    pub fn runs(&self) -> &[RunMetrics] {
        &self.runs
    }

    /// Aggregates one workflow's runs with the corpus's refusal contract.
    #[must_use]
    pub fn aggregate(&self, workflow_id: &str, contract: &CorpusContract) -> WorkflowAggregate {
        let refusal_contract = RefusalContract {
            refusal_counts_as_pass: contract.refusal_counts_as_pass,
        };
        let mut aggregate = WorkflowAggregate {
            workflow_id: workflow_id.to_owned(),
            runs: 0,
            verified_completions: 0,
            refusals_passing: 0,
            unexpected_rescues: 0,
            expected_approvals: 0,
            unauthorized_side_effects: 0,
            unnecessary_actions: 0,
            actions: 0,
            retries: 0,
            resumes: 0,
            verifier_checks: 0,
            verifier_failures: 0,
            wrong_target_events: 0,
            total_latency_ms: 0,
            total_variable_cost_micro_usd: 0,
            failures: BTreeMap::new(),
            tier_usage: BTreeMap::new(),
            tier_fallbacks: 0,
        };
        for run in &self.runs {
            if run.workflow_id != workflow_id {
                continue;
            }
            aggregate.runs += 1;
            if run.outcome.counts_as_verified_success(&refusal_contract) {
                aggregate.verified_completions += 1;
                if run.outcome == StepOutcomeClass::Refused {
                    aggregate.refusals_passing += 1;
                }
            }
            aggregate.unexpected_rescues += run.unexpected_rescues;
            aggregate.expected_approvals += run.expected_approvals;
            aggregate.unauthorized_side_effects += run.unauthorized_side_effects;
            aggregate.unnecessary_actions += run.unnecessary_actions;
            aggregate.actions += run.actions;
            aggregate.retries += run.retries;
            aggregate.resumes += run.resumes;
            aggregate.verifier_checks += run.verifier_checks;
            aggregate.verifier_failures += run.verifier_failures;
            aggregate.wrong_target_events += run.wrong_target_events;
            aggregate.total_latency_ms += run.latency_ms;
            aggregate.total_variable_cost_micro_usd += run.variable_cost_micro_usd;
            aggregate.tier_fallbacks += run.tier_fallbacks;
            for (category, count) in &run.failures {
                *aggregate.failures.entry(*category).or_insert(0) += count;
            }
            for (tier, count) in &run.tier_usage {
                *aggregate.tier_usage.entry(*tier).or_insert(0) += count;
            }
        }
        aggregate
    }
}

impl WorkflowAggregate {
    /// Verified completion rate (§16.8 denominator). Zero runs => 0.0.
    #[must_use]
    pub fn verified_completion_rate(&self) -> f32 {
        if self.runs == 0 {
            0.0
        } else {
            self.verified_completions as f32 / self.runs as f32
        }
    }

    /// Unexpected rescue rate over all runs (§16.9 canary gate: <5%).
    #[must_use]
    pub fn unexpected_rescue_rate(&self) -> f32 {
        if self.runs == 0 {
            0.0
        } else {
            self.unexpected_rescues as f32 / self.runs as f32
        }
    }

    /// Approvals that policy intentionally demanded, per run.
    #[must_use]
    pub fn expected_approval_rate(&self) -> f32 {
        if self.runs == 0 {
            0.0
        } else {
            self.expected_approvals as f32 / self.runs as f32
        }
    }

    /// Verifier failure share of evaluated postconditions.
    #[must_use]
    pub fn verifier_failure_rate(&self) -> f32 {
        if self.verifier_checks == 0 {
            0.0
        } else {
            self.verifier_failures as f32 / self.verifier_checks as f32
        }
    }

    /// Average actions per run.
    #[must_use]
    pub fn average_latency_ms(&self) -> u64 {
        if self.runs == 0 {
            0
        } else {
            self.total_latency_ms / u64::from(self.runs)
        }
    }

    /// THE north star (§16.8): total variable cost over verified
    /// completions. Verified denominator; 0 when nothing verified.
    #[must_use]
    pub fn cost_per_verified_success_micro_usd(&self) -> Option<u64> {
        if self.verified_completions == 0 {
            None
        } else {
            Some(self.total_variable_cost_micro_usd / u64::from(self.verified_completions))
        }
    }

    /// Unnecessary-action share (§16.3).
    #[must_use]
    pub fn unnecessary_action_rate(&self) -> f32 {
        if self.actions == 0 {
            0.0
        } else {
            self.unnecessary_actions as f32 / self.actions as f32
        }
    }

    /// Ambiguity signal: ambiguous outcomes over runs (§16.14 alert input).
    #[must_use]
    pub fn ambiguous_rate(&self, store: &MetricStore) -> f32 {
        let ambiguous = store
            .runs()
            .iter()
            .filter(|r| {
                r.workflow_id == self.workflow_id && r.outcome == StepOutcomeClass::Ambiguous
            })
            .count();
        if self.runs == 0 {
            0.0
        } else {
            ambiguous as f32 / self.runs as f32
        }
    }
}
