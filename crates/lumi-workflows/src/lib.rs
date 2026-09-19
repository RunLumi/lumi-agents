//! Repeatable workflow-pack evaluation harness (issue #9, spec 16/12).
//!
//! Loads pack definitions from `packs/<name>/pack.json`, executes them
//! repeatedly through the same orchestrator gate used in production
//! (policy -> approval -> journal -> verifier -> audit), records
//! [`lumi_evals::RunMetrics`] per run, and produces a certification report
//! with SLO evidence and economics derivations.
//!
//! The harness ships with deterministic fixture executors so the whole
//! certification corpus is reproducible in CI. `evaluate_pack` is fixture-only;
//! live runs require a real adapter and independent postcondition environment.

pub mod fixtures;
pub mod roles;

use lumi_evals::{DerivedEconomics, MetricStore, SloLevel, SloReport, WorkflowEconomics};
use lumi_packs::WorkflowPack;
use std::path::{Path, PathBuf};

pub use roles::{load_role_pack, repo_roles_dir, PreparedRoleRun, ResolvedRolePack, RoleRunError};

/// Loads a pack definition from a `pack.json` file.
///
/// # Errors
/// IO/parse errors; callers treat an invalid pack as a hard certification
/// failure.
pub fn load_pack(path: &Path) -> Result<WorkflowPack, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let pack: WorkflowPack =
        serde_json::from_str(&text).map_err(|e| format!("parsing {}: {e}", path.display()))?;
    pack.validate()
        .map_err(|violations| format!("pack invalid: {}", violations.join("; ")))?;
    Ok(pack)
}

/// Loads the three v1 cost-center packs from the repository's `packs/`
/// directory.
///
/// # Errors
/// Any load/validation failure.
pub fn load_v1_packs(packs_dir: &Path) -> Result<Vec<WorkflowPack>, String> {
    let mut packs = Vec::new();
    for name in [
        "invoice-reconciliation",
        "quote-followup",
        "expense-report-audit",
    ] {
        packs.push(load_pack(&packs_dir.join(name).join("pack.json"))?);
    }
    Ok(packs)
}

/// Certification evidence for one workflow pack.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CertificationReport {
    /// This harness never produces live-system certification evidence.
    pub evidence_class: &'static str,
    /// Baselines below are declared hypotheses, never measured customer ROI.
    pub economics_measured: bool,
    pub workflow_id: String,
    pub runs: u32,
    pub verified_completions: u32,
    pub verified_completion_rate: f32,
    pub expected_approvals: u32,
    pub unexpected_rescues: u32,
    pub unauthorized_side_effects: u32,
    pub cost_per_verified_success_micro_usd: Option<u64>,
    pub slo: SloReport,
    pub economics_declared: WorkflowEconomics,
    pub economics_derived: DerivedEconomics,
}

/// Runs one pack `runs` times through the orchestrator with the pack's
/// fixture executor, recording full metrics.
pub fn evaluate_pack(pack: &WorkflowPack, packs_dir: &Path, runs: u32) -> CertificationReport {
    let mut store = MetricStore::new();
    for i in 0..runs {
        let metrics = fixtures::run_single_scenario(pack, packs_dir, i);
        store.record(metrics);
    }
    let contract = lumi_evals::CorpusContract::default();
    let aggregate = store.aggregate(&pack.manifest.pack_id, &contract);
    let slo = lumi_evals::evaluate_slo(SloLevel::Alpha, &aggregate);
    let declared = WorkflowEconomics {
        runs_per_month: pack.manifest.economics.runs_per_month,
        baseline_manual_minutes_per_run: pack.manifest.economics.manual_minutes_per_run,
        residual_human_minutes_per_run: pack.manifest.economics.residual_human_minutes_per_run,
        loaded_labor_cost_per_hour_micro_usd: pack
            .manifest
            .economics
            .loaded_labor_cost_per_hour_micro_usd,
        runtime_variable_cost_per_run_micro_usd: pack
            .manifest
            .economics
            .runtime_variable_cost_per_run_micro_usd,
        support_cost_monthly_micro_usd: 0,
        implementation_cost_micro_usd: u64::from(pack.manifest.economics.implementation_hours)
            * 60_000_000, // Fixture hypothesis: $60/h implementation labor.
        cycle_time_before_minutes: pack.manifest.economics.manual_minutes_per_run,
        cycle_time_after_minutes: pack.manifest.economics.residual_human_minutes_per_run,
    };
    CertificationReport {
        evidence_class: "FIXTURE",
        economics_measured: false,
        workflow_id: pack.manifest.pack_id.clone(),
        runs: aggregate.runs,
        verified_completions: aggregate.verified_completions,
        verified_completion_rate: aggregate.verified_completion_rate(),
        expected_approvals: aggregate.expected_approvals,
        unexpected_rescues: aggregate.unexpected_rescues,
        unauthorized_side_effects: aggregate.unauthorized_side_effects,
        cost_per_verified_success_micro_usd: aggregate.cost_per_verified_success_micro_usd(),
        slo,
        economics_derived: declared.derived(),
        economics_declared: declared,
    }
}

/// Repository-relative packs directory for tests and CLI use.
#[must_use]
pub fn repo_packs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs")
}
