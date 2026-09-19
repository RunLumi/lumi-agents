//! Spec 16 §16.15 required proofs: metric calculations use VERIFIED
//! completion, and expected approvals are distinguished from rescue.

use lumi_evals::{
    evaluate_alerts, evaluate_slo, AlertThresholds, CorpusContract, MetricStore, RunMetrics,
    SloLevel, StepOutcomeClass,
};
use lumi_protocol::{ExecutionTier, FailureCategory};

fn run(run_id: &str, outcome: StepOutcomeClass) -> RunMetrics {
    let mut metrics = RunMetrics::new("wf", run_id);
    metrics.outcome = outcome;
    metrics
}

#[test]
fn verified_completion_uses_verified_denominator_only() {
    let mut store = MetricStore::new();
    // 10 runs: 9 delivered+verified, 1 executor "success" but AMBIGUOUS.
    for i in 0..9 {
        let mut r = run(&format!("r{i}"), StepOutcomeClass::Delivered);
        r.variable_cost_micro_usd = 1_000;
        store.record(r);
    }
    let mut ambiguous = run("r9", StepOutcomeClass::Ambiguous);
    ambiguous.variable_cost_micro_usd = 1_000; // cost still counts
    store.record(ambiguous);

    let aggregate = store.aggregate("wf", &CorpusContract::default());
    assert_eq!(aggregate.runs, 10);
    // 9/10 — the ambiguous run does NOT count as success.
    assert!((aggregate.verified_completion_rate() - 0.9).abs() < 1e-6);
    // Cost per verified success divides by 9, not 10: total 10_000 / 9.
    assert_eq!(
        aggregate.cost_per_verified_success_micro_usd(),
        Some(1_111) // floor(10000/9)
    );
}

#[test]
fn ambiguous_and_no_effect_never_count_as_success() {
    let mut store = MetricStore::new();
    store.record(run("a", StepOutcomeClass::NoEffect));
    store.record(run("b", StepOutcomeClass::Ambiguous));
    store.record(run("c", StepOutcomeClass::Error));
    store.record(run("d", StepOutcomeClass::Cancelled));
    let aggregate = store.aggregate("wf", &CorpusContract::default());
    assert_eq!(aggregate.verified_completions, 0);
    assert_eq!(aggregate.verified_completion_rate(), 0.0);
    // And cost per verified success is undefined, not zero.
    assert_eq!(aggregate.cost_per_verified_success_micro_usd(), None);
}

#[test]
fn expected_approvals_are_not_rescue() {
    let mut store = MetricStore::new();
    // 100 runs; every run had 1 expected approval; only 2 had unplanned
    // human rescue.
    for i in 0..100 {
        let mut r = run(&format!("r{i}"), StepOutcomeClass::Delivered);
        r.expected_approvals = 1;
        if i < 2 {
            r.unexpected_rescues = 1;
        }
        store.record(r);
    }
    let aggregate = store.aggregate("wf", &CorpusContract::default());
    assert_eq!(aggregate.expected_approvals, 100);
    assert_eq!(aggregate.unexpected_rescues, 2);
    assert!((aggregate.expected_approval_rate() - 1.0).abs() < 1e-6);
    assert!((aggregate.unexpected_rescue_rate() - 0.02).abs() < 1e-6);
    // Canary rescue gate: 2% < 5% passes.
    let report = evaluate_slo(SloLevel::CustomerCanary, &aggregate);
    let rescue_violations: Vec<_> = report
        .violations
        .iter()
        .filter(|v| v.requirement.contains("rescue"))
        .collect();
    assert!(rescue_violations.is_empty(), "{report:?}");
}

#[test]
fn alpha_gate_fails_without_evidence_or_completion() {
    let mut store = MetricStore::new();
    for i in 0..10 {
        store.record(run(&format!("r{i}"), StepOutcomeClass::Delivered));
    }
    let aggregate = store.aggregate("wf", &CorpusContract::default());
    let report = evaluate_slo(SloLevel::Alpha, &aggregate);
    assert!(!report.passed);
    assert!(report
        .violations
        .iter()
        .any(|v| v.requirement.contains("30 repeated runs")));
    // Completion itself (100%) passes; run count does not.

    // Now enough runs but one unauthorized side effect: hard fail.
    let mut store2 = MetricStore::new();
    for i in 0..30 {
        let mut r = run(&format!("s{i}"), StepOutcomeClass::Delivered);
        if i == 0 {
            r.unauthorized_side_effects = 1;
        }
        store2.record(r);
    }
    let aggregate2 = store2.aggregate("wf", &CorpusContract::default());
    let report2 = evaluate_slo(SloLevel::Alpha, &aggregate2);
    assert!(!report2.passed);
    assert!(report2
        .violations
        .iter()
        .any(|v| v.requirement.contains("zero unauthorized")));
}

#[test]
fn hardened_gate_requires_99_percent() {
    let mut store = MetricStore::new();
    // 100 runs, 99 verified: passes hardened.
    for i in 0..99 {
        store.record(run(&format!("h{i}"), StepOutcomeClass::Delivered));
    }
    store.record(run("h99", StepOutcomeClass::Error));
    let aggregate = store.aggregate("wf", &CorpusContract::default());
    assert!((aggregate.verified_completion_rate() - 0.99).abs() < 1e-6);
    let report = evaluate_slo(SloLevel::HardenedNarrow, &aggregate);
    assert!(report.passed, "{report:?}");

    // 98/100 fails.
    let mut store2 = MetricStore::new();
    for i in 0..98 {
        store2.record(run(&format!("k{i}"), StepOutcomeClass::Delivered));
    }
    store2.record(run("k98", StepOutcomeClass::Error));
    store2.record(run("k99", StepOutcomeClass::Ambiguous));
    let aggregate2 = store2.aggregate("wf", &CorpusContract::default());
    let report2 = evaluate_slo(SloLevel::HardenedNarrow, &aggregate2);
    assert!(!report2.passed);
}

#[test]
fn refusal_counts_only_when_contract_demands() {
    let mut store = MetricStore::new();
    store.record(run("a", StepOutcomeClass::Refused));
    let strict = store.aggregate("wf", &CorpusContract::default());
    assert_eq!(strict.verified_completions, 0);
    let adversarial = CorpusContract {
        refusal_counts_as_pass: true,
    };
    let passing = store.aggregate("wf", &adversarial);
    assert_eq!(passing.verified_completions, 1);
}

#[test]
fn failures_keep_canonical_categories() {
    // §16.6: no generic "model error" bucket.
    let mut store = MetricStore::new();
    let mut r = run("a", StepOutcomeClass::Error);
    r.record_failure(FailureCategory::BrowserSelector);
    r.record_failure(FailureCategory::BrowserSelector);
    r.record_failure(FailureCategory::ProviderRateLimit);
    r.record_tier(ExecutionTier::BrowserSemantic);
    store.record(r);
    let aggregate = store.aggregate("wf", &CorpusContract::default());
    assert_eq!(aggregate.failures[&FailureCategory::BrowserSelector], 2);
    assert_eq!(aggregate.failures[&FailureCategory::ProviderRateLimit], 1);
    assert_eq!(aggregate.tier_usage[&ExecutionTier::BrowserSemantic], 1);
}

#[test]
fn alerts_fire_on_unauthorized_ambiguity_cost_and_drop() {
    let mut store = MetricStore::new();
    // 10 runs, 4 verified (sharp drop: 40%), 1 unauthorized, high ambiguity.
    for i in 0..3 {
        let mut r = run(&format!("g{i}"), StepOutcomeClass::Delivered);
        r.variable_cost_micro_usd = 900_000_000; // very expensive
        store.record(r);
    }
    store.record(run("g4", StepOutcomeClass::Ambiguous));
    store.record(run("g5", StepOutcomeClass::Ambiguous));
    store.record(run("g6", StepOutcomeClass::Ambiguous));
    let mut unauthorized = run("g7", StepOutcomeClass::Delivered);
    unauthorized.unauthorized_side_effects = 1;
    store.record(unauthorized);
    store.record(run("g8", StepOutcomeClass::Error));
    store.record(run("g8b", StepOutcomeClass::Error));
    store.record(run("g9", StepOutcomeClass::Cancelled));

    let aggregate = store.aggregate("wf", &CorpusContract::default());
    let alerts = evaluate_alerts(&aggregate, &store, &AlertThresholds::default());
    let codes: Vec<&str> = alerts.iter().map(|a| a.code).collect();
    assert!(codes.contains(&"UNAUTHORIZED_SIDE_EFFECT"), "{codes:?}");
    assert!(codes.contains(&"HIGH_AMBIGUITY"), "{codes:?}");
    assert!(codes.contains(&"COST_RUNAWAY"), "{codes:?}");
    assert!(codes.contains(&"SHARP_SUCCESS_DROP"), "{codes:?}");
}

#[test]
fn alerts_stay_quiet_on_healthy_corpus() {
    let mut store = MetricStore::new();
    for i in 0..30 {
        let mut r = run(&format!("ok{i}"), StepOutcomeClass::Delivered);
        r.variable_cost_micro_usd = 1_000;
        r.expected_approvals = 1; // policy-intended approvals: quiet
        store.record(r);
    }
    let aggregate = store.aggregate("wf", &CorpusContract::default());
    let alerts = evaluate_alerts(&aggregate, &store, &AlertThresholds::default());
    assert!(alerts.is_empty(), "{alerts:?}");
    let report = evaluate_slo(SloLevel::Alpha, &aggregate);
    assert!(report.passed, "{report:?}");
}
