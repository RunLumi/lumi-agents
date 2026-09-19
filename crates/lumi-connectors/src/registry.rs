//! Extension registry: admission, capability-growth review, network
//! destination enforcement, and the organization policy ceiling
//! (spec 14 §14.9, §14.12, §14.16, §14.19).

use crate::manifest::ConnectorManifest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use lumi_protocol::Timestamp;

/// Review state for one pinned connector version (§14.9: a version that
/// ADDS capabilities requires explicit re-review before admission).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    /// Reviewed and admitted for this exact version.
    Approved,
    /// A new version widened the capability surface: blocked pending
    /// human review (§14.19 test).
    RequiresReview,
    /// Rejected by policy or review.
    Rejected,
}

/// The organization policy CEILING (§14.16). Project/user extensions
/// cannot widen anything beyond these bounds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OrgPolicyCeiling {
    /// Denied origins (exact match). Anything listed is refused
    /// regardless of project-level configuration.
    pub denied_origins: BTreeSet<String>,
    /// Denied licenses (exact match, e.g. `AGPL-3.0`).
    pub denied_licenses: BTreeSet<String>,
    /// When set, only these origins may be admitted at all.
    pub allowed_origins: Option<BTreeSet<String>>,
    /// Require declared network destinations to be non-empty.
    pub require_network_declaration: bool,
}

impl OrgPolicyCeiling {
    /// Checks one manifest against the ceiling.
    #[must_use]
    pub fn violates(&self, manifest: &ConnectorManifest) -> Option<String> {
        if self.denied_origins.contains(&manifest.origin) {
            return Some(format!(
                "origin {:?} is organization-denied",
                manifest.origin
            ));
        }
        if self.denied_licenses.contains(&manifest.license) {
            return Some(format!(
                "license {:?} is organization-denied",
                manifest.license
            ));
        }
        if let Some(allowed) = &self.allowed_origins {
            if !allowed.contains(&manifest.origin) {
                return Some(format!(
                    "origin {:?} is not on the organization allowlist",
                    manifest.origin
                ));
            }
        }
        if self.require_network_declaration && manifest.network_destinations.is_empty() {
            return Some("network destinations must be declared".to_owned());
        }
        None
    }
}

/// One admitted connector record: the pinned manifest + review state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorRecord {
    pub manifest: ConnectorManifest,
    pub review: ReviewState,
    pub reviewed_at: Option<Timestamp>,
}

/// Registry failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    InvalidManifest(Vec<String>),
    CeilingViolation {
        reason: String,
    },
    /// The new version added capabilities without review (§14.19).
    CapabilityGrowthRequiresReview {
        added: Vec<String>,
    },
    UnknownConnector {
        id: String,
    },
    /// Destination not declared (§14.8).
    NetworkTargetDenied {
        host: String,
    },
}

/// The extension registry.
#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    records: BTreeMap<String, ConnectorRecord>,
    /// The previously-admitted manifest per connector id, kept so
    /// version upgrades can diff capability surfaces (§14.9).
    prior_versions: BTreeMap<String, ConnectorManifest>,
    /// Manifests blocked pending human capability-growth review.
    pending_upgrades: BTreeMap<String, ConnectorManifest>,
    ceiling: OrgPolicyCeiling,
}

impl ExtensionRegistry {
    #[must_use]
    pub fn new(ceiling: OrgPolicyCeiling) -> Self {
        Self {
            records: BTreeMap::new(),
            prior_versions: BTreeMap::new(),
            pending_upgrades: BTreeMap::new(),
            ceiling,
        }
    }

    /// Admits a manifest: validates structure, applies the org ceiling,
    /// detects capability growth against the prior pinned version, and
    /// stores the record with the resulting review state.
    ///
    /// Capability growth ⇒ `RequiresReview` and the NEW manifest is not
    /// activated (the prior version stays active) until a human approves.
    ///
    /// # Errors
    /// [`RegistryError`].
    pub fn admit(&mut self, manifest: ConnectorManifest) -> Result<ConnectorRecord, RegistryError> {
        manifest
            .validate()
            .map_err(RegistryError::InvalidManifest)?;
        if let Some(reason) = self.ceiling.violates(&manifest) {
            return Err(RegistryError::CeilingViolation { reason });
        }

        let prior = self.prior_versions.get(&manifest.id);
        let review = match prior {
            None => ReviewState::Approved,
            Some(prior_manifest) => {
                let added: Vec<String> = manifest
                    .capability_surface()
                    .difference(prior_manifest.capability_surface())
                    .cloned()
                    .collect();
                if added.is_empty() {
                    ReviewState::Approved
                } else {
                    // §14.19: a version adding capabilities requires
                    // review. Park it; keep the prior version active.
                    self.park_pending_upgrade(manifest.clone());
                    return Err(RegistryError::CapabilityGrowthRequiresReview { added });
                }
            }
        };

        let record = ConnectorRecord {
            manifest: manifest.clone(),
            review,
            reviewed_at: Some(Timestamp::now()),
        };
        self.prior_versions
            .insert(manifest.id.clone(), manifest.clone());
        self.records.insert(manifest.id.clone(), record.clone());
        Ok(record)
    }

    /// Parks a new-version manifest that requires review (called
    /// internally by `admit` when growth is detected).
    fn park_pending_upgrade(&mut self, manifest: ConnectorManifest) {
        self.pending_upgrades.insert(manifest.id.clone(), manifest);
    }

    /// Human review approves a pending upgrade (§14.9): the new manifest
    /// becomes the active record and the new capability baseline.
    pub fn approve_upgrade(&mut self, id: &str) -> Result<ConnectorRecord, RegistryError> {
        let Some(manifest) = self.pending_upgrades.remove(id) else {
            return Err(RegistryError::UnknownConnector { id: id.to_owned() });
        };
        let record = ConnectorRecord {
            review: ReviewState::Approved,
            reviewed_at: Some(Timestamp::now()),
            manifest: manifest.clone(),
        };
        self.prior_versions.insert(id.to_owned(), manifest);
        self.records.insert(id.to_owned(), record.clone());
        Ok(record)
    }

    /// Human review rejects a pending upgrade: the prior version stays
    /// active, the pending manifest is dropped.
    pub fn reject_upgrade(&mut self, id: &str) -> bool {
        self.pending_upgrades.remove(id).is_some()
    }

    /// Looks up an admitted connector.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ConnectorRecord> {
        self.records.get(id)
    }

    /// Enforces the declared network destinations at call time (§14.8).
    ///
    /// # Errors
    /// [`RegistryError::NetworkTargetDenied`] for undeclared hosts, or
    /// unknown connectors.
    pub fn check_network_target(&self, id: &str, host: &str) -> Result<(), RegistryError> {
        let record = self
            .records
            .get(id)
            .ok_or_else(|| RegistryError::UnknownConnector { id: id.to_owned() })?;
        if record.manifest.network_destination_declared(host) {
            Ok(())
        } else {
            Err(RegistryError::NetworkTargetDenied {
                host: host.to_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_v(id_version: &str, caps: &[&str]) -> ConnectorManifest {
        ConnectorManifest {
            id: "acme-crm".to_owned(),
            version: id_version.to_owned(),
            kind: crate::manifest::IntegrationKind::Connector,
            origin: "https://connectors.acme.test/crm".to_owned(),
            license: "Apache-2.0".to_owned(),
            capabilities: caps.iter().map(|s| (*s).to_owned()).collect(),
            side_effects: vec![],
            network_destinations: BTreeSet::from(["crm.acme.test:443".to_owned()]),
            filesystem: crate::manifest::FilesystemScope::WorkspaceOnly,
            secret_refs: BTreeSet::from(["acme-crm-token".to_owned()]),
            data_categories: BTreeSet::new(),
            platform_requirements: BTreeSet::new(),
            update_source: "connectors.acme.test".to_owned(),
            min_runtime: "0.1.0".to_owned(),
        }
    }

    #[test]
    fn capability_growth_requires_review() {
        let mut registry = ExtensionRegistry::new(OrgPolicyCeiling::default());
        registry.admit(manifest_v("1.0.0", &["api.read"])).unwrap();

        // 1.1.0 adds a capability: blocked, prior version stays active.
        let err = registry
            .admit(manifest_v("1.1.0", &["api.read", "crm.quote.update"]))
            .unwrap_err();
        match err {
            RegistryError::CapabilityGrowthRequiresReview { added } => {
                assert_eq!(added, vec!["crm.quote.update".to_owned()]);
            }
            other => panic!("expected growth review, got {other:?}"),
        }
        // Prior version is still what the registry serves.
        assert_eq!(registry.get("acme-crm").unwrap().manifest.version, "1.0.0");
    }

    #[test]
    fn same_surface_upgrade_is_approved() {
        let mut registry = ExtensionRegistry::new(OrgPolicyCeiling::default());
        registry.admit(manifest_v("1.0.0", &["api.read"])).unwrap();
        let record = registry.admit(manifest_v("1.0.1", &["api.read"])).unwrap();
        assert_eq!(record.review, ReviewState::Approved);
        assert_eq!(record.manifest.version, "1.0.1");
    }

    #[test]
    fn org_ceiling_denies_origin_and_license() {
        let mut ceiling = OrgPolicyCeiling::default();
        ceiling
            .denied_origins
            .insert("https://random-marketplace.test".to_owned());
        ceiling.denied_licenses.insert("AGPL-3.0".to_owned());

        let mut registry = ExtensionRegistry::new(ceiling);
        let mut m = manifest_v("1.0.0", &["api.read"]);
        m.origin = "https://random-marketplace.test".to_owned();
        let err = registry.admit(m).unwrap_err();
        assert!(matches!(err, RegistryError::CeilingViolation { .. }));

        let mut m2 = manifest_v("1.0.0", &["api.read"]);
        m2.license = "AGPL-3.0".to_owned();
        let err2 = registry.admit(m2).unwrap_err();
        assert!(matches!(err2, RegistryError::CeilingViolation { .. }));
    }

    #[test]
    fn unexpected_network_target_denied() {
        let mut registry = ExtensionRegistry::new(OrgPolicyCeiling::default());
        registry.admit(manifest_v("1.0.0", &["api.read"])).unwrap();
        assert!(registry
            .check_network_target("acme-crm", "crm.acme.test:443")
            .is_ok());
        assert!(matches!(
            registry.check_network_target("acme-crm", "evil.example.test:443"),
            Err(RegistryError::NetworkTargetDenied { .. })
        ));
    }

    #[test]
    fn invalid_manifest_fails_with_all_violations() {
        let mut registry = ExtensionRegistry::new(OrgPolicyCeiling::default());
        let mut m = manifest_v("1.0.0", &[]);
        m.id = String::new();
        let err = registry.admit(m).unwrap_err();
        match err {
            RegistryError::InvalidManifest(violations) => {
                assert!(violations.len() >= 2, "{violations:?}");
            }
            other => panic!("expected InvalidManifest, got {other:?}"),
        }
    }
}

#[test]
fn pending_upgrade_approve_and_reject() {
    fn manifest_v(id_version: &str, caps: &[&str]) -> ConnectorManifest {
        use crate::manifest::{FilesystemScope, IntegrationKind};
        ConnectorManifest {
            id: "acme-crm".to_owned(),
            version: id_version.to_owned(),
            kind: IntegrationKind::Connector,
            origin: "https://connectors.acme.test/crm".to_owned(),
            license: "Apache-2.0".to_owned(),
            capabilities: caps.iter().map(|s| (*s).to_owned()).collect(),
            side_effects: vec![],
            network_destinations: BTreeSet::from(["crm.acme.test:443".to_owned()]),
            filesystem: FilesystemScope::WorkspaceOnly,
            secret_refs: BTreeSet::from(["acme-crm-token".to_owned()]),
            data_categories: BTreeSet::new(),
            platform_requirements: BTreeSet::new(),
            update_source: "connectors.acme.test".to_owned(),
            min_runtime: "0.1.0".to_owned(),
        }
    }
    let mut registry = ExtensionRegistry::new(OrgPolicyCeiling::default());
    registry.admit(manifest_v("1.0.0", &["api.read"])).unwrap();
    let new = manifest_v("1.1.0", &["api.read", "crm.quote.update"]);
    let _ = registry.admit(new); // Err(growth) but parks the pending manifest.

    let approved = registry.approve_upgrade("acme-crm").unwrap();
    assert_eq!(approved.review, ReviewState::Approved);
    assert_eq!(approved.manifest.version, "1.1.0");
    assert_eq!(registry.get("acme-crm").unwrap().manifest.version, "1.1.0");

    // A subsequent growth from 1.1.0 requires review again.
    let err = registry
        .admit(manifest_v(
            "1.2.0",
            &["api.read", "crm.quote.update", "email.send"],
        ))
        .unwrap_err();
    assert!(matches!(
        err,
        RegistryError::CapabilityGrowthRequiresReview { .. }
    ));
    registry.reject_upgrade("acme-crm");
    assert_eq!(registry.get("acme-crm").unwrap().manifest.version, "1.1.0");
}
