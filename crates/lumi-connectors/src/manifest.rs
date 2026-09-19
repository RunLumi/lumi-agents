//! The connector manifest (spec 14 §14.3, §14.8, §14.9).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Integration class (spec 14 §14.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationKind {
    /// Business/API integration.
    Connector,
    /// MCP capability provider.
    McpServer,
    /// Browser/native/app execution engine.
    ExecutorAdapter,
    /// Model provider.
    ModelAdapter,
    /// Artifact generator/validator.
    ArtifactAdapter,
}

/// A declared network destination (§14.8).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkDestination {
    /// Host or host:port, e.g. `api.acme.test:443`.
    pub host: String,
    /// Why the connector needs it (audit).
    pub purpose: String,
}

/// Declared side effect (§14.3/§14.6): the normalized operation + risk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffectDeclaration {
    /// Normalized business operation, e.g. `create_crm_quote`.
    pub operation: String,
    /// Canonical risk class name (validated at load).
    pub risk_class: String,
    /// Whether the effect is externally visible.
    pub externally_visible: bool,
}

/// Filesystem access scope (§14.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemScope {
    None,
    WorkspaceOnly,
    DeclaredPaths,
    Unrestricted,
}

/// Data categories accessed (§14.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataCategory {
    Public,
    InternalBusiness,
    CustomerData,
    PersonalData,
    Financial,
    Credentials,
    HealthData,
}

/// The full integration manifest (§14.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorManifest {
    pub id: String,
    /// Exact semantic version (§14.9: pinned, not floating).
    pub version: String,
    pub kind: IntegrationKind,
    /// Origin URL/repository — policy may deny by origin (§14.12/§14.16).
    pub origin: String,
    pub license: String,
    /// Capabilities this connector requires (must be covered by grants
    /// at run time — the manifest does not GRANT them, §14.4).
    pub capabilities: BTreeSet<String>,
    /// Declared side effects.
    #[serde(default)]
    pub side_effects: Vec<SideEffectDeclaration>,
    /// Declared network destinations.
    #[serde(default)]
    pub network_destinations: BTreeSet<String>,
    pub filesystem: FilesystemScope,
    /// Secret references REQUIRED (§14.5: nothing else resolves).
    #[serde(default)]
    pub secret_refs: BTreeSet<String>,
    #[serde(default)]
    pub data_categories: BTreeSet<DataCategory>,
    /// Platform requirements, e.g. `macos-14+`.
    #[serde(default)]
    pub platform_requirements: BTreeSet<String>,
    /// Update source identity (§14.3 update source).
    pub update_source: String,
    /// Minimum runtime version.
    pub min_runtime: String,
}

/// Manifest validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    InvalidField {
        field: &'static str,
        reason: String,
    },
    /// A side-effect risk class name is not canonical.
    UnknownRiskClass {
        risk: String,
    },
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField { field, reason } => {
                write!(f, "invalid manifest field {field:?}: {reason}")
            }
            Self::UnknownRiskClass { risk } => {
                write!(f, "side effect declares unknown risk class {risk:?}")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

const KNOWN_RISK_CLASSES: &[&str] = &[
    "READ",
    "LOCAL_WRITE",
    "EXTERNAL_WRITE",
    "COMMUNICATION",
    "DATA_EXPORT",
    "CREDENTIAL",
    "FINANCIAL",
    "LEGAL_CONSENT",
    "DESTRUCTIVE",
    "ADMIN",
];

impl ConnectorManifest {
    /// Structural validation (§14.3 completeness + §14.6 risk classes).
    ///
    /// # Errors
    /// [`ManifestError`] listing all violations.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();
        for (field, value) in [
            ("id", self.id.as_str()),
            ("version", self.version.as_str()),
            ("origin", self.origin.as_str()),
            ("license", self.license.as_str()),
            ("update_source", self.update_source.as_str()),
            ("min_runtime", self.min_runtime.as_str()),
        ] {
            if value.trim().is_empty() {
                violations.push(format!("{field} must not be empty"));
            }
        }
        if self.capabilities.is_empty() {
            violations.push("manifest must declare at least one capability".to_owned());
        }
        for side_effect in &self.side_effects {
            if !KNOWN_RISK_CLASSES.contains(&side_effect.risk_class.as_str()) {
                violations.push(format!(
                    "side effect {:?} declares non-canonical risk class {:?}",
                    side_effect.operation, side_effect.risk_class
                ));
            }
        }
        if self.filesystem == FilesystemScope::Unrestricted {
            violations
                .push("Unrestricted filesystem access is not permitted in v1 manifests".to_owned());
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    /// True when `destination` is among the declared network
    /// destinations (§14.8). Undeclared targets are denied by policy.
    #[must_use]
    pub fn network_destination_declared(&self, host: &str) -> bool {
        self.network_destinations.contains(host)
    }

    /// True when `reference` is a secret the manifest declares (§14.5).
    #[must_use]
    pub fn secret_declared(&self, reference: &str) -> bool {
        self.secret_refs.contains(reference)
    }

    /// The capability surface as a comparable set, for version review.
    #[must_use]
    pub fn capability_surface(&self) -> &BTreeSet<String> {
        &self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ConnectorManifest {
        ConnectorManifest {
            id: "acme-crm".to_owned(),
            version: "1.2.0".to_owned(),
            kind: IntegrationKind::Connector,
            origin: "https://connectors.acme.test/crm".to_owned(),
            license: "Apache-2.0".to_owned(),
            capabilities: BTreeSet::from(["crm.quote.update".to_owned(), "api.read".to_owned()]),
            side_effects: vec![SideEffectDeclaration {
                operation: "create_crm_quote".to_owned(),
                risk_class: "EXTERNAL_WRITE".to_owned(),
                externally_visible: true,
            }],
            network_destinations: BTreeSet::from([
                "api.acme.test:443".to_owned(),
                "crm.acme.test:443".to_owned(),
            ]),
            filesystem: FilesystemScope::WorkspaceOnly,
            secret_refs: BTreeSet::from(["acme-crm-token".to_owned()]),
            data_categories: BTreeSet::from([DataCategory::CustomerData]),
            platform_requirements: BTreeSet::new(),
            update_source: "connectors.acme.test".to_owned(),
            min_runtime: "0.1.0".to_owned(),
        }
    }

    #[test]
    fn valid_manifest_passes() {
        assert!(manifest().validate().is_ok());
    }

    #[test]
    fn non_canonical_risk_class_is_rejected() {
        let mut m = manifest();
        m.side_effects[0].risk_class = "SPICY".to_owned();
        let violations = m.validate().unwrap_err();
        assert!(violations.iter().any(|v| v.contains("SPICY")));
    }

    #[test]
    fn unrestricted_filesystem_is_rejected() {
        let mut m = manifest();
        m.filesystem = FilesystemScope::Unrestricted;
        assert!(m.validate().is_err());
    }

    #[test]
    fn network_destination_lookup() {
        let m = manifest();
        assert!(m.network_destination_declared("api.acme.test:443"));
        assert!(!m.network_destination_declared("evil.example.test:443"));
    }
}
