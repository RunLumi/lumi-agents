//! Pack manifest, schemas, compatibility, economics (spec 12 §12.4–12.5,
//! §12.8–12.9).

use lumi_protocol::Budget;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Hardening lifecycle (spec 12 §12.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PackStatus {
    Experimental,
    Alpha,
    /// Requires spec 21 certification gates.
    Certified,
    Deprecated,
}

/// Privacy classification of data the pack touches (§12.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrivacyClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
    PersonalData,
}

/// Input/output field schema (§12.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    /// Coarse type: `string` | `number` | `boolean` | `object` | `array`.
    pub value_type: ValueSchema,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

/// Coarse value types for v1 pack schemas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueSchema {
    String,
    Number,
    Boolean,
    Object,
    Array,
}

impl std::fmt::Display for ValueSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Object => "object",
            Self::Array => "array",
        };
        f.write_str(name)
    }
}

impl ValueSchema {
    fn accepts(&self, value: &serde_json::Value) -> bool {
        match self {
            Self::String => value.is_string(),
            Self::Number => value.is_number(),
            Self::Boolean => value.is_boolean(),
            Self::Object => value.is_object(),
            Self::Array => value.is_array(),
        }
    }
}

/// A schema-defined input/output set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SchemaFieldSet {
    pub fields: Vec<SchemaField>,
}

impl SchemaFieldSet {
    /// Validates a value object against the schema. Unknown REQUIRED
    /// fields and type mismatches fail explicitly (§12.5).
    ///
    /// # Errors
    /// Human-readable violations list.
    pub fn validate(&self, value: &serde_json::Value) -> Result<(), Vec<String>> {
        let object = value
            .as_object()
            .ok_or_else(|| vec!["input/output must be a JSON object".to_owned()])?;
        let mut violations = Vec::new();
        for field in &self.fields {
            match object.get(&field.name) {
                None => {
                    if field.required {
                        violations.push(format!(
                            "missing required field {:?} ({})",
                            field.name, field.value_type
                        ));
                    }
                }
                Some(v) => {
                    if !field.value_type.accepts(v) {
                        violations.push(format!(
                            "field {:?} expected {}, got {}",
                            field.name,
                            field.value_type,
                            kind_of(v)
                        ));
                    }
                }
            }
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

fn kind_of(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Tested combinations (§12.8). "macOS supported" is not a matrix entry:
/// each dimension lists the concrete versions a deployment may assume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CompatibilityMatrix {
    /// Supported runtime versions (semver ranges, e.g. `>=0.1, <0.2`).
    #[serde(default)]
    pub runtime_versions: BTreeSet<String>,
    /// Supported OS entries, e.g. `macos-14`, `windows-11-23h2`.
    #[serde(default)]
    pub operating_systems: BTreeSet<String>,
    /// Material app/browser versions, e.g. `chrome-130`.
    #[serde(default)]
    pub app_versions: BTreeSet<String>,
    /// Connector versions, e.g. `crm-connector@1.2`.
    #[serde(default)]
    pub connector_versions: BTreeSet<String>,
}

/// Economic baseline (§12.9). Mirrors the eval crate's economics inputs;
/// kept as pack-owned declared baselines (evidence feeds actuals).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EconomicBaseline {
    pub manual_minutes_per_run: u32,
    pub residual_human_minutes_per_run: u32,
    pub runs_per_month: u32,
    /// Loaded labor cost per hour, micro-USD.
    pub loaded_labor_cost_per_hour_micro_usd: u64,
    /// Expected runtime variable cost per run, micro-USD.
    pub runtime_variable_cost_per_run_micro_usd: u64,
    /// Implementation hours (reuse metric input, §12.12).
    pub implementation_hours: u32,
    /// One-line payback hypothesis, reviewed per deployment.
    pub payback_hypothesis: String,
}

/// The pack manifest (§12.2, §12.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackManifest {
    pub pack_id: String,
    /// Semantic version, e.g. `1.2.0`.
    pub version: String,
    pub owner: String,
    pub status: PackStatus,
    /// Supported runtime version range.
    pub runtime_version_range: String,
    pub supported_os: BTreeSet<String>,
    /// Required connector ids.
    #[serde(default)]
    pub required_connectors: BTreeSet<String>,
    /// Required model capability names, e.g. `tool_use`.
    #[serde(default)]
    pub required_model_capabilities: BTreeSet<String>,
    pub privacy_classification: PrivacyClassification,
    /// Default budgets applied to runs of this pack.
    pub default_budget: Budget,
    pub inputs: SchemaFieldSet,
    pub outputs: SchemaFieldSet,
    pub compatibility: CompatibilityMatrix,
    pub economics: EconomicBaseline,
}

impl PackManifest {
    /// Structural validation beyond field types.
    ///
    /// # Errors
    /// Violations list (empty pack_id, bad version, certified status
    /// without compatibility entries, etc.).
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();
        if self.pack_id.trim().is_empty() {
            violations.push("pack_id must not be empty".to_owned());
        }
        let parts: Vec<&str> = self.version.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|p| p.parse::<u64>().is_err()) {
            violations.push(format!(
                "version {:?} is not semantic (major.minor.patch)",
                self.version
            ));
        }
        if self.owner.trim().is_empty() {
            violations.push("owner must not be empty".to_owned());
        }
        // A certified pack must declare a real compatibility matrix.
        if self.status == PackStatus::Certified && self.compatibility.operating_systems.is_empty() {
            violations.push(
                "certified packs must declare supported OS entries in the compatibility matrix"
                    .to_owned(),
            );
        }
        if self.inputs.fields.is_empty() {
            violations.push("input schema must define at least one field".to_owned());
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}
