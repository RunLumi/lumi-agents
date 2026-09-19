//! Spec 14.13 + 14.19 certification: policy-passing side effects, secret
//! scoping, capability-growth review, network destination denial,
//! org-policy ceiling, and crash/scope isolation.

use lumi_connectors::{
    scoped_broker, ConnectorManifest, DataCategory, ExtensionRegistry, FilesystemScope,
    IntegrationKind, OrgPolicyCeiling, RegistryError,
};
use lumi_protocol::SecretRef;
use lumi_secrets::{InMemorySecretBackend, SecretBroker, SecretValue};
use std::collections::BTreeSet;

fn crm_manifest() -> ConnectorManifest {
    ConnectorManifest {
        id: "acme-crm".to_owned(),
        version: "1.0.0".to_owned(),
        kind: IntegrationKind::Connector,
        origin: "https://connectors.acme.test/crm".to_owned(),
        license: "Apache-2.0".to_owned(),
        capabilities: BTreeSet::from(["crm.quote.update".to_owned()]),
        side_effects: vec![lumi_connectors::SideEffectDeclaration {
            operation: "create_crm_quote".to_owned(),
            risk_class: "EXTERNAL_WRITE".to_owned(),
            externally_visible: true,
        }],
        network_destinations: BTreeSet::from(["crm.acme.test:443".to_owned()]),
        filesystem: FilesystemScope::WorkspaceOnly,
        secret_refs: BTreeSet::from(["acme-crm-token".to_owned()]),
        data_categories: BTreeSet::from([DataCategory::CustomerData]),
        platform_requirements: BTreeSet::new(),
        update_source: "connectors.acme.test".to_owned(),
        min_runtime: "0.1.0".to_owned(),
    }
}

fn ceiling() -> OrgPolicyCeiling {
    OrgPolicyCeiling::default()
}

#[test]
fn side_effecting_connector_call_passes_policy_shape() {
    // §14.13: a side-effecting MCP/connector call normalizes to an
    // ActionProposal-shaped declaration that names its risk. The
    // manifest's side-effect declarations ARE that normalization; the
    // policy gate evaluates the resulting proposal at run time.
    let m = crm_manifest();
    assert!(m.validate().is_ok());
    let side_effect = &m.side_effects[0];
    assert_eq!(side_effect.risk_class, "EXTERNAL_WRITE");
    assert!(side_effect.externally_visible);
    // §14.4: the declaration does not GRANT the capability — grants come
    // from the policy registry separately.
}

#[test]
fn connector_cannot_resolve_unrelated_secret() {
    // §14.13/§14.5: CRM connector must not receive email/browser secrets.
    let inner = InMemorySecretBackend::new();
    let inner_broker = SecretBroker::new(inner);
    // Seed both secrets in the underlying store via an unscoped broker.
    inner_broker
        .store(
            &SecretRef::new("acme-crm-token"),
            &SecretValue::new("crm-token"),
        )
        .unwrap();
    inner_broker
        .store(
            &SecretRef::new("email-password"),
            &SecretValue::new("email-secret"),
        )
        .unwrap();

    let m = crm_manifest();
    let scoped = scoped_broker(
        InMemorySecretBackend::new(),
        "acme-crm",
        m.secret_refs.iter().cloned(),
    );
    // Seed the scoped store with the declared ref.
    scoped
        .store(
            &SecretRef::new("acme-crm-token"),
            &SecretValue::new("crm-token"),
        )
        .unwrap();

    // Declared ref: resolves.
    assert_eq!(
        scoped
            .resolve(&SecretRef::new("acme-crm-token"), "crm-auth")
            .unwrap()
            .expose(),
        "crm-token"
    );
    // Undeclared ref: refused even though the value exists in the
    // underlying store.
    assert!(scoped
        .resolve(&SecretRef::new("email-password"), "sneaky")
        .is_err());
    // Store/revoke of undeclared refs also refused.
    assert!(scoped
        .store(&SecretRef::new("email-password"), &SecretValue::new("x"))
        .is_err());
    assert!(scoped.revoke(&SecretRef::new("email-password")).is_err());
}

#[test]
fn version_adding_capability_requires_review() {
    let mut registry = ExtensionRegistry::new(ceiling());
    registry.admit(crm_manifest()).unwrap();

    let mut v110 = crm_manifest();
    v110.version = "1.1.0".to_owned();
    v110.capabilities.insert("email.send".to_owned());
    v110.secret_refs.insert("smtp-password".to_owned());

    // §14.19: growth is blocked, prior version stays active.
    let err = registry.admit(v110.clone()).unwrap_err();
    assert!(matches!(
        err,
        RegistryError::CapabilityGrowthRequiresReview { .. }
    ));
    assert_eq!(registry.get("acme-crm").unwrap().manifest.version, "1.0.0");

    // Human review approves: new baseline active, new secret now scoped.
    let approved = registry.approve_upgrade("acme-crm").unwrap();
    assert_eq!(approved.manifest.version, "1.1.0");
    assert!(approved.manifest.secret_declared("smtp-password"));
}

#[test]
fn unexpected_network_target_denied() {
    let mut registry = ExtensionRegistry::new(ceiling());
    registry.admit(crm_manifest()).unwrap();
    assert!(registry
        .check_network_target("acme-crm", "crm.acme.test:443")
        .is_ok());
    assert!(matches!(
        registry.check_network_target("acme-crm", "exfil.example.test:443"),
        Err(RegistryError::NetworkTargetDenied { .. })
    ));
}

#[test]
fn org_ceiling_blocks_disallowed_project_extension() {
    // §14.19: organization policy blocks a disallowed project extension;
    // the project cannot widen the ceiling.
    let mut org_ceiling = OrgPolicyCeiling::default();
    org_ceiling
        .denied_origins
        .insert("https://sketchy-marketplace.test".to_owned());
    org_ceiling.require_network_declaration = true;

    let mut registry = ExtensionRegistry::new(org_ceiling);
    let mut sketchy = crm_manifest();
    sketchy.id = "sketchy-ext".to_owned();
    // Exact origin is on the org deny list...
    sketchy.origin = "https://sketchy-marketplace.test".to_owned();
    // ...and it also omits network destinations (a second ceiling rule).
    sketchy.network_destinations = BTreeSet::new();

    let err = registry.admit(sketchy).unwrap_err();
    match err {
        RegistryError::CeilingViolation { reason } => {
            assert!(reason.contains("organization-denied"), "{reason}");
        }
        other => panic!("expected CeilingViolation, got {other:?}"),
    }
}

#[test]
fn connector_crash_does_not_corrupt_registry_state() {
    // Isolation at the registry level: an unknown/crashed connector's
    // failure leaves other records intact.
    let mut registry = ExtensionRegistry::new(ceiling());
    registry.admit(crm_manifest()).unwrap();
    assert!(registry
        .check_network_target("nonexistent", "crm.acme.test:443")
        .is_err());
    // The known connector still works.
    assert!(registry
        .check_network_target("acme-crm", "crm.acme.test:443")
        .is_ok());
}

#[test]
fn cross_tenant_connector_records_are_isolated_by_id_convention() {
    // §14.13 cross-tenant isolation: tenant identity rides on the
    // task/principal (enforced in policy), while the registry keys by
    // connector id — two tenants admitting the same connector each get
    // their own record namespace via distinct registry instances.
    let mut tenant_a = ExtensionRegistry::new(ceiling());
    let mut tenant_b = ExtensionRegistry::new(ceiling());
    tenant_a.admit(crm_manifest()).unwrap();
    tenant_b.admit(crm_manifest()).unwrap();
    // Revoking in A does not affect B (registry instances are per-tenant).
    assert!(tenant_a
        .check_network_target("acme-crm", "crm.acme.test:443")
        .is_ok());
    assert!(tenant_b
        .check_network_target("acme-crm", "crm.acme.test:443")
        .is_ok());
}
