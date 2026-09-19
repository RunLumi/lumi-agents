//! Pack runner: turns a validated pack + schema-validated inputs into a
//! sequence of normalized, policy-gated actions.
//!
//! The runner is deliberately thin over [`lumi_protocol::ActionProposal`]:
//! every step becomes one action proposal, so policy, approval, journal,
//! verification, and audit all flow through the same orchestrator gate as
//! every other execution path.

use crate::manifest::SchemaFieldSet;
use crate::step::{ApprovalRule, PackStep, WorkflowPack};
use lumi_protocol::{
    ActionId, ActionProposal, ExecutionPreferences, Idempotency, Principal, ResourceRef,
    ResourceType, RunId, Target, TaskId,
};

/// Errors from pack preparation (before any execution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackRunError {
    /// Inputs failed the pack's input schema.
    InvalidInputs(Vec<String>),
    /// The pack itself failed structural validation.
    InvalidPack(Vec<String>),
    /// An argument template referenced a missing input field.
    UnresolvedTemplate { step: String, reference: String },
}

impl std::fmt::Display for PackRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInputs(violations) => {
                write!(f, "invalid inputs: {}", violations.join("; "))
            }
            Self::InvalidPack(violations) => {
                write!(f, "invalid pack: {}", violations.join("; "))
            }
            Self::UnresolvedTemplate { step, reference } => {
                write!(
                    f,
                    "step {step:?}: unresolved template reference {reference:?}"
                )
            }
        }
    }
}

impl std::error::Error for PackRunError {}

/// Resolves `$input.<dotted.path>` string leaves against validated
/// inputs.
///
/// # Errors
/// [`PackRunError::UnresolvedTemplate`] for references missing from
/// inputs.
pub fn resolve_value(
    value: &serde_json::Value,
    inputs: &serde_json::Value,
    step_id: &str,
) -> Result<serde_json::Value, PackRunError> {
    match value {
        serde_json::Value::String(text) => {
            if let Some(reference) = text.strip_prefix("$input.") {
                // Whole-value reference: preserve the input's JSON type.
                let resolved = reference
                    .split('.')
                    .try_fold(inputs, |current, segment| current.get(segment))
                    .ok_or_else(|| PackRunError::UnresolvedTemplate {
                        step: step_id.to_owned(),
                        reference: reference.to_owned(),
                    })?;
                Ok(resolved.clone())
            } else if text.contains("$input.") {
                // Embedded reference(s): interpolate stringified values.
                let mut interpolated = String::with_capacity(text.len());
                let mut rest = text.as_str();
                while let Some(start) = rest.find("$input.") {
                    interpolated.push_str(&rest[..start]);
                    let after = &rest[start + "$input.".len()..];
                    let end = after
                        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .unwrap_or(after.len());
                    let reference = &after[..end];
                    let resolved = reference
                        .split('.')
                        .try_fold(inputs, |current, segment| current.get(segment))
                        .ok_or_else(|| PackRunError::UnresolvedTemplate {
                            step: step_id.to_owned(),
                            reference: reference.to_owned(),
                        })?;
                    match resolved {
                        serde_json::Value::String(s) => interpolated.push_str(s),
                        other => interpolated.push_str(&other.to_string()),
                    }
                    rest = &after[end..];
                }
                interpolated.push_str(rest);
                Ok(serde_json::Value::String(interpolated))
            } else {
                Ok(value.clone())
            }
        }
        serde_json::Value::Object(map) => {
            let mut resolved = serde_json::Map::new();
            for (key, inner) in map {
                resolved.insert(key.clone(), resolve_value(inner, inputs, step_id)?);
            }
            Ok(serde_json::Value::Object(resolved))
        }
        serde_json::Value::Array(items) => {
            let mut resolved = Vec::with_capacity(items.len());
            for item in items {
                resolved.push(resolve_value(item, inputs, step_id)?);
            }
            Ok(serde_json::Value::Array(resolved))
        }
        other => Ok(other.clone()),
    }
}

/// A pack prepared for one run: validated inputs and the materialized
/// action proposal per step.
#[derive(Debug, Clone)]
pub struct PreparedPackRun {
    pub pack_id: String,
    /// Materialized proposal per step, in execution order.
    pub actions: Vec<(String, ActionProposal)>,
}

/// Prepares a pack run: schema validation + template resolution +
/// proposal construction. No execution happens here.
///
/// # Errors
/// [`PackRunError`] for invalid packs, invalid inputs, or unresolved
/// templates. All failures occur before any action is proposed.
pub fn prepare_run(
    pack: &WorkflowPack,
    inputs: &serde_json::Value,
    task_id: &TaskId,
    run_id: &RunId,
    principal: &Principal,
) -> Result<PreparedPackRun, PackRunError> {
    pack.validate().map_err(PackRunError::InvalidPack)?;
    validate_schema(&pack.manifest.inputs, inputs).map_err(PackRunError::InvalidInputs)?;

    let mut actions = Vec::with_capacity(pack.steps.len());
    for (index, step) in pack.steps.iter().enumerate() {
        let arguments = resolve_value(&step.arguments_template, inputs, &step.step_id)?;
        let resource_id = resolve_value(
            &serde_json::Value::String(step.resource_id_template.clone()),
            inputs,
            &step.step_id,
        )?
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| step.resource_id_template.clone());
        let target_canonical = resolve_value(
            &serde_json::Value::String(step.target_template.clone()),
            inputs,
            &step.step_id,
        )?
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| step.target_template.clone());

        let mut preferences = ExecutionPreferences {
            allowed_tiers: step.allowed_fallback_tiers.clone(),
            preferred_tier: Some(step.preferred_tier),
        };
        if preferences.allowed_tiers.is_empty() {
            preferences.allowed_tiers = vec![step.preferred_tier];
        }

        let proposal = ActionProposal::builder(
            ActionId::parse(format!(
                "{}-{}-{index}",
                pack.manifest.pack_id, step.step_id
            ))
            .map_err(|e| PackRunError::InvalidPack(vec![e]))?,
            task_id.clone(),
            run_id.clone(),
            principal.clone(),
            step.capability.clone(),
            ResourceRef {
                resource_type: ResourceType::parse(step.resource_type.clone())
                    .map_err(|e| PackRunError::InvalidPack(vec![e]))?,
                id: resource_id,
                sensitivity: Some(match pack.manifest.privacy_classification {
                    crate::manifest::PrivacyClassification::Public => {
                        lumi_protocol::SensitivityLabel::Public
                    }
                    crate::manifest::PrivacyClassification::Internal => {
                        lumi_protocol::SensitivityLabel::Internal
                    }
                    crate::manifest::PrivacyClassification::Confidential => {
                        lumi_protocol::SensitivityLabel::Confidential
                    }
                    crate::manifest::PrivacyClassification::Restricted => {
                        lumi_protocol::SensitivityLabel::Restricted
                    }
                    crate::manifest::PrivacyClassification::PersonalData => {
                        lumi_protocol::SensitivityLabel::PersonalData
                    }
                }),
            },
            Target::canonical(target_canonical),
            step.operation.clone(),
            step.risk_class,
        )
        .arguments(arguments)
        .expected_effect(lumi_protocol::ExpectedEffect {
            summary: step.purpose.clone(),
            external_visibility: step.risk_class.is_consequential(),
            reversible: !matches!(step.risk_class, lumi_protocol::RiskClass::Destructive),
        })
        .execution_preferences(preferences)
        .evidence_requirements(step.evidence.clone())
        .postconditions({
            // Postcondition checks may reference validated inputs (e.g.
            // expected = "$input.report_id"); resolve before execution so
            // verifiers compare concrete values.
            let raw = serde_json::to_value(&step.postconditions)
                .map_err(|e| PackRunError::InvalidPack(vec![e.to_string()]))?;
            let resolved = resolve_value(&raw, inputs, &step.step_id)?;
            serde_json::from_value(resolved)
                .map_err(|e| PackRunError::InvalidPack(vec![e.to_string()]))?
        })
        .idempotency(Idempotency {
            key: step.idempotency.key.clone(),
            semantics: step.idempotency.semantics,
        })
        .step_id(
            lumi_protocol::StepId::parse(step.step_id.clone())
                .map_err(|e| PackRunError::InvalidPack(vec![e]))?,
        )
        .workflow_id(
            lumi_protocol::WorkflowId::parse(pack.manifest.pack_id.clone())
                .map_err(|e| PackRunError::InvalidPack(vec![e]))?,
        )
        .build()
        .map_err(|e| PackRunError::InvalidPack(vec![e.to_string()]))?;
        actions.push((step.step_id.clone(), proposal));
    }

    Ok(PreparedPackRun {
        pack_id: pack.manifest.pack_id.clone(),
        actions,
    })
}

fn validate_schema(schema: &SchemaFieldSet, inputs: &serde_json::Value) -> Result<(), Vec<String>> {
    schema.validate(inputs)
}

/// Per-step execution helpers shared with the orchestrator loop.
impl PackStep {
    /// Whether this step must acquire a fresh scoped approval before
    /// execution, regardless of policy defaults (§12.6 approval rule).
    #[must_use]
    pub const fn requires_explicit_approval(&self) -> bool {
        matches!(self.approval_rule, ApprovalRule::Always)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_resolution_walks_nested_paths() {
        let inputs = serde_json::json!({
            "customer": {"email": "c@example.test", "name": "Acme"},
            "total": "4200.00"
        });
        let template = serde_json::json!({
            "to": "$input.customer.email",
            "body": "Your total",
            "nested": {"ref": "$input.total"},
            "literal": 42,
        });
        let resolved = resolve_value(&template, &inputs, "s1").unwrap();
        assert_eq!(resolved["to"], "c@example.test");
        assert_eq!(resolved["nested"]["ref"], "4200.00");
        assert_eq!(resolved["literal"], 42);
    }

    #[test]
    fn unresolved_references_fail_before_execution() {
        let inputs = serde_json::json!({"customer": {}});
        let template = serde_json::json!({"to": "$input.customer.email"});
        let error = resolve_value(&template, &inputs, "s1").unwrap_err();
        assert!(matches!(error, PackRunError::UnresolvedTemplate { .. }));
    }
}
