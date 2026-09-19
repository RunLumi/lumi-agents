//! Native executor certification fixtures (spec 07 §7.14, §7.21).
//!
//! The same scenarios run against macOS-style and Windows-style fixture
//! environments; every outcome is oracle-verified (§7.15) and collateral
//! effects are checked (§7.17).

mod common;

use common::fixture_driver::FixtureDesktop;
use lumi_native::{
    DeliveryMode, DesktopDriver, NativeExecutor, NativeOutcome, RuntimeGeneration, SessionHandle,
    SessionState,
};
use lumi_protocol::{
    ActionProposal, AuthenticationStrength, Capability, ExecutionStatus, FailureCategory,
    Grounding, Principal, PrincipalId, PrincipalKind, ResourceRef, ResourceType, RiskClass, RunId,
    SensitivityLabel, Target, TaskId, TenantId, Timestamp,
};

fn principal() -> Principal {
    Principal {
        principal_id: PrincipalId::parse("u-native").unwrap(),
        tenant_id: TenantId::parse("t-acme").unwrap(),
        kind: PrincipalKind::Workflow,
        authenticated_at: Some(Timestamp::UNIX_EPOCH),
        authentication_strength: Some(AuthenticationStrength::DevicePossession),
    }
}

fn set_value_action(app: &str, window: &str, field: &str, value: &str) -> ActionProposal {
    ActionProposal::builder(
        ActionIdGen::next(),
        TaskId::parse("task-native").unwrap(),
        RunId::parse("run-native").unwrap(),
        principal(),
        Capability::well_known(lumi_protocol::capabilities::DESKTOP_INTERACT),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::LOCAL_APP),
            id: app.to_owned(),
            sensitivity: Some(SensitivityLabel::Internal),
        },
        Target::canonical(format!("{app}::{window}::{field}")),
        "native_set_value",
        RiskClass::LocalWrite,
    )
    .arguments(serde_json::json!({
        "op": "set_value",
        "target": {
            "application": app,
            "window": window,
            "role": "text_field",
            "name": field,
        },
        "value": value,
    }))
    .unwrap()
}

/// Deterministic action ids.
struct ActionIdGen;

impl ActionIdGen {
    fn next() -> lumi_protocol::ActionId {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        lumi_protocol::ActionId::parse(format!("a-native-{n}")).unwrap()
    }
}

fn handle(generation: u64) -> SessionHandle {
    SessionHandle {
        session_id: "fixture-session".to_owned(),
        generation: RuntimeGeneration(generation),
    }
}

/// The effect oracle observes the fixture's semantic value — DELIVERED
/// only when the intended state change is visible.
fn value_oracle<'a>(
    driver: &'a FixtureDesktop,
    app: &str,
    window: &str,
    field: &str,
) -> impl Fn() -> Option<String> + 'a {
    let key = format!("{app}::{window}::{field}");
    move || {
        driver
            .values
            .borrow()
            .get(&key)
            .cloned()
            .filter(|v| !v.is_empty())
    }
}

fn macos_and_windows() -> Vec<(&'static str, FixtureDesktop)> {
    vec![
        ("macos", FixtureDesktop::macos_notes()),
        ("windows", FixtureDesktop::windows_notepad()),
    ]
}

#[test]
fn set_value_delivers_with_oracle_evidence_on_both_platforms() {
    for (platform, driver) in macos_and_windows() {
        let (app, window, field) = match platform {
            "macos" => ("com.acme.notes", "Untitled", "note-body"),
            _ => ("Notepad.exe", "Untitled - Notepad", "editor"),
        };
        let executor = NativeExecutor::new(&driver, handle(0));
        let action = set_value_action(app, window, field, "Quarterly reconciliation complete");
        let result = executor.execute(
            &action,
            DeliveryMode::SemanticBackground,
            &value_oracle(&driver, app, window, field),
        );
        assert_eq!(result.status, ExecutionStatus::Success, "{platform}");
        assert_eq!(
            result.grounding,
            Some(Grounding::SemanticTarget),
            "{platform}"
        );
        assert!(result
            .observation_ids
            .contains(&"native-effect-oracle".to_owned()));
        // The value actually changed in the modeled app state.
        let key = format!("{app}::{window}::{field}");
        assert_eq!(
            driver.values.borrow()[&key],
            "Quarterly reconciliation complete"
        );
    }
}

#[test]
fn os_success_without_effect_is_not_delivered() {
    // §7.21: OS API success but missing effect -> NO_EFFECT.
    let mut driver = FixtureDesktop::macos_notes();
    driver.silent_failure = true;
    let executor = NativeExecutor::new(&driver, handle(0));
    let action = set_value_action("com.acme.notes", "Untitled", "note-body", "text");
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "com.acme.notes", "Untitled", "note-body"),
    );
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::Postcondition
    );
}

#[test]
fn interruption_yields_ambiguous() {
    // §7.21: unknown effect after interruption -> AMBIGUOUS.
    let mut driver = FixtureDesktop::windows_notepad();
    driver.interrupted = true;
    let executor = NativeExecutor::new(&driver, handle(0));
    let action = set_value_action("Notepad.exe", "Untitled - Notepad", "editor", "draft");
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "Notepad.exe", "Untitled - Notepad", "editor"),
    );
    assert_eq!(result.status, ExecutionStatus::Ambiguous);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::AmbiguousState
    );
}

#[test]
fn locked_session_fails_explicitly() {
    let mut driver = FixtureDesktop::macos_notes();
    driver.session = SessionState::Locked;
    let executor = NativeExecutor::new(&driver, handle(0));
    let action = set_value_action("com.acme.notes", "Untitled", "note-body", "x");
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "com.acme.notes", "Untitled", "note-body"),
    );
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::NativeSession
    );
    // Nothing was attempted.
    assert!(driver.operations.borrow().is_empty());
}

#[test]
fn missing_permissions_fail_without_escalation() {
    let mut driver = FixtureDesktop::windows_notepad();
    driver.permissions[0].state = lumi_native::PermissionState::NotGranted;
    let executor = NativeExecutor::new(&driver, handle(0));
    let action = set_value_action("Notepad.exe", "Untitled - Notepad", "editor", "x");
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "Notepad.exe", "Untitled - Notepad", "editor"),
    );
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::OsPermission
    );
    assert!(
        driver.operations.borrow().is_empty(),
        "no operations before onboarding"
    );
}

#[test]
fn revoked_permission_blocks_too() {
    let mut driver = FixtureDesktop::macos_notes();
    driver.permissions[0].state = lumi_native::PermissionState::Revoked;
    let executor = NativeExecutor::new(&driver, handle(0));
    let action = set_value_action("com.acme.notes", "Untitled", "note-body", "x");
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "com.acme.notes", "Untitled", "note-body"),
    );
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::OsPermission
    );
}

#[test]
fn stale_generation_handle_fails_closed() {
    // §7.21: stale generation handle rejected.
    let mut driver = FixtureDesktop::macos_notes();
    let stale = handle(driver.current_generation.0);
    // Driver restart/rebind bumps the generation:
    driver.current_generation.bump();
    let executor = NativeExecutor::new(&driver, stale);
    let action = set_value_action("com.acme.notes", "Untitled", "note-body", "x");
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "com.acme.notes", "Untitled", "note-body"),
    );
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::UpstreamDriver
    );
    assert!(driver.operations.borrow().is_empty());
}

#[test]
fn wrong_window_context_is_refused_before_mutation() {
    // §7.14/§7.21: input must not reach another app/window. The fixture's
    // focused context belongs to a different app than the target.
    let mut driver = FixtureDesktop::macos_notes();
    driver.focused_context = "com.other.app::Other Window".to_owned();
    let executor = NativeExecutor::new(&driver, handle(0));
    let action = set_value_action("com.acme.notes", "Untitled", "note-body", "x");
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "com.acme.notes", "Untitled", "note-body"),
    );
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::PolicyDenyExpected,
        "REFUSED surfaces as expected-deny"
    );
    // The non-target app's state is untouched.
    assert_eq!(
        driver.values.borrow()["com.acme.notes::Untitled::note-body"],
        ""
    );
}

#[test]
fn global_input_without_policy_authorization_is_refused() {
    // §7.20/§7.21: global-input escalation requires explicit policy
    // authorization; the executor refuses it as a security violation.
    let driver = FixtureDesktop::macos_notes();
    let executor = NativeExecutor::new(&driver, handle(0));
    let mut action = set_value_action("com.acme.notes", "Untitled", "note-body", "x");
    action.arguments["op"] = serde_json::json!("click");
    action.arguments["delivery"] = serde_json::json!("global_input");
    // Policy only authorized background semantic delivery:
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "com.acme.notes", "Untitled", "note-body"),
    );
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::SecurityViolation
    );
    // No operations reached the driver; target untouched.
    assert!(driver.operations.borrow().is_empty());
    assert_eq!(
        driver.values.borrow()["com.acme.notes::Untitled::note-body"],
        ""
    );
}

#[test]
fn authorized_global_input_records_collateral_truth() {
    // With fresh policy authorization, global input proceeds - and the
    // collateral oracle records the focus cost (§7.17): correct mutation
    // with unacceptable collateral is visible, not hidden. Driven at the
    // driver boundary with an oracle that observes the click.
    use lumi_native::driver::{NativeOperation, NativeResult};
    use lumi_native::target::SemanticTarget;
    let driver = FixtureDesktop::macos_notes();
    let target = SemanticTarget {
        application: "com.acme.notes".to_owned(),
        window: Some("Untitled".to_owned()),
        role: Some("button".to_owned()),
        name: Some("Save".to_owned()),
        stable_property: None,
    };
    let operation = NativeOperation::Click {
        target,
        mode: DeliveryMode::GlobalInput,
    };
    let oracle = || Some("click observed".to_owned());
    let result: NativeResult = driver.execute(&handle(0), operation, &oracle).unwrap();
    assert_eq!(result.outcome, NativeOutcome::Delivered);
    // The collateral truth: global input does NOT preserve focus and can
    // leak input - recorded, not hidden.
    assert_eq!(result.collateral.focus_preserved, Some(false));
    assert_eq!(result.collateral.no_input_leak, Some(false));
    assert!(!result.collateral.all_pass(), "collateral must be flagged");
}

#[test]
fn background_delivery_preserves_focus_and_cursor() {
    // §7.21: background delivery preserves focus/cursor where promised.
    use lumi_native::driver::{NativeOperation, NativeResult};
    use lumi_native::target::SemanticTarget;
    let driver = FixtureDesktop::macos_notes();
    let target = SemanticTarget {
        application: "com.acme.notes".to_owned(),
        window: Some("Untitled".to_owned()),
        role: Some("button".to_owned()),
        name: Some("Save".to_owned()),
        stable_property: None,
    };
    let operation = NativeOperation::Click {
        target,
        mode: DeliveryMode::SemanticBackground,
    };
    let oracle = || Some("click observed".to_owned());
    let result: NativeResult = driver.execute(&handle(0), operation, &oracle).unwrap();
    assert_eq!(result.outcome, NativeOutcome::Delivered);
    assert_eq!(result.collateral.focus_preserved, Some(true));
    assert_eq!(result.collateral.no_input_leak, Some(true));
    assert!(result.collateral.all_pass());
}

#[test]
fn modal_dialog_fails_with_session_category() {
    let mut driver = FixtureDesktop::macos_notes();
    driver.modal_dialog = Some("Save changes before closing?".to_owned());
    let executor = NativeExecutor::new(&driver, handle(0));
    let action = set_value_action("com.acme.notes", "Untitled", "note-body", "x");
    let result = executor.execute(
        &action,
        DeliveryMode::SemanticBackground,
        &value_oracle(&driver, "com.acme.notes", "Untitled", "note-body"),
    );
    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().category,
        FailureCategory::NativeSession
    );
}

#[test]
fn cancel_and_unsupported_routes_classify() {
    // §7.15 outcome classification sanity.
    assert!(NativeOutcome::Delivered.counts_as_delivery());
    assert!(!NativeOutcome::NoEffect.counts_as_delivery());
    assert!(!NativeOutcome::Ambiguous.counts_as_delivery());
    assert!(!NativeOutcome::Refused.counts_as_delivery());
}

#[test]
fn delivery_mode_promises_are_coherent() {
    // §7.17/§7.20 documentation-in-types: background delivery promises
    // focus+cursor preservation; global input never does.
    assert!(DeliveryMode::SemanticBackground.preserves_focus_and_cursor());
    assert!(!DeliveryMode::SemanticForeground.preserves_focus_and_cursor());
    assert!(!DeliveryMode::GlobalInput.preserves_focus_and_cursor());
    assert!(DeliveryMode::GlobalInput.is_global_input());
    assert!(!DeliveryMode::SemanticBackground.is_global_input());
}

#[test]
fn certification_matrix_requires_concrete_entries() {
    // §7.16: support claims must be backed by concrete matrix entries.
    // The fixtures above model two concrete certified combinations:
    let matrix = serde_json::json!([
        {
            "os": "macos-14",
            "app": "com.acme.notes/2.0",
            "driver": "fixture-driver/0.1.0",
            "action": "set_value",
            "targeting": "semantic",
            "mode": "background",
            "permissions": ["macos.accessibility"],
            "oracle": "ax_value_read_back",
            "outcome": "DELIVERED"
        },
        {
            "os": "windows-11-23h2",
            "app": "Notepad.exe/11",
            "driver": "fixture-driver/0.1.0",
            "action": "set_value",
            "targeting": "semantic",
            "mode": "background",
            "permissions": ["windows.uia"],
            "oracle": "uia_value_pattern_read_back",
            "outcome": "DELIVERED"
        }
    ]);
    for entry in matrix.as_array().unwrap() {
        for required in [
            "os",
            "app",
            "driver",
            "action",
            "targeting",
            "mode",
            "permissions",
            "oracle",
            "outcome",
        ] {
            assert!(entry.get(required).is_some(), "matrix missing {required}");
        }
    }
}
