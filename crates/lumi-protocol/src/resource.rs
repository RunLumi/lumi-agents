//! Policy-addressable resources and capabilities (spec 01 §1.9–1.10).

use crate::ids::TenantId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of a policy-addressable resource. Open vocabulary with well-known
/// constants; policy rules match on exact strings, and unknown types fail
/// closed at the policy layer (spec 04 §4.5) rather than here.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceType(pub String);

impl ResourceType {
    /// Local file.
    pub const FILE: &'static str = "file";
    /// CRM quote record.
    pub const CRM_QUOTE: &'static str = "crm_quote";
    /// Email draft.
    pub const EMAIL_DRAFT: &'static str = "email_draft";
    /// Sent email message.
    pub const EMAIL_MESSAGE: &'static str = "email_message";
    /// Invoice.
    pub const INVOICE: &'static str = "invoice";
    /// Browser origin (scheme + host + port).
    pub const BROWSER_ORIGIN: &'static str = "browser_origin";
    /// Locally installed application.
    pub const LOCAL_APP: &'static str = "local_app";
    /// Database record.
    pub const DATABASE_RECORD: &'static str = "database_record";
    /// External destination (export/share target).
    pub const EXTERNAL_DESTINATION: &'static str = "external_destination";

    /// Builds a resource type from a non-empty string.
    ///
    /// # Errors
    /// Rejects empty strings.
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.is_empty() {
            return Err("resource type must not be empty".to_owned());
        }
        Ok(Self(value))
    }

    /// Well-known resource type constant.
    #[must_use]
    pub fn well_known(name: &'static str) -> Self {
        Self(name.to_owned())
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Sensitivity label used for redaction, evidence, and export decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SensitivityLabel {
    Public,
    Internal,
    Confidential,
    Restricted,
    /// Data whose egress is governed by privacy regulation.
    PersonalData,
}

/// A policy-addressable business/technical object (spec 01 §1.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    /// Canonical identifier within the type (path, URL, record id…).
    pub id: String,
    pub tenant_id: TenantId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<SensitivityLabel>,
}

/// Resource reference carried inside action proposals. Identifies the
/// resource class + canonical id; tenant scope comes from the proposal's
/// principal so it cannot be spoofed per-resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRef {
    pub resource_type: ResourceType,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity: Option<SensitivityLabel>,
}

impl From<&Resource> for ResourceRef {
    fn from(r: &Resource) -> Self {
        Self {
            resource_type: r.resource_type.clone(),
            id: r.id.clone(),
            sensitivity: r.sensitivity,
        }
    }
}

/// Permission to *attempt* a category of action (spec 01 §1.10).
///
/// Dotted lowercase segments, e.g. `email.send`. Capability does not imply
/// policy authorization for every target; policy still evaluates the full
/// normalized action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Capability(pub String);

impl Capability {
    /// Builds a capability from a string, validating its shape.
    ///
    /// # Errors
    /// Rejects empty segments, non-lowercase segments, and names without a
    /// dot-separated family.
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        let segments: Vec<&str> = value.split('.').collect();
        if segments.len() < 2 || segments.iter().any(|s| s.is_empty()) {
            return Err(format!("capability must be dotted segments: {value:?}"));
        }
        if segments.iter().any(|s| {
            !s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        }) {
            return Err(format!(
                "capability segments must be lowercase [a-z0-9_]: {value:?}"
            ));
        }
        Ok(Self(value))
    }

    /// Well-known capability constant (already validated shape).
    #[must_use]
    pub fn well_known(name: &'static str) -> Self {
        debug_assert!(
            Self::parse(name).is_ok(),
            "invalid well-known capability {name}"
        );
        Self(name.to_owned())
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The capability family (first segment), e.g. `email` of `email.send`.
    #[must_use]
    pub fn family(&self) -> &str {
        self.0.split('.').next().unwrap_or_default()
    }
}

/// Well-known v1 capabilities.
pub mod capabilities {
    use super::Capability;

    pub const BROWSER_READ: &str = "browser.read";
    pub const BROWSER_WRITE: &str = "browser.write";
    pub const FILES_READ: &str = "files.read";
    pub const FILES_WRITE: &str = "files.write";
    pub const CRM_QUOTE_UPDATE: &str = "crm.quote.update";
    pub const EMAIL_DRAFT_CREATE: &str = "email.draft.create";
    pub const EMAIL_SEND: &str = "email.send";
    pub const SHELL_EXECUTE: &str = "shell.execute";
    pub const DESKTOP_INTERACT: &str = "desktop.interact";
    pub const ARTIFACT_CREATE: &str = "artifact.create";
    pub const DATA_EXPORT: &str = "data.export";

    /// The canonical read-only capabilities: allowed without approval under
    /// default policy when the action risk is also READ.
    pub fn read_only() -> [Capability; 4] {
        [
            Capability::well_known(BROWSER_READ),
            Capability::well_known(FILES_READ),
            Capability::well_known("desktop.read"),
            Capability::well_known("api.read"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_shape_validation() {
        assert!(Capability::parse(capabilities::EMAIL_SEND).is_ok());
        assert!(Capability::parse("send").is_err());
        assert!(Capability::parse("email..send").is_err());
        assert!(Capability::parse("Email.Send").is_err());
        assert!(Capability::parse("email.send_extra").is_ok());
        assert_eq!(
            Capability::parse(capabilities::EMAIL_SEND)
                .unwrap()
                .family(),
            "email"
        );
    }

    #[test]
    fn resource_ref_from_resource_preserves_fields() {
        let r = Resource {
            resource_type: ResourceType::well_known(ResourceType::EMAIL_DRAFT),
            id: "draft-42".to_owned(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            sensitivity: Some(SensitivityLabel::Confidential),
        };
        let rref = ResourceRef::from(&r);
        assert_eq!(rref.resource_type, r.resource_type);
        assert_eq!(rref.id, "draft-42");
        assert_eq!(rref.sensitivity, Some(SensitivityLabel::Confidential));
    }

    #[test]
    fn sensitivity_serde() {
        let json = serde_json::to_string(&SensitivityLabel::PersonalData).unwrap();
        assert_eq!(json, "\"PERSONAL_DATA\"");
        assert!(serde_json::from_str::<SensitivityLabel>("\"TSHIRT\"").is_err());
    }
}
