//! Desktop task runner: executes one persisted Work-mode task against
//! its project workspace through the real planning loop and orchestrator
//! gate.
//!
//! This is the production wiring the shell calls: the model path is a
//! live [`ModelPlanner`] over the user's configured provider session
//! (endpoint + model + credential resolved at the transport boundary —
//! the key is a header built per call, never logged or persisted), the
//! tools are the bounded Work-mode set, and every proposed action passes
//! the same policy → approval → journal → verification → audit gate as
//! every other Lumi surface. Tests inject a [`FixturePlanner`-shaped]
//! planner instead of a provider; the gate, tools, durability, and
//! verification are the real ones in both paths.

use crate::runtime::DesktopRuntime;
use lumi_agent::runner::run_persisted_task;
use lumi_agent::{ModelPlanner, Planner, ReadFileTool, RunShellTool, WriteFileTool};
use lumi_models::adapters::{OpenAICompatibleDriver, OpenAIDriver};
use lumi_models::instance::{AuthHeaders, ProviderInstance};
use lumi_models::transport::HttpTransport;
use lumi_orchestrator::ExecutorDescriptor;
use lumi_policy::{CapabilityGrant, GrantSource};
use lumi_protocol::{
    Capability, ExecutionStatus, ExecutionTier, ResourceType, RunId, Task, TaskStatus,
    WorkspaceKind,
};
use lumi_workspaces::Workspace;
use std::collections::BTreeSet;

/// The provider session the user configured for this app session. The
/// credential lives in memory only — never persisted, never logged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSession {
    /// `"openai"` or `"openai-compatible"` for any compatible endpoint.
    pub family: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
}

/// Statuses a (re)run may start from. `RUNNING` is excluded on purpose:
/// a RUNNING record belongs to a live or crashed prior run, and blind
/// reruns are exactly the duplicate-execution hazard the orchestrator's
/// replay guard exists for.
#[must_use]
pub fn is_runnable(status: &TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Created | TaskStatus::Failed | TaskStatus::Cancelled
    )
}

/// Owned provider session pieces with the lifetimes a planner needs.
pub struct ProviderParts {
    pub driver: Box<dyn lumi_models::ProviderDriver>,
    pub instance: ProviderInstance,
    pub auth: AuthHeaders,
}

/// Builds the owned provider pieces for the configured session.
///
/// # Errors
/// Unknown driver family.
pub fn provider_parts(session: &ProviderSession) -> Result<ProviderParts, String> {
    let driver: Box<dyn lumi_models::ProviderDriver> = match session.family.as_str() {
        "openai" => Box::new(OpenAIDriver::new()),
        "openai-compatible" => Box::new(OpenAICompatibleDriver::new()),
        other => return Err(format!("unknown provider family {other:?}")),
    };
    let instance = ProviderInstance::new(
        "desktop-session",
        "openai-compatible",
        "desktop provider session",
        session.endpoint.clone(),
        lumi_protocol::SecretRef(session.api_key.clone()),
        session.family.clone(),
    );
    let auth: AuthHeaders = vec![(
        "Authorization".to_owned(),
        format!("Bearer {}", session.api_key),
    )];
    Ok(ProviderParts {
        driver,
        instance,
        auth,
    })
}

/// Builds a durable progress summary of the task's prior VERIFIED
/// actions from the audit ledger, so a re-run of an interrupted or
/// failed task continues instead of redoing work. Failed and ambiguous
/// prior actions are deliberately omitted: they are not progress.
/// `None` when the task has no verified action history.
fn resume_context_from_ledger(
    orchestrator: &lumi_orchestrator::Orchestrator<lumi_state::JsonStateStore>,
    task: &Task,
) -> Option<String> {
    let events = orchestrator.audit.events_for(&task.tenant_id);
    let mut lines: Vec<String> = events
        .iter()
        .filter(|event| event.task_id == task.task_id)
        .filter_map(|event| {
            let lumi_audit::AuditEventKind::Action(details) = &event.kind else {
                return None;
            };
            let verified = matches!(
                (&details.execution_status, &details.verification),
                (
                    Some(ExecutionStatus::Success),
                    lumi_audit::VerificationStatus::Passed
                ) | (
                    Some(ExecutionStatus::Success),
                    lumi_audit::VerificationStatus::NotRequired
                ),
            );
            verified.then(|| {
                format!(
                    "- {} on {} (verified)",
                    details.operation, details.target.canonical
                )
            })
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    lines.sort();
    lines.dedup();
    Some(format!(
        "DURABLE PROGRESS FROM THE PREVIOUS ATTEMPT (recorded by the runtime;          these verified actions must not be redone, and their files already \
        exist — extend the work instead of rewriting it):\n{}",
        lines.join("\n")
    ))
}

/// Wires the runtime's deny-by-default orchestrator with the narrow,
/// project-scoped policy this Work-mode run needs, attaches the task's
/// project root as an in-memory workspace (the user's repository is not
/// written by the binding), and executes the persisted task through the
/// real gate.
///
/// # Errors
/// Workspace attach or durable store failures. Task-level failures come
/// back inside the [`lumi_agent::runner::TaskRunOutcome`].
pub fn run_desktop_task(
    runtime: &mut DesktopRuntime,
    task: &Task,
    planner: &dyn Planner,
) -> Result<lumi_agent::runner::TaskRunOutcome, String> {
    if !is_runnable(&task.status) {
        return Err(format!(
            "task status {:?} cannot be run right now (a RUNNING record belongs to a live or crashed prior run, not a rerun)",
            task.status
        ));
    }
    let binding = task
        .project_binding
        .clone()
        .ok_or("task has no project binding; only project-bound tasks run locally")?;

    // Narrow, project-scoped authority for this run. The shell ships
    // deny-by-default with NO grants; the local desktop device is the
    // user's authority, so the host widens exactly this much before
    // execution and no more (no destructive, external-write, or
    // communication capability is granted here).
    let grant = |id: &str, capability: &'static str| CapabilityGrant {
        grant_id: id.to_owned(),
        tenant_id: task.tenant_id.clone(),
        principal_id: None,
        capability: Capability::well_known(capability),
        resource_scope: lumi_policy::ResourceScope::all_of([ResourceType::well_known(
            ResourceType::FILE,
        )]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: GrantSource::OrganizationPolicy,
    };
    runtime.orchestrator.config.registry = lumi_policy::CapabilityRegistry::default()
        .grant(grant(
            "g-files-read",
            lumi_protocol::capabilities::FILES_READ,
        ))
        .grant(grant(
            "g-files-create",
            lumi_protocol::capabilities::FILES_CREATE,
        ))
        .grant(grant("g-shell", lumi_protocol::capabilities::SHELL_EXECUTE));
    runtime.orchestrator.config.executors = vec![ExecutorDescriptor::new(
        ExecutionTier::ConnectorApi,
        "workspace-tools",
        "1.0.0",
        [
            Capability::well_known(lumi_protocol::capabilities::FILES_READ),
            Capability::well_known(lumi_protocol::capabilities::FILES_CREATE),
            Capability::well_known(lumi_protocol::capabilities::SHELL_EXECUTE),
        ],
    )];
    runtime.orchestrator.config.device_state = lumi_policy::DeviceExecutionState::Trusted;
    runtime.orchestrator.config.policy_version = "desktop-work-1".to_owned();

    let workspace = Workspace::attach(
        std::path::Path::new(&binding.workspace_root),
        &task.task_id,
        task.principal.principal_id.as_str(),
        Some(binding.project_id.clone()),
        Some(WorkspaceKind::ProjectRoot),
    )?;

    // A re-run of a failed/interrupted task carries the previous
    // attempt's verified progress so the planner continues the work
    // instead of colliding with its own earlier output.
    let resume_context = resume_context_from_ledger(&runtime.orchestrator, task);

    // Independent verification path: file postconditions read the real
    // disk under the workspace boundary; record/remote checks remain
    // unresolvable and therefore fail closed (AMBIGUOUS), never PASSED.
    let env = lumi_audit::FixtureEnvironment::new().with_workspace(workspace.root().to_path_buf());

    run_persisted_task(
        &mut runtime.orchestrator,
        planner,
        vec![
            Box::new(ReadFileTool),
            Box::new(WriteFileTool),
            Box::new(RunShellTool),
        ],
        &workspace,
        &env,
        task,
        resume_context.as_deref(),
        RunId::generate(),
    )
}

/// Production entry: builds the provider planner from the session and
/// executes the run. `transport` is injected so tests can replay the
/// provider wire without network.
///
/// # Errors
/// Unknown provider family, workspace attach, or store failures.
pub fn run_desktop_task_with_provider(
    runtime: &mut DesktopRuntime,
    task: &Task,
    session: &ProviderSession,
    transport: &dyn HttpTransport,
) -> Result<lumi_agent::runner::TaskRunOutcome, String> {
    let parts = provider_parts(session)?;
    let binding = task
        .project_binding
        .as_ref()
        .ok_or("task has no project binding; only project-bound tasks run locally")?;
    let tool_names = ["read_file", "write_file", "run_shell"];
    let planner = ModelPlanner {
        driver: parts.driver.as_ref(),
        instance: &parts.instance,
        auth: &parts.auth,
        transport,
        model: session.model.clone(),
        tools: lumi_agent::tools::default_tool_specs(),
        system_prompt: lumi_agent::planner::system_prompt(
            &task.goal,
            &binding.workspace_root,
            &tool_names,
        ),
        timeout_ms: 120_000,
    };
    run_desktop_task(runtime, task, &planner)
}
