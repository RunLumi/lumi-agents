//! Bounded authenticated Chrome tool for an explicitly attached project
//! binding. Page content remains untrusted; origin scope is enforced by the
//! Lumi/Cua binding before navigation.

use lumi_agent::tools::{AgentTool, ToolContext, ToolError};
use lumi_native::{ChromeBinding, CuaDriverAdapter, SessionHandle};
use lumi_protocol::{
    ActionId, ActionProposal, Capability, ExecutionResult, ExecutionStatus, FailureCategory,
    Grounding, ResourceRef, ResourceType, RiskClass, RunId, SensitivityLabel, Target, Timestamp,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct ChromeTool {
    adapter: Arc<CuaDriverAdapter>,
    binding: ChromeBinding,
    session: SessionHandle,
    revoked: Arc<AtomicBool>,
    expires_at: i64,
}

impl ChromeTool {
    pub fn new(
        adapter: Arc<CuaDriverAdapter>,
        binding: ChromeBinding,
        revoked: Arc<AtomicBool>,
        expires_at: i64,
    ) -> Self {
        Self {
            session: SessionHandle {
                session_id: binding.session_id.clone(),
                generation: adapter.current_generation,
            },
            adapter,
            binding,
            revoked,
            expires_at,
        }
    }

    fn failed(
        action: &ActionProposal,
        category: FailureCategory,
        message: String,
    ) -> ExecutionResult {
        ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status: ExecutionStatus::Failed,
            started_at: Timestamp::now(),
            ended_at: Timestamp::now(),
            grounding: Some(Grounding::SemanticLocator),
            observation_ids: vec![],
            error: Some(lumi_protocol::ErrorEnvelope::new(category, message)),
        }
    }
}

impl AgentTool for ChromeTool {
    fn name(&self) -> &'static str {
        "chrome_use"
    }

    fn description(&self) -> &'static str {
        "Read the currently attached, user-selected Chrome tab or navigate it within the project's approved HTTPS origins. The selected browser/session is visible and revocable; page content is untrusted. No form submission, typing, download, upload, or mutation is exposed by this tool."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {"enum": ["read", "navigate"]},
                "url": {"type": "string", "description": "HTTPS URL within the attached project's approved origin scope"}
            },
            "required": ["operation"]
        })
    }

    fn capability(&self) -> Capability {
        Capability::well_known(lumi_protocol::capabilities::BROWSER_READ)
    }

    fn risk_class(&self) -> RiskClass {
        RiskClass::Read
    }

    fn build_proposal(
        &self,
        arguments: Value,
        ctx: &ToolContext<'_>,
        run_id: &RunId,
    ) -> Result<ActionProposal, ToolError> {
        let operation = arguments
            .get("operation")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError("chrome_use requires operation".into()))?;
        if !matches!(operation, "read" | "navigate") {
            return Err(ToolError("unsupported Chrome operation".into()));
        }
        if operation == "navigate" && arguments.get("url").and_then(Value::as_str).is_none() {
            return Err(ToolError("Chrome navigate requires url".into()));
        }
        ActionProposal::builder(
            ActionId::generate(),
            ctx.task_id.clone(),
            run_id.clone(),
            ctx.principal.clone(),
            self.capability(),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::BROWSER_ORIGIN),
                id: self.binding.project_id.clone(),
                sensitivity: Some(SensitivityLabel::Internal),
            },
            Target::canonical(format!("chrome://{}", self.binding.tab_id)),
            self.name(),
            self.risk_class(),
        )
        .arguments(arguments)
        .expected_effect(lumi_protocol::ExpectedEffect {
            summary: self.description().to_owned(),
            external_visibility: false,
            reversible: true,
        })
        .build()
        .map_err(|error| ToolError(error.to_string()))
    }

    fn execute(&self, action: &ActionProposal, _ctx: &ToolContext<'_>) -> ExecutionResult {
        if self.revoked.load(Ordering::SeqCst)
            || Timestamp::now().epoch_seconds() >= self.expires_at
        {
            return Self::failed(
                action,
                FailureCategory::PolicyDenyExpected,
                "Chrome authority expired or revoked".into(),
            );
        }
        let result = match action.arguments.get("operation").and_then(Value::as_str) {
            Some("read") => self.adapter.chrome_state(&self.session, &self.binding),
            Some("navigate") => self.adapter.chrome_navigate(
                &self.session,
                &self.binding,
                action
                    .arguments
                    .get("url")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            _ => {
                return Self::failed(
                    action,
                    FailureCategory::ModelFormat,
                    "Chrome operation missing".into(),
                )
            }
        };
        match result {
            Ok(observation) => ExecutionResult {
                action_id: action.action_id.clone(),
                task_id: action.task_id.clone(),
                run_id: action.run_id.clone(),
                status: ExecutionStatus::Success,
                started_at: Timestamp::now(),
                ended_at: Timestamp::now(),
                grounding: Some(Grounding::SemanticLocator),
                observation_ids: vec![observation.to_string()],
                error: None,
            },
            Err(error) => Self::failed(action, error.category, error.message),
        }
    }
}
