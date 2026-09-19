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
    /// A resolved identity or idempotency field was not a nonempty string.
    InvalidTemplateValue { step: String, field: String },
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
            Self::InvalidTemplateValue { step, field } => {
                write!(
                    f,
                    "step {step:?}: {field} must resolve to a nonempty string"
                )
            }
        }
    }
}

impl std::error::Error for PackRunError {}

/// Resolves `$input.<dotted.path>` and `${input.<dotted.path>}` leaves against validated
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
            let mut interpolated = String::with_capacity(text.len());
            let mut rest = text.as_str();
            while let Some((start, end, reference)) = next_reference(rest, step_id)? {
                let resolved = lookup_input(inputs, reference, step_id)?;
                if start == 0 && end == text.len() && rest.len() == text.len() {
                    return Ok(resolved.clone());
                }
                interpolated.push_str(&rest[..start]);
                match resolved {
                    serde_json::Value::String(value) => interpolated.push_str(value),
                    other => interpolated.push_str(&other.to_string()),
                }
                // Input text is appended once, never interpreted as a new template.
                rest = &rest[end..];
            }
            interpolated.push_str(rest);
            Ok(serde_json::Value::String(interpolated))
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
    if pack.steps.iter().any(|step| !step.preconditions.is_empty()) {
        return Err(PackRunError::InvalidPack(vec![
            "pack preconditions are not implemented by this runner; refusing preparation"
                .to_owned(),
        ]));
    }
    validate_schema(&pack.manifest.inputs, inputs).map_err(PackRunError::InvalidInputs)?;

    let mut actions = Vec::with_capacity(pack.steps.len());
    for (index, step) in pack.steps.iter().enumerate() {
        let arguments = resolve_value(&step.arguments_template, inputs, &step.step_id)?;
        let resource_id = resolve_identity(
            &step.resource_id_template,
            inputs,
            &step.step_id,
            "resource_id",
        )?;
        let target_canonical =
            resolve_identity(&step.target_template, inputs, &step.step_id, "target")?;
        let idempotency_key = step
            .idempotency
            .key
            .as_deref()
            .map(|key| resolve_identity(key, inputs, &step.step_id, "idempotency_key"))
            .transpose()?;

        let mut preferences = ExecutionPreferences {
            allowed_tiers: step.allowed_fallback_tiers.clone(),
            preferred_tier: Some(step.preferred_tier),
        };
        if preferences.allowed_tiers.is_empty() {
            preferences.allowed_tiers = vec![step.preferred_tier];
        }

        let proposal = ActionProposal::builder(
            // Stable for reconstruction of this run, distinct across tenants,
            // tasks and runs. Hash the tuple to avoid delimiter collisions and
            // oversized identifiers. Input changes retain the same action id
            // so journal/approval digest checks can detect material mutation.
            ActionId::parse(format!(
                "pack-action-{}",
                lumi_protocol::canonical::sha256_canonical(&serde_json::json!([
                    principal.tenant_id.as_str(),
                    task_id.as_str(),
                    run_id.as_str(),
                    pack.manifest.pack_id,
                    step.step_id,
                    index,
                ]))
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
            key: idempotency_key,
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

/// A dotted path grammar shared by whole and embedded references. A period
/// belongs to a path only when followed by another nonempty segment.
fn reference_length(text: &str) -> usize {
    let mut end = 0;
    let mut characters = text.char_indices().peekable();
    while let Some((offset, character)) = characters.next() {
        if character.is_alphanumeric() || character == '_' {
            end = offset + character.len_utf8();
        } else if character == '.'
            && end > 0
            && characters
                .peek()
                .is_some_and(|(_, next)| next.is_alphanumeric() || *next == '_')
        {
            end = offset + 1;
        } else {
            break;
        }
    }
    end
}

fn lookup_input<'a>(
    inputs: &'a serde_json::Value,
    reference: &str,
    step: &str,
) -> Result<&'a serde_json::Value, PackRunError> {
    reference
        .split('.')
        .try_fold(inputs, |current, segment| current.get(segment))
        .ok_or_else(|| PackRunError::UnresolvedTemplate {
            step: step.to_owned(),
            reference: reference.to_owned(),
        })
}

/// Explicit `${input.path}` ends a reference before a filename suffix.
/// Unbraced references retain the full dotted-path grammar.
fn next_reference<'a>(
    text: &'a str,
    step: &str,
) -> Result<Option<(usize, usize, &'a str)>, PackRunError> {
    let start = match (text.find("$input."), text.find("${input.")) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return Ok(None),
    };
    let braced = text[start..].starts_with("${input.");
    let prefix = if braced { "${input." } else { "$input." };
    let after = &text[start + prefix.len()..];
    let length = if braced {
        after
            .find('}')
            .ok_or_else(|| PackRunError::InvalidTemplateValue {
                step: step.to_owned(),
                field: "unterminated input reference".to_owned(),
            })?
    } else {
        reference_length(after)
    };
    let reference = &after[..length];
    if reference.is_empty() || reference_length(reference) != reference.len() {
        return Err(PackRunError::InvalidTemplateValue {
            step: step.to_owned(),
            field: "input reference".to_owned(),
        });
    }
    Ok(Some((
        start,
        start + prefix.len() + length + usize::from(braced),
        reference,
    )))
}

fn resolve_identity(
    template: &str,
    inputs: &serde_json::Value,
    step: &str,
    field: &str,
) -> Result<String, PackRunError> {
    let mut rest = template;
    while let Some((_, end, reference)) = next_reference(rest, step)? {
        if lookup_input(inputs, reference, step)?
            .as_str()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(PackRunError::InvalidTemplateValue {
                step: step.to_owned(),
                field: field.to_owned(),
            });
        }
        rest = &rest[end..];
    }
    let resolved = resolve_value(
        &serde_json::Value::String(template.to_owned()),
        inputs,
        step,
    )?;
    resolved
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| PackRunError::InvalidTemplateValue {
            step: step.to_owned(),
            field: field.to_owned(),
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
    fn embedded_nested_paths_and_suffixes_resolve_without_changing_target() {
        let inputs = serde_json::json!({"customer": {"id": "C-14"}, "quote": "Q-7"});
        for (template, expected) in [
            (
                "crm://accounts/$input.customer.id/quotes/$input.quote",
                "crm://accounts/C-14/quotes/Q-7",
            ),
            ("send-$input.customer.id-$input.quote", "send-C-14-Q-7"),
            ("$input.quote-suffix", "Q-7-suffix"),
            ("Quote $input.quote.", "Quote Q-7."),
            ("report-${input.customer.id}.json", "report-C-14.json"),
        ] {
            assert_eq!(
                resolve_identity(template, &inputs, "s1", "target").unwrap(),
                expected
            );
        }
        assert!(resolve_identity("crm://$input.customer", &inputs, "s1", "target").is_err());
        assert!(
            resolve_identity("crm://$input.customer.missing", &inputs, "s1", "target").is_err()
        );
        assert!(resolve_identity("crm://${input.customer.id", &inputs, "s1", "target").is_err());
        let literal = serde_json::json!({"id":"$input.other", "other":"must-not-expand"});
        assert_eq!(
            resolve_identity("id/${input.id}.json", &literal, "s1", "target").unwrap(),
            "id/$input.other.json"
        );
    }

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
