//! Policy posture for external/destructive git operations (spec 26
//! §26.34 items 20–21): push/force/reset-class actions are denied or
//! approval-gated by the default policy engine, and the `GitRepo`
//! capability wrapper cannot reach them at all.

use lumi_policy::{evaluate, CapabilityRegistry, DeviceExecutionState, PolicyContext};
use lumi_protocol::{
    ActionProposal, AuthenticationStrength, Capability, Principal, PrincipalId, PrincipalKind,
    ResourceRef, ResourceType, RiskClass, Target, TenantId, Timestamp,
};

fn principal() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-1").unwrap(),
        tenant_id: TenantId::parse("t-1").unwrap(),
        kind: PrincipalKind::User,
        authenticated_at: Some(Timestamp::from_epoch(0, 0).unwrap()),
        authentication_strength: Some(AuthenticationStrength::DevicePossession),
    }
}

fn git_action(capability: &str, operation: &str, risk: RiskClass) -> ActionProposal {
    ActionProposal::builder(
        lumi_protocol::ActionId::parse("action-git-1").unwrap(),
        lumi_protocol::TaskId::parse("task-1").unwrap(),
        lumi_protocol::RunId::parse("run-1").unwrap(),
        principal(),
        Capability(capability.to_owned()),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::FILE),
            id: "repo/printup".to_owned(),
            sensitivity: None,
        },
        Target::canonical("repo/printup"),
        operation,
        risk,
    )
    .unwrap()
}

fn context(registry: Option<&CapabilityRegistry>) -> PolicyContext<'_> {
    PolicyContext {
        now: Timestamp::from_epoch(0, 0).unwrap(),
        device_state: DeviceExecutionState::Trusted,
        registry,
    }
}

#[test]
fn unregistered_device_denies_everything_including_local_writes() {
    let registry = CapabilityRegistry::default();
    let ctx = PolicyContext {
        now: Timestamp::from_epoch(0, 0).unwrap(),
        device_state: DeviceExecutionState::Unregistered,
        registry: Some(&registry),
    };
    let decision = evaluate(
        &git_action("git.commit", "git_commit", RiskClass::LocalWrite),
        &[],
        &ctx,
    );
    assert!(
        matches!(decision, lumi_policy::Decision::Deny { .. }),
        "unregistered device must fail closed: {decision:?}"
    );
}

#[test]
fn external_push_is_not_allowed_without_explicit_authority() {
    let registry = CapabilityRegistry::default();
    let ctx = context(Some(&registry));
    for (capability, operation, risk) in [
        ("git.push", "git_push", RiskClass::ExternalWrite),
        ("git.force_push", "git_force_push", RiskClass::Destructive),
        ("git.reset_hard", "git_reset_hard", RiskClass::Destructive),
        ("git.clean", "git_clean", RiskClass::Destructive),
    ] {
        let decision = evaluate(&git_action(capability, operation, risk), &[], &ctx);
        assert!(
            matches!(decision, lumi_policy::Decision::Deny { .. }),
            "{capability} must be denied by default, got {decision:?}"
        );
    }
}

#[test]
fn destructive_git_action_is_approval_gated_when_pre_authorized() {
    // Even WITH an organization grant + narrow pre-authorization for a
    // destructive git capability, the DESTRUCTIVE risk class forces the
    // approval path — never a silent allow (§26.17).
    let grant = lumi_policy::CapabilityGrant {
        grant_id: "g-git-force".to_owned(),
        tenant_id: TenantId::parse("t-1").unwrap(),
        principal_id: None,
        capability: Capability("git.force_push".to_owned()),
        resource_scope: lumi_policy::ResourceScope::all_of([ResourceType::well_known(
            ResourceType::FILE,
        )]),
        target_prefixes: std::collections::BTreeSet::new(),
        expires_at: None,
        source: lumi_policy::GrantSource::OrganizationPolicy,
    };
    let registry = CapabilityRegistry::default().grant(grant);
    let ctx = context(Some(&registry));
    let pre_auth = lumi_policy::PreAuthorization {
        auth_id: "pre-git-force".to_owned(),
        capability: Capability("git.force_push".to_owned()),
        resource_types: vec![ResourceType::well_known(ResourceType::FILE)],
        target_prefixes: vec!["repo/printup".to_owned()],
        workflow_id: None,
        max_sensitivity: None,
        expires_at: Timestamp::from_epoch(9_999_999, 0).unwrap(),
    };
    let decision = evaluate(
        &git_action("git.force_push", "git_force_push", RiskClass::Destructive),
        &[pre_auth],
        &ctx,
    );
    match decision {
        lumi_policy::Decision::RequireApproval { .. } => {}
        other => panic!("destructive git action must require approval, got {other:?}"),
    }
}

#[test]
fn policy_registry_absent_fails_closed_for_git_actions() {
    let ctx = context(None);
    let decision = evaluate(
        &git_action("git.push", "git_push", RiskClass::ExternalWrite),
        &[],
        &ctx,
    );
    assert!(
        matches!(decision, lumi_policy::Decision::Deny { .. }),
        "no registry = no authority: {decision:?}"
    );
}
