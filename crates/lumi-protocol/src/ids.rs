//! Opaque identifier newtypes (spec 01 §1.2).
//!
//! All externally persisted IDs are opaque strings. They MUST NOT encode
//! secrets. New IDs are generated as UUIDv7 (time-ordered, globally unique).
//! Serialization is transparent: an ID is a JSON string.

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

macro_rules! opaque_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// Generates a fresh globally-unique identifier (UUIDv7).
            #[must_use]
            pub fn generate() -> Self {
                Self(uuid::Uuid::now_v7().to_string())
            }

            /// Wraps an externally-provided opaque id.
            ///
            /// # Errors
            /// Rejects empty strings; IDs must be non-empty.
            pub fn parse(value: impl Into<String>) -> Result<Self, String> {
                let value = value.into();
                if value.is_empty() {
                    return Err(concat!("empty id for ", stringify!($name)).to_owned());
                }
                if value.len() > 512 {
                    return Err(concat!("id too long for ", stringify!($name)).to_owned());
                }
                Ok(Self(value))
            }

            /// The opaque identifier string.
            #[must_use]
            pub const fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                Self::parse(s).map_err(de::Error::custom)
            }
        }
    };
}

opaque_id!(
    /// Tenant: the security and data-isolation boundary.
    TenantId
);
opaque_id!(
    /// Organization within/across tenant policy structures.
    OrganizationId
);
opaque_id!(
    /// Human user principal.
    UserId
);
opaque_id!(
    /// Principal id for any principal kind (user, workflow, schedule…).
    PrincipalId
);
opaque_id!(
    /// Local execution host.
    DeviceId
);
opaque_id!(
    /// A user/business goal.
    TaskId
);
opaque_id!(
    /// One execution attempt of a task.
    RunId
);
opaque_id!(
    /// Reusable versioned automation contract.
    WorkflowId
);
opaque_id!(
    /// Semantic version string of a workflow (e.g. `"1.2.0"`).
    WorkflowVersion
);
opaque_id!(
    /// A step within a workflow/run.
    StepId
);
opaque_id!(
    /// A normalized action proposal.
    ActionId
);
opaque_id!(
    /// A scoped human approval.
    ApprovalId
);
opaque_id!(
    /// A piece of evidence.
    EvidenceId
);
opaque_id!(
    /// A user-visible output artifact.
    ArtifactId
);
opaque_id!(
    /// Correlation id for a model provider request.
    ProviderRequestId
);
opaque_id!(
    /// Registered connector instance.
    ConnectorInstanceId
);
opaque_id!(
    /// Durable project identity (spec 26 §26.3): stable across restarts
    /// and independent of display name and filesystem path.
    ProjectId
);
opaque_id!(
    /// Execution environment identity (spec 01 §1.16): the local host
    /// that owns filesystem, sessions, and credentials for a project.
    EnvironmentId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_unique_ids() {
        let a = TaskId::generate();
        let b = TaskId::generate();
        assert_ne!(a, b);
        assert!(!a.as_str().is_empty());
    }

    #[test]
    fn rejects_empty_and_oversized() {
        assert!(TenantId::parse("").is_err());
        assert!(TenantId::parse(" ").is_ok());
        let long = "x".repeat(513);
        assert!(TenantId::parse(long).is_err());
    }

    #[test]
    fn serde_transparent_string() {
        let id = WorkflowId::parse("crm-quote-intake").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"crm-quote-intake\"");
        let back: WorkflowId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
        assert!(serde_json::from_str::<WorkflowId>("\"\"").is_err());
    }
}
