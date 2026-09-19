//! Schema envelopes and protocol-level error handling (spec 01 §1.15).
//!
//! Every persisted protocol message MUST carry `schema_name` and
//! `schema_version`. Unknown schema versions and unknown enum values MUST
//! fail explicitly rather than being silently coerced. This module provides
//! the checked [`Envelope`] and the error type surfaced by protocol
//! deserialization.

use serde::{Deserialize, Serialize};

/// Current protocol version carried by all v1 messages.
pub const PROTOCOL_VERSION: &str = "1.0";

/// Canonical `schema_name` values for v1 messages.
pub mod schema_names {
    pub const ACTION: &str = "lumi.action";
    pub const OBSERVATION: &str = "lumi.observation";
    pub const EXECUTION_RESULT: &str = "lumi.execution-result";
    pub const AUDIT_EVENT: &str = "lumi.audit-event";
}

/// Envelope persisted alongside every protocol message.
///
/// Deserialization goes through [`Envelope::ensure`], which rejects unknown
/// names/versions instead of coercing them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub schema_name: String,
    pub schema_version: String,
}

impl Envelope {
    /// Builds the envelope for `schema_name` at the current protocol version.
    #[must_use]
    pub fn new(schema_name: &'static str) -> Self {
        Self {
            schema_name: schema_name.to_owned(),
            schema_version: PROTOCOL_VERSION.to_owned(),
        }
    }

    /// Fails closed unless this envelope identifies exactly
    /// (`schema_name`, [`PROTOCOL_VERSION`]).
    ///
    /// # Errors
    /// Returns [`ProtocolError::UnsupportedSchema`] for any other
    /// name/version pair; migrations (spec 20) will extend this later.
    pub fn ensure(&self, schema_name: &str) -> Result<(), ProtocolError> {
        if self.schema_name != schema_name {
            return Err(ProtocolError::UnsupportedSchema {
                expected: schema_name.to_owned(),
                found: self.schema_name.clone(),
            });
        }
        if self.schema_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedSchemaVersion {
                schema: self.schema_name.clone(),
                expected: PROTOCOL_VERSION.to_owned(),
                found: self.schema_version.clone(),
            });
        }
        Ok(())
    }
}

/// Errors raised while parsing Lumi protocol messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// A message claimed a different `schema_name` than expected.
    UnsupportedSchema { expected: String, found: String },
    /// A message used a schema version this runtime does not understand.
    UnsupportedSchemaVersion {
        schema: String,
        expected: String,
        found: String,
    },
    /// A required structural invariant was violated (e.g. non-object
    /// `arguments`, empty id, zero timeout).
    Malformed(String),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema { expected, found } => {
                write!(
                    f,
                    "unsupported schema: expected {expected:?}, found {found:?}"
                )
            }
            Self::UnsupportedSchemaVersion {
                schema,
                expected,
                found,
            } => write!(
                f,
                "unsupported schema version for {schema}: expected {expected}, found {found}"
            ),
            Self::Malformed(detail) => write!(f, "malformed protocol message: {detail}"),
        }
    }
}

impl std::error::Error for ProtocolError {}
