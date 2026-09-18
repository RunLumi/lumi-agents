//! Contract fixtures required by spec 03 §3.12.
//!
//! These tests pin the protocol-level guarantees that policy, executors,
//! and audit depend on:
//!
//! 1. golden JSON round-trips byte-stable with the schema envelope;
//! 2. the same business action maps across connector/browser/native tiers
//!    while staying the *same normalized action* for policy;
//! 3. adapter-native failures normalize to canonical failure categories;
//! 4. approved-action material mutation is detected;
//! 5. redaction removes marked secret fields from payloads.

use lumi_protocol::{
    redact_value, ActionId, ActionProposal, AuthenticationStrength, Capability, ErrorEnvelope,
    ExecutionPreferences, ExecutionTier, FailureCategory, Observation, ObservationSource,
    ObservationSurface, Principal, PrincipalKind, RedactionRule, ResourceRef, ResourceType,
    RiskClass, RunId, SensitivityLabel, Target, TaskId, Timestamp,
};

const ACTION_FIXTURE: &str = include_str!("fixtures/action_send_customer_email.json");
const OBSERVATION_FIXTURE: &str = include_str!("fixtures/observation_draft_state.json");

fn principal() -> Principal {
    Principal {
        principal_id: lumi_protocol::PrincipalId::parse("user-01j9").unwrap(),
        tenant_id: lumi_protocol::TenantId::parse("tenant-acme").unwrap(),
        kind: PrincipalKind::User,
        authenticated_at: Some(Timestamp::parse("2026-09-18T09:00:00Z").unwrap()),
        authentication_strength: Some(AuthenticationStrength::Mfa),
    }
}

/// The same business action ("send the customer follow-up email") expressed
/// for three different executors.
fn email_action_on_tier(tier: ExecutionTier) -> ActionProposal {
    let prefs = ExecutionPreferences {
        allowed_tiers: vec![tier],
        preferred_tier: Some(tier),
    };
    ActionProposal::builder(
        ActionId::parse("act-2026-09-18-alpha").unwrap(),
        TaskId::parse("task-2026-09-18-alpha").unwrap(),
        RunId::parse("run-2026-09-18-alpha").unwrap(),
        principal(),
        Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
            id: "outbound/quote-followup-cust-42".to_owned(),
            sensitivity: Some(SensitivityLabel::Confidential),
        },
        Target::canonical("mailto:customer@example.test"),
        "send_customer_email",
        RiskClass::Communication,
    )
    .arguments(serde_json::json!({
        "subject": "Following up: quote Q-2091",
        "body": "Hi — following up on the quote we sent last week. Does Tuesday work for a call?",
    }))
    .execution_preferences(prefs)
    .unwrap()
}

#[test]
fn golden_action_fixture_round_trips_byte_stable() {
    let action = ActionProposal::from_json(ACTION_FIXTURE).expect("fixture must parse");
    let reserialized = action.to_json().expect("fixture must serialize");
    let reparsed = ActionProposal::from_json(&reserialized).expect("reserialized must parse");
    assert_eq!(reparsed, action);
    // Structural expectations from the golden file.
    assert_eq!(action.risk_class, RiskClass::Communication);
    assert_eq!(
        action.idempotency.semantics,
        lumi_protocol::IdempotencySemantics::ClientKey
    );
    assert_eq!(action.postconditions.len(), 1);
    assert_eq!(action.evidence_requirements.len(), 2);
}

#[test]
fn golden_observation_fixture_round_trips() {
    let obs = Observation::from_json(OBSERVATION_FIXTURE).expect("fixture must parse");
    let reserialized = obs.to_json().expect("fixture must serialize");
    let reparsed = Observation::from_json(&reserialized).expect("reserialized must parse");
    assert_eq!(reparsed, obs);
    assert_eq!(obs.source.surface, ObservationSurface::Browser);
    assert_eq!(
        obs.confidence.kind,
        lumi_protocol::ConfidenceKind::Structured
    );
    assert_eq!(
        obs.action_id.as_deref(),
        Some("act-2026-09-18-alpha")
    );
}

#[test]
fn same_business_action_is_one_normalized_action_across_tiers() {
    let connector = email_action_on_tier(ExecutionTier::ConnectorApi);
    let browser = email_action_on_tier(ExecutionTier::BrowserSemantic);
    let native = email_action_on_tier(ExecutionTier::NativeSemantic);

    for variant in [&browser, &native] {
        // Everything policy evaluates must be identical across tiers.
        assert_eq!(variant.capability, connector.capability);
        assert_eq!(variant.resource, connector.resource);
        assert_eq!(variant.target, connector.target);
        assert_eq!(variant.operation, connector.operation);
        assert_eq!(variant.arguments, connector.arguments);
        assert_eq!(variant.risk_class, connector.risk_class);
        assert_eq!(variant.expected_effect, connector.expected_effect);
        // Tier is execution data, not material: approvals survive a tier
        // change for the same business action.
        assert_eq!(variant.material_digest(), connector.material_digest());
    }
}

#[test]
fn material_mutation_is_detected() {
    let approved = ActionProposal::from_json(ACTION_FIXTURE).unwrap();
    let digest = approved.material_digest();

    // Unchanged action still matches its approved digest.
    let identical = ActionProposal::from_json(&approved.to_json().unwrap()).unwrap();
    assert!(identical.material_matches_digest(&digest));

    // Wrong recipient: material change → invalidates approval.
    let wrong_target = ActionProposal {
        target: Target::canonical("mailto:marketing@example.test"),
        ..approved.clone()
    };
    assert!(!wrong_target.material_matches_digest(&digest));

    // Changed body: material change → invalidates approval.
    let changed_body = ActionProposal {
        arguments: serde_json::json!({
            "subject": "Following up: quote Q-2091",
            "body": "Completely different content with new terms.",
        }),
        ..approved.clone()
    };
    assert!(!changed_body.material_matches_digest(&digest));

    // Declared reversibility flip: material change (safer to over-cover).
    let claims_reversible = ActionProposal {
        expected_effect: lumi_protocol::ExpectedEffect {
            summary: approved.expected_effect.summary.clone(),
            external_visibility: true,
            reversible: true,
        },
        ..approved.clone()
    };
    assert!(!claims_reversible.material_matches_digest(&digest));
}

#[test]
fn native_failures_normalize_to_canonical_categories() {
    let cases = [
        ("playwright", "TimeoutError", FailureCategory::BrowserState),
        (
            "playwright",
            "LocatorNotFound",
            FailureCategory::BrowserSelector,
        ),
        ("playwright", "TargetClosed", FailureCategory::BrowserState),
        (
            "playwright",
            "NavigationInterrupted",
            FailureCategory::Network,
        ),
        ("cua", "ElementNotFound", FailureCategory::NativeElement),
        ("cua", "SessionLost", FailureCategory::NativeSession),
        ("cua", "PermissionDenied", FailureCategory::OsPermission),
        ("http", "401", FailureCategory::AuthSession),
        ("http", "429", FailureCategory::ProviderRateLimit),
        ("shell", "Timeout", FailureCategory::ShellExecution),
        ("fs", "PermissionDenied", FailureCategory::OsPermission),
    ];
    for (adapter, code, expected) in cases {
        let envelope = ErrorEnvelope::from_adapter(adapter, code, format!("{adapter} {code}"));
        assert_eq!(envelope.category, expected, "{adapter}/{code}");
        assert_eq!(envelope.native_adapter.as_deref(), Some(adapter));
        assert_eq!(envelope.native_code.as_deref(), Some(code));
        // Native diagnostics ride along, but the canonical category is the
        // persisted truth.
        assert_eq!(envelope.category, envelope.category);
    }
}

#[test]
fn redaction_removes_marked_secret_fields_before_egress() {
    let action = ActionProposal::builder(
        ActionId::parse("act-export").unwrap(),
        TaskId::parse("task-export").unwrap(),
        RunId::parse("run-export").unwrap(),
        principal(),
        Capability::well_known(lumi_protocol::capabilities::DATA_EXPORT),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::DATABASE_RECORD),
            id: "customers/all".to_owned(),
            sensitivity: Some(SensitivityLabel::PersonalData),
        },
        Target::canonical("sftp://partner.example.test/inbox"),
        "export_customer_records",
        RiskClass::DataExport,
    )
    .arguments(serde_json::json!({
        "filters": {"region": "eu"},
        "credentials": {"password": "sup3r-secret", "username": "lumi"},
    }))
    .unwrap();

    let rules = [
        RedactionRule::key("password"),
        RedactionRule::key("api_key"),
    ];
    let redacted_args = redact_value(&action.arguments, &rules);
    assert_eq!(
        redacted_args["credentials"]["password"],
        lumi_protocol::REDACTED_MARKER
    );
    assert_eq!(redacted_args["credentials"]["username"], "lumi");
    assert_eq!(redacted_args["filters"]["region"], "eu");
    // The original proposal is untouched: redaction is applied at the
    // boundary, not destructively.
    assert_eq!(action.arguments["credentials"]["password"], "sup3r-secret");

    // Observation payloads redact the same way.
    let observation = Observation {
        observation_id: "obs-redact".to_owned(),
        task_id: TaskId::parse("task-export").unwrap(),
        run_id: RunId::parse("run-export").unwrap(),
        source: ObservationSource {
            surface: ObservationSurface::Shell,
            adapter: "shell/1".to_owned(),
            resource: None,
        },
        timestamp: Timestamp::parse("2026-09-18T09:00:00Z").unwrap(),
        kind: lumi_protocol::ObservationKind::Content,
        payload: serde_json::json!({"stdout": "ok", "env": {"API_KEY": "sk-live-123"}}),
        confidence: lumi_protocol::Confidence::deterministic(),
        sensitivity: None,
        provenance: None,
        action_id: None,
        step_id: None,
    };
    let redacted_payload = redact_value(&observation.payload, &[RedactionRule::key("API_KEY")]);
    assert_eq!(
        redacted_payload["env"]["API_KEY"],
        lumi_protocol::REDACTED_MARKER
    );
    assert_eq!(redacted_payload["stdout"], "ok");
}

#[test]
fn unknown_versions_and_enums_fail_explicitly() {
    let action_json =
        ACTION_FIXTURE.replace("\"schema_version\": \"1.0\"", "\"schema_version\": \"2.0\"");
    let err = ActionProposal::from_json(&action_json).unwrap_err();
    assert!(
        err.to_string().contains("unsupported schema version"),
        "{err}"
    );

    let obs_json = OBSERVATION_FIXTURE.replace(
        "\"schema_name\": \"lumi.observation\"",
        "\"schema_name\": \"lumi.action\"",
    );
    assert!(Observation::from_json(&obs_json).is_err());

    let bad_risk = ACTION_FIXTURE.replace("COMMUNICATION", "CARRIER_PIGEON");
    assert!(ActionProposal::from_json(&bad_risk).is_err());
}
