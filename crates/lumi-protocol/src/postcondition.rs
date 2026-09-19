//! Machine-checkable postconditions (spec 11 §11.6).
//!
//! A postcondition is a claim that must hold after an action executes.
//! "The model says done" is not a verifier: every consequential action
//! carries postconditions that a deterministic verifier can evaluate.
//!
//! The check vocabulary here is the protocol-level description; evaluation
//! implementations live with the verifier (runtime/audit layer). Unknown
//! check kinds fail explicitly on deserialization — a workflow pack that
//! names a check this runtime cannot evaluate must fail closed, not skip.

use crate::resource::ResourceRef;
use serde::{Deserialize, Serialize};

/// Stable id for a postcondition within an action/workflow step.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PostconditionId(pub String);

impl PostconditionId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for PostconditionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Check descriptor for a postcondition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "check", rename_all = "snake_case")]
pub enum PostconditionCheck {
    /// A record of the given type/id exists in the connected system.
    RecordExists { resource: ResourceRef },
    /// A named field on a record equals an expected value (string compare
    /// of canonical JSON scalar).
    RecordFieldEquals {
        resource: ResourceRef,
        field: String,
        expected: serde_json::Value,
    },
    /// Local file exists with the expected SHA-256 (hex, lowercase).
    FileChecksum {
        /// Workspace-relative path.
        path: String,
        sha256: String,
    },
    /// Local file exists and is non-empty.
    FileExists { path: String },
    /// Artifact validated (schema/checksum) with the expected status.
    ArtifactValid { artifact_id: crate::ids::ArtifactId },
    /// Remote state probe: a queryable endpoint/selector must yield the
    /// expected canonical JSON value. Used for independent verification
    /// through a different observation path than the action (spec 11 §11.9).
    RemoteStateMatches {
        /// Verifier-registered probe id (connector query, DOM query…).
        probe_id: String,
        expected: serde_json::Value,
    },
    /// Custom verifier registered with the runtime; parameters are
    /// verifier-defined. The runtime fails closed if no verifier with this
    /// id exists.
    Custom {
        verifier_id: String,
        #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
        params: serde_json::Value,
    },
}

/// A required post-action claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Postcondition {
    pub id: PostconditionId,
    /// What this claim proves, in human terms (shown in approval UX and
    /// evidence).
    pub description: String,
    #[serde(flatten)]
    pub check: PostconditionCheck,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{ResourceRef, ResourceType};

    fn record_field_check() -> Postcondition {
        Postcondition {
            id: PostconditionId::new("draft-exists"),
            description: "Email draft exists with expected subject".to_owned(),
            check: PostconditionCheck::RecordFieldEquals {
                resource: ResourceRef {
                    resource_type: ResourceType::well_known(ResourceType::EMAIL_DRAFT),
                    id: "draft-42".to_owned(),
                    sensitivity: None,
                },
                field: "subject".to_owned(),
                expected: serde_json::json!("Q3 reconciliation"),
            },
        }
    }

    #[test]
    fn tagged_check_serde_round_trip() {
        let p = record_field_check();
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"check\":\"record_field_equals\""), "{json}");
        let back: Postcondition = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn unknown_check_kind_fails_closed() {
        let err = serde_json::from_str::<Postcondition>(
            r#"{"id":"x","description":"d","check":"teleport_record"}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown variant"), "{err}");
    }
}
