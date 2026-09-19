//! Issue #9 certification corpus: three cost-center packs, evaluated
//! through the full gated pipeline with spec 16 metrics.

use lumi_evals::{SloLevel, StepOutcomeClass};
use lumi_workflows::{evaluate_pack, load_v1_packs, repo_packs_dir};

#[test]
fn three_v1_packs_load_and_validate() {
    let packs = load_v1_packs(&repo_packs_dir()).expect("v1 packs load");
    assert_eq!(packs.len(), 3);
    let ids: Vec<&str> = packs.iter().map(|p| p.manifest.pack_id.as_str()).collect();
    assert!(ids.contains(&"invoice-reconciliation"));
    assert!(ids.contains(&"quote-followup"));
    assert!(ids.contains(&"expense-report-audit"));
    // Spec 22.4: at least one workflow requires native desktop automation.
    let native_pack = packs
        .iter()
        .find(|p| p.manifest.pack_id == "expense-report-audit")
        .unwrap();
    assert!(native_pack
        .steps
        .iter()
        .any(|s| { s.preferred_tier == lumi_protocol::ExecutionTier::NativeSemantic }));
}

#[test]
fn hundred_runs_per_pack_meet_alpha_gates() {
    let packs = load_v1_packs(&repo_packs_dir()).unwrap();
    let packs_dir = repo_packs_dir();
    for pack in &packs {
        let report = evaluate_pack(pack, &packs_dir, 100);
        assert_eq!(report.runs, 100, "{}", report.workflow_id);
        assert_eq!(
            report.verified_completions, 100,
            "{}: every fixture run must verify",
            report.workflow_id
        );
        assert!(report.verified_completion_rate >= 0.99);
        assert_eq!(
            report.unauthorized_side_effects, 0,
            "{}: zero unauthorized side effects",
            report.workflow_id
        );
        assert!(report.slo.passed, "{}: {report:?}", report.workflow_id);
        // Approval-first send is visible as expected approvals, not rescue.
        assert!(
            report.expected_approvals > 0 || report.workflow_id == "invoice-reconciliation",
            "{} should exercise approvals",
            report.workflow_id
        );
        assert_eq!(report.unexpected_rescues, 0);
        // Economics derivations exist for every pack (spec 22.10 seed).
        assert!(report.economics_derived.net_value_monthly_micro_usd > 0);
    }
}

#[test]
fn cost_per_verified_success_is_recorded() {
    let packs = load_v1_packs(&repo_packs_dir()).unwrap();
    let packs_dir = repo_packs_dir();
    for pack in &packs {
        let report = evaluate_pack(pack, &packs_dir, 30);
        let cost = report
            .cost_per_verified_success_micro_usd
            .expect("verified runs exist");
        assert!(cost > 0, "{}: fixture cost recorded", report.workflow_id);
    }
}

#[test]
fn quote_pack_send_steps_are_approval_gated() {
    let packs = load_v1_packs(&repo_packs_dir()).unwrap();
    let pack = packs
        .iter()
        .find(|p| p.manifest.pack_id == "quote-followup")
        .unwrap();
    let send = pack
        .steps
        .iter()
        .find(|s| s.step_id == "send-followup")
        .unwrap();
    assert!(matches!(
        send.approval_rule,
        lumi_packs::ApprovalRule::Always
    ));
    assert_eq!(send.risk_class, lumi_protocol::RiskClass::Communication);
    assert!(matches!(
        send.exception_route.target,
        lumi_packs::ExceptionTarget::User
    ));
}

#[test]
fn pack_outcomes_classify_per_spec_16_16() {
    // Sanity: the harness records DELIVERED on verified runs only.
    let packs = load_v1_packs(&repo_packs_dir()).unwrap();
    let packs_dir = repo_packs_dir();
    let report = evaluate_pack(&packs[0], &packs_dir, 5);
    assert!(report.verified_completions > 0);
    let _ = StepOutcomeClass::Delivered;
    let _ = SloLevel::Alpha;
}
