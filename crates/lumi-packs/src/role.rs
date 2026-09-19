//! Minimal Role Pack composition (mission §4, §7, and §19 priority 2).
//!
//! A role owns a bounded stream of work by composing existing workflow packs.
//! It does not replace pack execution, policy, approvals, verification, or
//! evidence.  The authority ceiling is checked both when the role is resolved
//! and when a prepared action envelope is handed to a workflow runner.

use crate::{ApprovalRule, SchemaField, SchemaFieldSet, ValueSchema, WorkflowPack};
use lumi_protocol::{canonical::sha256_canonical, ActionProposal, Budget, Capability, RiskClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Versioned wire identity for Role Pack manifests.
pub const ROLE_SCHEMA_NAME: &str = "lumi.role_pack";
pub const ROLE_SCHEMA_VERSION: u32 = 0;

/// A reference to one exact version of an existing workflow pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPackRef {
    pub pack_id: String,
    pub version: String,
}

/// A single queue owned by a role in v0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleQueue {
    pub queue_id: String,
    /// Field in [`RolePack::work_item_schema`] that is the stable work-item
    /// identity.  A queue must not rely on a model-generated identity.
    pub work_item_id_field: String,
    /// Named event/schedule causes.  Trigger execution remains a scheduler
    /// concern; these names are the role's input contract.
    pub triggers: Vec<String>,
}

/// A finite authority ceiling applied on top of every composed pack.
///
/// Capabilities and risk classes are exact sets for the composed role.  The
/// budgets are additional maxima and are intersected with each pack's own
/// budget at preparation time.  A role cannot widen a pack's authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleAuthorityCeiling {
    pub allowed_capabilities: BTreeSet<Capability>,
    pub allowed_risk_classes: BTreeSet<RiskClass>,
    /// Consequential risk classes that the role requires to remain approval
    /// gated.  Local policy is still the final authorizer.
    pub required_approval_risk_classes: BTreeSet<RiskClass>,
    pub max_model_cost_micro_usd: u64,
    pub max_actions: u32,
    pub max_retries: u32,
    pub max_vision_actions: u32,
    pub max_external_writes: u32,
}

/// Strict role-owned work-item schema. Existing workflow-pack schemas remain
/// unchanged; role admission rejects unknown keys in this schema envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleWorkItemSchema {
    pub fields: Vec<RoleWorkItemField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleWorkItemField {
    pub name: String,
    pub value_type: ValueSchema,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

impl RoleWorkItemSchema {
    fn as_pack_schema(&self) -> SchemaFieldSet {
        SchemaFieldSet {
            fields: self
                .fields
                .iter()
                .map(|field| SchemaField {
                    name: field.name.clone(),
                    value_type: field.value_type,
                    description: field.description.clone(),
                    required: field.required,
                })
                .collect(),
        }
    }
}

/// A bounded operational role composed from workflow packs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolePack {
    pub schema_name: String,
    pub schema_version: u32,
    pub role_id: String,
    pub version: String,
    pub owner: String,
    pub supervisor: String,
    pub job_to_be_done: String,
    pub scope: Vec<String>,
    pub non_scope: Vec<String>,
    pub queue: RoleQueue,
    pub work_item_schema: RoleWorkItemSchema,
    pub workflow_packs: Vec<WorkflowPackRef>,
    pub authority: RoleAuthorityCeiling,
}

/// Queue envelope accepted by Role Pack preparation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleWorkItem {
    pub work_item_id: String,
    pub workflow_id: String,
    pub inputs: serde_json::Value,
}

impl RolePack {
    /// Validates composition and authority before a role can be resolved.
    ///
    /// The supplied workflow packs must already be loaded from trusted local
    /// definitions.  This check still validates each pack so callers cannot
    /// accidentally compose an invalid definition.
    ///
    /// # Errors
    /// Returns every composition or authority violation.  Unknown workflow
    /// references, capability/risk expansion, approval narrowing, and budget
    /// expansion all fail closed.
    pub fn validate(&self, packs: &[WorkflowPack]) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();

        if self.schema_name != ROLE_SCHEMA_NAME {
            violations.push(format!(
                "schema_name {:?} is unsupported; expected {:?}",
                self.schema_name, ROLE_SCHEMA_NAME
            ));
        }
        if self.schema_version != ROLE_SCHEMA_VERSION {
            violations.push(format!(
                "schema_version {} is unsupported; expected {}",
                self.schema_version, ROLE_SCHEMA_VERSION
            ));
        }
        if self.role_id.trim().is_empty() {
            violations.push("role_id must not be empty".to_owned());
        }
        if !is_semver(&self.version) {
            violations.push(format!(
                "version {:?} is not semantic (major.minor.patch)",
                self.version
            ));
        }
        for (field, value) in [
            ("owner", self.owner.as_str()),
            ("supervisor", self.supervisor.as_str()),
            ("job_to_be_done", self.job_to_be_done.as_str()),
        ] {
            if value.trim().is_empty() {
                violations.push(format!("{field} must not be empty"));
            }
        }
        if self.scope.is_empty() {
            violations.push("scope must define at least one responsibility".to_owned());
        }
        if self.non_scope.is_empty() {
            violations.push("non_scope must define at least one boundary".to_owned());
        }
        if self.queue.queue_id.trim().is_empty() {
            violations.push("queue.queue_id must not be empty".to_owned());
        }
        if self.queue.work_item_id_field.trim().is_empty() {
            violations.push("queue.work_item_id_field must not be empty".to_owned());
        }
        if self.queue.triggers.is_empty()
            || self
                .queue
                .triggers
                .iter()
                .any(|trigger| trigger.trim().is_empty())
        {
            violations.push("queue.triggers must contain named triggers".to_owned());
        }
        if self.work_item_schema.fields.is_empty() {
            violations.push("work_item_schema must define fields".to_owned());
        }
        let mut seen_work_item_fields = BTreeSet::new();
        for field in &self.work_item_schema.fields {
            if field.name.trim().is_empty() {
                violations.push("work_item_schema field names must not be empty".to_owned());
            }
            if !seen_work_item_fields.insert(field.name.clone()) {
                violations.push(format!("duplicate work_item_schema field {:?}", field.name));
            }
        }
        match self
            .work_item_schema
            .fields
            .iter()
            .find(|field| field.name == self.queue.work_item_id_field)
        {
            Some(field) if field.required => {}
            Some(_) => violations.push(format!(
                "work item identity field {:?} must be required",
                self.queue.work_item_id_field
            )),
            None => violations.push(format!(
                "work item identity field {:?} is absent from work_item_schema",
                self.queue.work_item_id_field
            )),
        }
        if self.workflow_packs.is_empty() {
            violations.push("workflow_packs must contain at least one exact reference".to_owned());
        }
        if self.authority.max_actions == 0 {
            violations.push("authority.max_actions must be greater than zero".to_owned());
        }
        if self.authority.max_model_cost_micro_usd == 0 {
            violations
                .push("authority.max_model_cost_micro_usd must be greater than zero".to_owned());
        }
        if !self
            .authority
            .required_approval_risk_classes
            .is_subset(&self.authority.allowed_risk_classes)
        {
            violations.push(
                "required approval risk classes must be within allowed risk classes".to_owned(),
            );
        }

        let mut seen_refs = BTreeSet::new();
        let mut seen_pack_ids = BTreeSet::new();
        let mut used_capabilities = BTreeSet::new();
        let mut used_risk_classes = BTreeSet::new();
        let mut resolved_packs = Vec::new();
        for reference in &self.workflow_packs {
            let key = (reference.pack_id.clone(), reference.version.clone());
            if !seen_refs.insert(key) {
                violations.push(format!(
                    "duplicate workflow pack reference {}@{}",
                    reference.pack_id, reference.version
                ));
                continue;
            }
            if !seen_pack_ids.insert(reference.pack_id.clone()) {
                violations.push(format!(
                    "workflow pack id {:?} is referenced more than once",
                    reference.pack_id
                ));
                continue;
            }
            let Some(pack) = packs.iter().find(|pack| {
                pack.manifest.pack_id == reference.pack_id
                    && pack.manifest.version == reference.version
            }) else {
                if packs
                    .iter()
                    .any(|pack| pack.manifest.pack_id == reference.pack_id)
                {
                    violations.push(format!(
                        "workflow pack {} does not provide required version {}",
                        reference.pack_id, reference.version
                    ));
                } else {
                    violations.push(format!(
                        "workflow pack {}@{} is not available",
                        reference.pack_id, reference.version
                    ));
                }
                continue;
            };
            if let Err(pack_violations) = pack.validate() {
                violations.extend(
                    pack_violations
                        .into_iter()
                        .map(|violation| format!("{}: {violation}", pack.manifest.pack_id)),
                );
            }
            for step in &pack.steps {
                if !step.preconditions.is_empty() {
                    violations.push(format!(
                        "role v0 refuses {}:{} because preconditions are not wired into pack preparation",
                        pack.manifest.pack_id, step.step_id
                    ));
                }
                used_capabilities.insert(step.capability.clone());
                used_risk_classes.insert(step.risk_class);
                if step.risk_class.is_consequential()
                    && !self
                        .authority
                        .required_approval_risk_classes
                        .contains(&step.risk_class)
                {
                    violations.push(format!(
                        "step {}:{} has consequential risk {:?} without a role approval ceiling",
                        pack.manifest.pack_id, step.step_id, step.risk_class
                    ));
                }
                if matches!(step.approval_rule, ApprovalRule::Always)
                    && !self
                        .authority
                        .required_approval_risk_classes
                        .contains(&step.risk_class)
                {
                    violations.push(format!(
                        "step {}:{} requires approval but role authority narrows it",
                        pack.manifest.pack_id, step.step_id
                    ));
                }
            }
            resolved_packs.push(pack);
        }

        if self.authority.allowed_capabilities != used_capabilities {
            let missing: BTreeSet<_> = used_capabilities
                .difference(&self.authority.allowed_capabilities)
                .map(|capability| capability.as_str().to_owned())
                .collect();
            let extra: BTreeSet<_> = self
                .authority
                .allowed_capabilities
                .difference(&used_capabilities)
                .map(|capability| capability.as_str().to_owned())
                .collect();
            if !missing.is_empty() {
                violations.push(format!(
                    "authority.allowed_capabilities omits composed capabilities: {missing:?}"
                ));
            }
            if !extra.is_empty() {
                violations.push(format!(
                    "authority.allowed_capabilities expands beyond composed capabilities: {extra:?}"
                ));
            }
        }
        if self.authority.allowed_risk_classes != used_risk_classes {
            let missing: BTreeSet<_> = used_risk_classes
                .difference(&self.authority.allowed_risk_classes)
                .copied()
                .collect();
            let extra: BTreeSet<_> = self
                .authority
                .allowed_risk_classes
                .difference(&used_risk_classes)
                .copied()
                .collect();
            if !missing.is_empty() {
                violations.push(format!(
                    "authority.allowed_risk_classes omits composed risks: {missing:?}"
                ));
            }
            if !extra.is_empty() {
                violations.push(format!(
                    "authority.allowed_risk_classes expands beyond composed risks: {extra:?}"
                ));
            }
        }

        for pack in resolved_packs {
            validate_budget_ceiling(&mut violations, &self.authority, pack);
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    /// Validates the queue envelope before any composed workflow input is
    /// handed to the existing pack runner.
    ///
    /// # Errors
    /// Returns missing/invalid identity, workflow, type, unknown-field, or
    /// identity-mismatch violations.
    pub fn validate_work_item(&self, item: &RoleWorkItem) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();
        if item.work_item_id.trim().is_empty() {
            violations.push("work_item_id must not be empty".to_owned());
        }
        if !self
            .workflow_packs
            .iter()
            .any(|reference| reference.pack_id == item.workflow_id)
        {
            violations.push(format!(
                "workflow_id {:?} is not composed by this role",
                item.workflow_id
            ));
        }
        let Some(object) = item.inputs.as_object() else {
            violations.push("work item inputs must be a JSON object".to_owned());
            return Err(violations);
        };
        if let Err(schema_violations) = self
            .work_item_schema
            .as_pack_schema()
            .validate(&item.inputs)
        {
            violations.extend(schema_violations);
        }
        let declared: BTreeSet<&str> = self
            .work_item_schema
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect();
        for field in object.keys() {
            if !declared.contains(field.as_str()) {
                violations.push(format!("unknown work item input field {:?}", field));
            }
        }
        match object.get(&self.queue.work_item_id_field) {
            Some(serde_json::Value::String(value)) if value == &item.work_item_id => {}
            Some(serde_json::Value::String(_)) => violations.push(format!(
                "work item identity field {:?} does not match envelope work_item_id",
                self.queue.work_item_id_field
            )),
            Some(_) => violations.push(format!(
                "work item identity field {:?} must be a string",
                self.queue.work_item_id_field
            )),
            None => violations.push(format!(
                "work item identity field {:?} is missing",
                self.queue.work_item_id_field
            )),
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    /// Computes a canonical digest over this role manifest and the exact
    /// referenced workflow-pack contents. Same-version content changes
    /// therefore invalidate prepared runs.
    pub fn content_digest(&self, packs: &[WorkflowPack]) -> Result<String, String> {
        let mut referenced = Vec::with_capacity(self.workflow_packs.len());
        for reference in &self.workflow_packs {
            let pack = packs.iter().find(|pack| {
                pack.manifest.pack_id == reference.pack_id
                    && pack.manifest.version == reference.version
            });
            let Some(pack) = pack else {
                return Err(format!(
                    "workflow pack {}@{} is not available for role digest",
                    reference.pack_id, reference.version
                ));
            };
            referenced.push(serde_json::to_value(pack).map_err(|error| error.to_string())?);
        }
        let role = serde_json::to_value(self).map_err(|error| error.to_string())?;
        Ok(sha256_canonical(&serde_json::json!({
            "role": role,
            "workflow_packs": referenced,
        })))
    }

    /// Intersects the role ceiling with one pack's budget.  This is the
    /// effective budget that a resolved role run must pass to the executor.
    #[must_use]
    pub fn effective_budget(&self, pack: &WorkflowPack) -> Budget {
        let pack_budget = &pack.manifest.default_budget;
        Budget {
            deadline: pack_budget.deadline,
            max_model_cost_micro_usd: Some(min_optional_u64(
                self.authority.max_model_cost_micro_usd,
                pack_budget.max_model_cost_micro_usd,
            )),
            max_actions: Some(min_optional_u32(
                self.authority.max_actions,
                pack_budget.max_actions,
            )),
            max_retries: Some(min_optional_u32(
                self.authority.max_retries,
                pack_budget.max_retries,
            )),
            max_vision_actions: Some(min_optional_u32(
                self.authority.max_vision_actions,
                pack_budget.max_vision_actions,
            )),
            max_external_writes: Some(min_optional_u32(
                self.authority.max_external_writes,
                pack_budget.max_external_writes,
            )),
        }
    }

    /// Checks a prepared action against the role's capability and risk
    /// envelope.  This is a role ceiling check, not policy authorization;
    /// the orchestrator must still run local policy afterwards.
    ///
    /// # Errors
    /// Returns a role-authority violation when the action would escape the
    /// role's declared composition.
    pub fn check_action_ceiling(&self, action: &ActionProposal) -> Result<(), String> {
        if !self
            .authority
            .allowed_capabilities
            .contains(&action.capability)
        {
            return Err(format!(
                "capability {} is outside role {} authority",
                action.capability.as_str(),
                self.role_id
            ));
        }
        if !self
            .authority
            .allowed_risk_classes
            .contains(&action.risk_class)
        {
            return Err(format!(
                "risk class {:?} is outside role {} authority",
                action.risk_class, self.role_id
            ));
        }
        if action.risk_class.is_consequential()
            && !self
                .authority
                .required_approval_risk_classes
                .contains(&action.risk_class)
        {
            return Err(format!(
                "consequential risk {:?} is not approval-bound by role {}",
                action.risk_class, self.role_id
            ));
        }
        Ok(())
    }

    /// Whether a prepared action of this risk class must remain approval
    /// gated at the role boundary.  Local policy remains authoritative.
    #[must_use]
    pub fn requires_approval(&self, risk_class: RiskClass) -> bool {
        self.authority
            .required_approval_risk_classes
            .contains(&risk_class)
    }
}

fn validate_budget_ceiling(
    violations: &mut Vec<String>,
    authority: &RoleAuthorityCeiling,
    pack: &WorkflowPack,
) {
    let declared = &pack.manifest.default_budget;
    check_u64(
        violations,
        &format!("{} max_model_cost_micro_usd", pack.manifest.pack_id),
        authority.max_model_cost_micro_usd,
        declared.max_model_cost_micro_usd,
    );
    check_u32(
        violations,
        &format!("{} max_actions", pack.manifest.pack_id),
        authority.max_actions,
        declared.max_actions,
    );
    check_u32(
        violations,
        &format!("{} max_retries", pack.manifest.pack_id),
        authority.max_retries,
        declared.max_retries,
    );
    check_u32(
        violations,
        &format!("{} max_vision_actions", pack.manifest.pack_id),
        authority.max_vision_actions,
        declared.max_vision_actions,
    );
    check_u32(
        violations,
        &format!("{} max_external_writes", pack.manifest.pack_id),
        authority.max_external_writes,
        declared.max_external_writes,
    );
}

fn check_u32(violations: &mut Vec<String>, field: &str, role: u32, pack: Option<u32>) {
    if let Some(pack) = pack {
        if role > pack {
            violations.push(format!(
                "role authority {field}={role} widens composed pack ceiling {pack}"
            ));
        }
    }
}

fn check_u64(violations: &mut Vec<String>, field: &str, role: u64, pack: Option<u64>) {
    if let Some(pack) = pack {
        if role > pack {
            violations.push(format!(
                "role authority {field}={role} widens composed pack ceiling {pack}"
            ));
        }
    }
}

fn min_optional_u32(role: u32, pack: Option<u32>) -> u32 {
    pack.map_or(role, |pack| role.min(pack))
}

fn min_optional_u64(role: u64, pack: Option<u64>) -> u64 {
    pack.map_or(role, |pack| role.min(pack))
}

fn is_semver(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u64>().is_ok())
}
