//! Agent tools: Lumi-owned capabilities the model may *propose*. Each
//! tool deterministically builds its own ActionProposal — the model only
//! supplies arguments, which the tool validates. Execution goes through
//! the orchestrator gate like every other action.

use lumi_protocol::{
    ActionId, ActionProposal, AuthenticationStrength, Capability, ExecutionResult, ExecutionStatus,
    FailureCategory, Principal, PrincipalKind, ResourceRef, ResourceType, RiskClass, Target,
    TaskId, Timestamp,
};
use lumi_workspaces::FileOp;
use serde_json::json;
use std::path::PathBuf;

/// Errors while mapping model arguments to a proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError(pub String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ToolError {}

/// Context handed to tools for proposal construction and execution.
pub struct ToolContext<'a> {
    pub workspace: &'a lumi_workspaces::Workspace,
    pub task_id: &'a TaskId,
    pub principal: &'a Principal,
}

/// A tool the model may propose.
pub trait AgentTool {
    /// Tool name the model references in tool calls.
    fn name(&self) -> &'static str;
    /// Model-facing description (system prompt material).
    fn description(&self) -> &'static str;
    /// JSON schema for the arguments object.
    fn parameters(&self) -> serde_json::Value;
    /// Capability the resulting action requires.
    fn capability(&self) -> Capability;
    /// Risk class of the resulting action.
    fn risk_class(&self) -> RiskClass;
    /// Builds the normalized proposal from model arguments.
    ///
    /// # Errors
    /// [`ToolError`] when arguments are malformed — a MODEL_FORMAT
    /// problem, never executed.
    fn build_proposal(
        &self,
        arguments: serde_json::Value,
        ctx: &ToolContext<'_>,
        run_id: &lumi_protocol::RunId,
    ) -> Result<ActionProposal, ToolError>;
    /// Executes the authorized action (called only through the gate).
    fn execute(&self, action: &ActionProposal, ctx: &ToolContext<'_>) -> ExecutionResult;
}

fn base_proposal(
    tool: &dyn AgentTool,
    arguments: serde_json::Value,
    ctx: &ToolContext<'_>,
    run_id: &lumi_protocol::RunId,
) -> Result<ActionProposal, ToolError> {
    if !arguments.is_object() {
        return Err(ToolError("tool arguments must be a JSON object".to_owned()));
    }
    ActionProposal::builder(
        ActionId::generate(),
        ctx.task_id.clone(),
        run_id.clone(),
        ctx.principal.clone(),
        tool.capability(),
        ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::FILE),
            id: format!("workspaces/{}", ctx.workspace.metadata.task_id),
            sensitivity: Some(lumi_protocol::SensitivityLabel::Internal),
        },
        Target::canonical(format!("file://{}", ctx.workspace.root().display())),
        tool.name(),
        tool.risk_class(),
    )
    .arguments(arguments)
    .expected_effect(lumi_protocol::ExpectedEffect {
        summary: tool.description().to_owned(),
        external_visibility: tool.risk_class().is_consequential(),
        reversible: !matches!(
            tool.risk_class(),
            RiskClass::Destructive | RiskClass::ExternalWrite
        ),
    })
    .build()
    .map_err(|e| ToolError(e.to_string()))
}

fn err_result(action: &ActionProposal, message: String) -> ExecutionResult {
    ExecutionResult {
        action_id: action.action_id.clone(),
        task_id: action.task_id.clone(),
        run_id: action.run_id.clone(),
        status: ExecutionStatus::Failed,
        started_at: Timestamp::now(),
        ended_at: Timestamp::now(),
        grounding: Some(lumi_protocol::Grounding::DeterministicApi),
        observation_ids: vec![],
        error: Some(lumi_protocol::ErrorEnvelope::new(
            FailureCategory::Filesystem,
            message,
        )),
    }
}

// ---------------------------------------------------------------------------
// Built-in tools
// ---------------------------------------------------------------------------

/// `write_file`: writes (creates) a file inside the task workspace.
/// Overwriting an existing file is refused here — a second write must be
/// an explicit policy-approved step, not an incidental model choice.
#[derive(Debug, Clone, Copy, Default)]
pub struct WriteFileTool;

impl AgentTool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Create a new file inside the task workspace with the given text content."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Workspace-relative path"},
                "content": {"type": "string", "description": "Full text content"}
            },
            "required": ["path", "content"]
        })
    }

    fn capability(&self) -> Capability {
        Capability::well_known(lumi_protocol::capabilities::FILES_CREATE)
    }

    fn risk_class(&self) -> RiskClass {
        RiskClass::LocalWrite
    }

    fn build_proposal(
        &self,
        arguments: serde_json::Value,
        ctx: &ToolContext<'_>,
        run_id: &lumi_protocol::RunId,
    ) -> Result<ActionProposal, ToolError> {
        for key in ["path", "content"] {
            if arguments.get(key).and_then(|v| v.as_str()).is_none() {
                return Err(ToolError(format!("write_file requires string {key:?}")));
            }
        }
        base_proposal(self, arguments, ctx, run_id)
    }

    fn execute(&self, action: &ActionProposal, ctx: &ToolContext<'_>) -> ExecutionResult {
        let files = lumi_workspaces::WorkspaceFiles::new(ctx.workspace);
        let op = FileOp::Create {
            path: PathBuf::from(action.arguments["path"].as_str().unwrap_or_default()),
            content: action.arguments["content"]
                .as_str()
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
            allow_overwrite: false,
        };
        match files.execute(&op) {
            Ok(record) => ExecutionResult {
                action_id: action.action_id.clone(),
                task_id: action.task_id.clone(),
                run_id: action.run_id.clone(),
                status: ExecutionStatus::Success,
                started_at: Timestamp::now(),
                ended_at: Timestamp::now(),
                grounding: Some(lumi_protocol::Grounding::DeterministicApi),
                observation_ids: vec![record.sha256_after.unwrap_or_default()],
                error: None,
            },
            Err(e) => err_result(action, e.to_string()),
        }
    }
}

/// `read_file`: reads a workspace file into the observation.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReadFileTool;

impl AgentTool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read a text file from the task workspace."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Workspace-relative path"}
            },
            "required": ["path"]
        })
    }

    fn capability(&self) -> Capability {
        Capability::well_known(lumi_protocol::capabilities::FILES_READ)
    }

    fn risk_class(&self) -> RiskClass {
        RiskClass::Read
    }

    fn build_proposal(
        &self,
        arguments: serde_json::Value,
        ctx: &ToolContext<'_>,
        run_id: &lumi_protocol::RunId,
    ) -> Result<ActionProposal, ToolError> {
        if arguments.get("path").and_then(|v| v.as_str()).is_none() {
            return Err(ToolError("read_file requires string \"path\"".to_owned()));
        }
        base_proposal(self, arguments, ctx, run_id)
    }

    fn execute(&self, action: &ActionProposal, ctx: &ToolContext<'_>) -> ExecutionResult {
        let files = lumi_workspaces::WorkspaceFiles::new(ctx.workspace);
        match files.execute(&FileOp::Read {
            path: PathBuf::from(action.arguments["path"].as_str().unwrap_or_default()),
        }) {
            Ok(record) => {
                let content = std::fs::read_to_string(ctx.workspace.root().join(&record.path))
                    .unwrap_or_default();
                ExecutionResult {
                    action_id: action.action_id.clone(),
                    task_id: action.task_id.clone(),
                    run_id: action.run_id.clone(),
                    status: ExecutionStatus::Success,
                    started_at: Timestamp::now(),
                    ended_at: Timestamp::now(),
                    grounding: Some(lumi_protocol::Grounding::DeterministicApi),
                    observation_ids: vec![content],
                    error: None,
                }
            }
            Err(e) => err_result(action, e.to_string()),
        }
    }
}

/// `run_shell`: bounded shell execution inside the workspace.
#[derive(Debug, Clone, Copy, Default)]
pub struct RunShellTool;

impl AgentTool for RunShellTool {
    fn name(&self) -> &'static str {
        "run_shell"
    }

    fn description(&self) -> &'static str {
        "Run a short command inside the task workspace (no network, no inherited environment)."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "Program to run"},
                "args": {"type": "array", "items": {"type": "string"}}
            },
            "required": ["command"]
        })
    }

    fn capability(&self) -> Capability {
        Capability::well_known(lumi_protocol::capabilities::SHELL_EXECUTE)
    }

    fn risk_class(&self) -> RiskClass {
        RiskClass::LocalWrite
    }

    fn build_proposal(
        &self,
        arguments: serde_json::Value,
        ctx: &ToolContext<'_>,
        run_id: &lumi_protocol::RunId,
    ) -> Result<ActionProposal, ToolError> {
        if arguments.get("command").and_then(|v| v.as_str()).is_none() {
            return Err(ToolError(
                "run_shell requires string \"command\"".to_owned(),
            ));
        }
        base_proposal(self, arguments, ctx, run_id)
    }

    fn execute(&self, action: &ActionProposal, ctx: &ToolContext<'_>) -> ExecutionResult {
        let sandbox = lumi_workspaces::ShellSandbox::new(ctx.workspace);
        let spec = lumi_workspaces::ShellSpec {
            program: action.arguments["command"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            args: action.arguments["args"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|i| i.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            ..lumi_workspaces::ShellSpec::default()
        };
        let outcome = sandbox.run(&spec);
        let (status, error) = match outcome.status {
            lumi_workspaces::ShellStatus::Completed => (ExecutionStatus::Success, None),
            lumi_workspaces::ShellStatus::Failed(code) => (
                ExecutionStatus::Failed,
                Some(lumi_protocol::ErrorEnvelope::new(
                    FailureCategory::ShellExecution,
                    format!("exit code {code}"),
                )),
            ),
            lumi_workspaces::ShellStatus::TimedOut => (
                ExecutionStatus::Failed,
                Some(lumi_protocol::ErrorEnvelope::new(
                    FailureCategory::ShellExecution,
                    "command timed out",
                )),
            ),
            lumi_workspaces::ShellStatus::Cancelled => (ExecutionStatus::Cancelled, None),
            lumi_workspaces::ShellStatus::SpawnError => (
                ExecutionStatus::Failed,
                Some(lumi_protocol::ErrorEnvelope::new(
                    FailureCategory::ShellExecution,
                    outcome.stderr.clone(),
                )),
            ),
        };
        ExecutionResult {
            action_id: action.action_id.clone(),
            task_id: action.task_id.clone(),
            run_id: action.run_id.clone(),
            status,
            started_at: Timestamp::now(),
            ended_at: Timestamp::now(),
            grounding: Some(lumi_protocol::Grounding::DeterministicApi),
            observation_ids: vec![format!(
                "exit:{:?} stdout:{}",
                outcome.status, outcome.stdout
            )],
            error,
        }
    }
}

/// Convenience: the default tool set for Work mode.
#[must_use]
pub fn default_tools() -> Vec<Box<dyn AgentTool>> {
    vec![
        Box::new(WriteFileTool),
        Box::new(ReadFileTool),
        Box::new(RunShellTool),
    ]
}

/// Authentication strength helper for runner principals.
#[must_use]
pub fn workflow_principal(tenant: &str, id: &str) -> Principal {
    Principal {
        principal_id: lumi_protocol::PrincipalId::parse(id)
            .unwrap_or_else(|_| lumi_protocol::PrincipalId::generate()),
        tenant_id: lumi_protocol::TenantId::parse(tenant)
            .unwrap_or_else(|_| lumi_protocol::TenantId::generate()),
        kind: PrincipalKind::Workflow,
        authenticated_at: Some(Timestamp::now()),
        authentication_strength: Some(AuthenticationStrength::DevicePossession),
    }
}
