//! Evals, observability, and workflow economics (spec 16).
//!
//! The north-star metric (§16.8) is cost per **verified** successful
//! workflow: the denominator is verified completion only — executor
//! success, model self-report, and unobserved effects never count
//! (§16.16: NO_EFFECT/AMBIGUOUS are not delivery). Human approvals that
//! policy intentionally requires are NOT human rescue (§16.10), so the
//! two are recorded separately and reported separately.
//!
//! Failure metrics use the canonical taxonomy from spec 18 (§16.6) —
//! there is no "model error" bucket.

pub mod economics;
pub mod metrics;
pub mod outcomes;
pub mod slo;

pub use economics::{WorkflowEconomics, YEAR_MONTHS};
pub use metrics::{
    CorpusContract, MetricStore, ProviderRequestStats, RunMetrics, VERIFIED_COMPLETION_ALPHA,
    VERIFIED_COMPLETION_CANARY, VERIFIED_COMPLETION_HARDENED,
};
pub use outcomes::StepOutcomeClass;
pub use slo::{
    evaluate_alerts, evaluate_slo, Alert, AlertThresholds, SloLevel, SloReport, SloViolation,
};
