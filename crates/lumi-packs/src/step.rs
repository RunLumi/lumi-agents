//! Pack steps and the pack aggregate (spec 12 §12.6–12.7).

use crate::manifest::{PackManifest, SchemaFieldSet};
use lumi_protocol::{
    Capability, EvidenceRequirement, ExecutionTier, FailureCategory, Idempotency, Postcondition,
    RiskClass,
};
use serde::{Deserialize, Serialize};

/// Who/what handles an exception (§12.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "route")]
pub enum ExceptionTarget {
    /// Human exception queue (task owner).
    User,
    /// Manager/approver role.
    Approver,
    /// Specialist queue (e.g. ops).
    Specialist,
    /// A named alternate executor/adapter.
    AlternateExecutor { adapter: String },
    /// Retry within policy, then escalate.
    Retry,
    /// Controlled stop.
    Stop,
}

/// The exception route for a step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExceptionRoute {
    /// Canonical failure categories that trigger this route.
    pub triggers: Vec<FailureCategory>,
    pub target: ExceptionTarget,
    /// Human-readable instruction for the exception handler.
    pub guidance: String,
}

/// When a step demands approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRule {
    /// Always require scoped approval before this step.
    Always,
    /// Defer to policy evaluation (default): policy decides from risk
    /// class and pre-authorizations.
    PerPolicy,
}

/// One step of the pack's routine path (§12.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackStep {
    pub step_id: String,
    /// Business purpose, shown in UX/evidence.
    pub purpose: String,
    pub capability: Capability,
    /// Operation name carried on the generated ActionProposal.
    pub operation: String,
    /// Argument template: JSON object with `$input.<path>` string leaves
    /// resolved from validated run inputs (§12.13: no customer data in
    /// the pack itself).
    pub arguments_template: serde_json::Value,
    pub preferred_tier: ExecutionTier,
    /// Tiers the router may fall back to.
    pub allowed_fallback_tiers: Vec<ExecutionTier>,
    pub risk_class: RiskClass,
    pub approval_rule: ApprovalRule,
    /// Pre-execution checks (same shape as postconditions).
    #[serde(default)]
    pub preconditions: Vec<Postcondition>,
    pub postconditions: Vec<Postcondition>,
    /// Retry classification + idempotency metadata for this step.
    pub idempotency: Idempotency,
    pub evidence: Vec<EvidenceRequirement>,
    /// Resource type/id template (`$input.` references allowed in id).
    pub resource_type: String,
    pub resource_id_template: String,
    /// Canonical target template.
    pub target_template: String,
    pub exception_route: ExceptionRoute,
}

/// A versioned workflow pack (§12.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowPack {
    pub manifest: PackManifest,
    /// Routine path, in execution order. v1 executes linearly; graphs
    /// (branch/join) extend this without schema breakage by adding an
    /// optional routing section.
    pub steps: Vec<PackStep>,
}

impl WorkflowPack {
    /// Validates the pack's structural invariants.
    ///
    /// # Errors
    /// Violations list: unique step ids, every step fully specified,
    /// fallback tiers include the preferred tier, exception routes exist,
    /// manifest valid.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut violations = match self.manifest.validate() {
            Ok(()) => Vec::new(),
            Err(mut v) => {
                v.insert(0, "manifest invalid".to_owned());
                v
            }
        };
        if self.steps.is_empty() {
            violations.push("pack must define at least one step".to_owned());
        }
        let mut seen = std::collections::BTreeSet::new();
        for step in &self.steps {
            if step.step_id.trim().is_empty() {
                violations.push("step_id must not be empty".to_owned());
            }
            if !seen.insert(step.step_id.clone()) {
                violations.push(format!("duplicate step_id {:?}", step.step_id));
            }
            if step.purpose.trim().is_empty() {
                violations.push(format!("step {:?} needs a purpose", step.step_id));
            }
            if !step.allowed_fallback_tiers.contains(&step.preferred_tier) {
                violations.push(format!(
                    "step {:?}: preferred tier must be among allowed fallback tiers",
                    step.step_id
                ));
            }
            if step.postconditions.is_empty() && step.risk_class.is_consequential() {
                violations.push(format!(
                    "step {:?}: consequential steps must declare postconditions",
                    step.step_id
                ));
            }
            if step.exception_route.triggers.is_empty() {
                violations.push(format!(
                    "step {:?}: exception route must list triggers",
                    step.step_id
                ));
            }
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    /// The declared output schema, for post-run validation.
    #[must_use]
    pub const fn output_schema(&self) -> &SchemaFieldSet {
        &self.manifest.outputs
    }
}
