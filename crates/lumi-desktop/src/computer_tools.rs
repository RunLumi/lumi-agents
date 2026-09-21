//! Lumi-owned agent tool for bounded native computer actions.
//!
//! The model proposes semantic application/window/control targets. The Cua
//! adapter resolves snapshot-bound native elements and independently reads
//! back the effect; upstream tool names never enter the workflow contract.

use lumi_agent::tools::{AgentTool, ToolContext, ToolError};
use lumi_native::{
    CuaDriverAdapter, DesktopDriver, NativeOperation, NativeOutcome, SemanticTarget, SessionHandle,
};
use lumi_protocol::{
    ActionId, ActionProposal, Capability, ExecutionResult, ExecutionStatus, FailureCategory,
    Grounding, ResourceRef, ResourceType, RiskClass, RunId, SensitivityLabel, Target, Timestamp,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct ComputerTool {
    adapter: Arc<CuaDriverAdapter>,
    session: SessionHandle,
    revoked: Arc<AtomicBool>,
    expires_at: i64,
}

impl ComputerTool {
    pub fn new(
        adapter: Arc<CuaDriverAdapter>,
        session: SessionHandle,
        revoked: Arc<AtomicBool>,
        expires_at: i64,
    ) -> Self {
        Self {
            adapter,
            session,
            revoked,
            expires_at,
        }
    }

    fn target(arguments: &Value) -> Result<SemanticTarget, ToolError> {
        let application = arguments
            .get("application")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ToolError("computer_use requires application".into()))?;
        Ok(SemanticTarget {
            application: application.to_owned(),
            window: arguments
                .get("window")
                .and_then(Value::as_str)
                .map(str::to_owned),
            role: arguments
                .get("role")
                .and_then(Value::as_str)
                .map(str::to_owned),
            name: arguments
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_owned),
            stable_property: arguments
                .get("stable_property")
                .and_then(Value::as_object)
                .and_then(|property| {
                    Some((
                        property.get("key")?.as_str()?.to_owned(),
                        property.get("value")?.as_str()?.to_owned(),
                    ))
                }),
        })
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
            grounding: Some(Grounding::SemanticTarget),
            observation_ids: vec![],
            error: Some(lumi_protocol::ErrorEnvelope::new(category, message)),
        }
    }
}

impl AgentTool for ComputerTool {
    fn name(&self) -> &'static str {
        "computer_use"
    }

    fn description(&self) -> &'static str {
        "Read or set one semantic value in an explicitly named native application and window. Only operations with independent native readback are exposed. No shell fallback, arbitrary coordinates, clicks, keypresses, or hidden foreground escalation."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {"enum": ["read_value", "set_value"]},
                "application": {"type": "string"},
                "window": {"type": "string"},
                "role": {"type": "string"},
                "name": {"type": "string"},
                "stable_property": {"type": "object", "properties": {"key":{"type":"string"},"value":{"type":"string"}}, "required":["key","value"]},
                "value": {"type": "string"},
                "action": {"enum": ["confirm", "cancel", "return", "escape", "space"]}
            },
            "required": ["operation", "application"]
        })
    }

    fn capability(&self) -> Capability {
        Capability::well_known(lumi_protocol::capabilities::DESKTOP_INTERACT)
    }

    fn risk_class(&self) -> RiskClass {
        RiskClass::LocalWrite
    }

    fn build_proposal(
        &self,
        arguments: Value,
        ctx: &ToolContext<'_>,
        run_id: &RunId,
    ) -> Result<ActionProposal, ToolError> {
        if !arguments.is_object() {
            return Err(ToolError("computer_use arguments must be an object".into()));
        }
        let operation = arguments
            .get("operation")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError("computer_use requires operation".into()))?;
        if operation == "set_value" && arguments.get("value").and_then(Value::as_str).is_none() {
            return Err(ToolError("set_value requires value".into()));
        }
        let target = Self::target(&arguments)?;
        ActionProposal::builder(
            ActionId::generate(),
            ctx.task_id.clone(),
            run_id.clone(),
            ctx.principal.clone(),
            self.capability(),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::LOCAL_APP),
                id: target.application.clone(),
                sensitivity: Some(SensitivityLabel::Internal),
            },
            Target::canonical(target.context_key()),
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
                "computer authority expired or revoked".into(),
            );
        }
        let target = match Self::target(&action.arguments) {
            Ok(target) => target,
            Err(error) => {
                return Self::failed(action, FailureCategory::ModelFormat, error.to_string())
            }
        };
        let operation = match action.arguments.get("operation").and_then(Value::as_str) {
            Some("read_value") => NativeOperation::ReadValue { target },
            Some("set_value") => NativeOperation::SetValue {
                target,
                value: action.arguments["value"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            },
            Some(other) => {
                return Self::failed(
                    action,
                    FailureCategory::ModelFormat,
                    format!("unsupported computer operation {other}"),
                )
            }
            None => {
                return Self::failed(
                    action,
                    FailureCategory::ModelFormat,
                    "operation missing".into(),
                )
            }
        };
        let result = self.adapter.execute(&self.session, operation, &|| {
            Some("native state readback requested".into())
        });
        match result {
            Ok(result) => {
                let status = match result.outcome {
                    NativeOutcome::Delivered => ExecutionStatus::Success,
                    NativeOutcome::Cancelled => ExecutionStatus::Cancelled,
                    NativeOutcome::Ambiguous => ExecutionStatus::Ambiguous,
                    _ => ExecutionStatus::Failed,
                };
                ExecutionResult {
                    action_id: action.action_id.clone(),
                    task_id: action.task_id.clone(),
                    run_id: action.run_id.clone(),
                    status,
                    started_at: Timestamp::now(),
                    ended_at: Timestamp::now(),
                    grounding: Some(Grounding::SemanticTarget),
                    observation_ids: result.oracle_observation.into_iter().collect(),
                    error: result
                        .failure
                        .map(|failure| {
                            lumi_protocol::ErrorEnvelope::new(failure, "native action failed")
                        })
                        .or_else(|| match result.outcome {
                            NativeOutcome::NoEffect => Some(lumi_protocol::ErrorEnvelope::new(
                                FailureCategory::Postcondition,
                                "native readback did not observe the requested effect",
                            )),
                            NativeOutcome::Ambiguous => Some(lumi_protocol::ErrorEnvelope::new(
                                FailureCategory::AmbiguousState,
                                "native effect state is unknown",
                            )),
                            NativeOutcome::Refused => Some(lumi_protocol::ErrorEnvelope::new(
                                FailureCategory::PolicyDenyExpected,
                                "native action was refused before mutation",
                            )),
                            _ => None,
                        }),
                }
            }
            Err(error) => Self::failed(action, error.category, error.message),
        }
    }
}
