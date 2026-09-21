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
    Capability, ConsumedBudget, EnvironmentId, ExecutionStatus, ExecutionTier, ProjectId,
    ProjectTaskBinding, ResourceType, Run, RunId, RunState, Task, TaskId, TaskStatus, TenantId,
    Timestamp, WorkspaceKind,
};
use lumi_workspaces::Workspace;
use std::collections::BTreeSet;
use std::path::Path;

/// The provider session the user configured for this app session. The
/// credential lives in memory only — never persisted, never logged.
#[derive(Clone, PartialEq, Eq)]
pub struct ProviderSession {
    /// `"openai"` or `"openai-compatible"` for any compatible endpoint.
    pub family: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
}

impl std::fmt::Debug for ProviderSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderSession")
            .field("family", &self.family)
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("api_key", &"[REDACTED]")
            .finish()
    }
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
pub(crate) fn resume_context_from_ledger(
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
            let verified = match (&details.execution_status, &details.verification) {
                (Some(ExecutionStatus::Success), lumi_audit::VerificationStatus::Passed) => true,
                // A non-consequential read has no external effect to verify;
                // it is safe progress to carry forward. Consequential work
                // must always have independent Passed evidence.
                (Some(ExecutionStatus::Success), lumi_audit::VerificationStatus::NotRequired) => {
                    !details.risk_class.is_consequential()
                }
                _ => false,
            };
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

// ===== Gated UI file saves (Spec 30.5 / 30.12) =====
//
// A standalone Files-tab save is a standalone consequential operation:
// the shell mints a lightweight project-bound USER Task/Run, normalizes
// the save into an ActionProposal, persists durable intent, passes the
// policy gate, executes the mutation, verifies the actual file on disk,
// and audits the result. There is no privileged direct-save path.

/// One UI file mutation, normalized for the gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSaveOp {
    Create {
        path: String,
        content: String,
    },
    /// Binary create (Spec 30 Office edits): payload arrives base64.
    CreateBinary {
        path: String,
        content_base64: String,
    },
    Edit {
        path: String,
        expected_sha256: String,
        content: String,
    },
    /// Binary edit (Spec 30 Office edits): payload arrives base64.
    EditBinary {
        path: String,
        expected_sha256: String,
        content_base64: String,
    },
    Delete {
        path: String,
        expected_sha256: Option<String>,
    },
}

fn user_principal(tenant: &TenantId) -> lumi_protocol::Principal {
    lumi_protocol::Principal {
        principal_id: lumi_protocol::PrincipalId::generate(),
        tenant_id: tenant.clone(),
        kind: lumi_protocol::PrincipalKind::User,
        authenticated_at: Some(Timestamp::now()),
        authentication_strength: Some(lumi_protocol::AuthenticationStrength::DevicePossession),
    }
}

/// Executes one UI file save through the full orchestrator gate on a
/// dedicated durable USER Task/Run: normalize → durable intent → policy
/// → execute → verify actual file → audit. Same fail-closed project
/// path resolution and checksum guards as before — but now attributed,
/// journaled, policy-checked, and evidence-producing like every other
/// Lumi action.
///
/// # Errors
/// Project lookup, policy denial, executor failure, or verification
/// failure. The durable task/run records persist the outcome first; the
/// error string is for the UI toast.
pub fn gated_file_save(
    runtime: &mut DesktopRuntime,
    projects: &mut crate::projects::ProjectService,
    project_id: &str,
    op: &FileSaveOp,
) -> Result<(), String> {
    use lumi_state::StateStore as _;

    // Binary payloads (Spec 30 Office edits) arrive base64-encoded from
    // the UI; decode BEFORE anything is persisted so a malformed payload
    // fails closed with no Task/Run/Action on the books.
    let payload_bytes: Vec<u8> = match op {
        FileSaveOp::Create { content, .. } | FileSaveOp::Edit { content, .. } => {
            content.as_bytes().to_vec()
        }
        FileSaveOp::CreateBinary { content_base64, .. }
        | FileSaveOp::EditBinary { content_base64, .. } => {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD
                .decode(content_base64)
                .map_err(|e| format!("decode save payload: {e}"))?
        }
        FileSaveOp::Delete { .. } => Vec::new(),
    };

    let (goal, path, capability, operation, postconditions, expected_sha) = match op {
        FileSaveOp::Create { path, .. } | FileSaveOp::CreateBinary { path, .. } => {
            let sha = lumi_protocol::canonical::sha256_hex(&payload_bytes);
            (
                format!("Create {path}"),
                path.clone(),
                lumi_protocol::capabilities::FILES_CREATE,
                "create_file",
                vec![lumi_protocol::Postcondition {
                    id: lumi_protocol::PostconditionId::new("saved-content"),
                    description: "saved file carries the candidate checksum".to_owned(),
                    check: lumi_protocol::PostconditionCheck::FileChecksum {
                        path: path.clone(),
                        sha256: sha,
                    },
                }],
                None,
            )
        }
        FileSaveOp::Edit {
            path,
            expected_sha256,
            ..
        }
        | FileSaveOp::EditBinary {
            path,
            expected_sha256,
            ..
        } => {
            let sha = lumi_protocol::canonical::sha256_hex(&payload_bytes);
            (
                format!("Edit {path}"),
                path.clone(),
                lumi_protocol::capabilities::FILES_EDIT,
                "edit_file",
                vec![lumi_protocol::Postcondition {
                    id: lumi_protocol::PostconditionId::new("edited-content"),
                    description: "edited file carries the candidate checksum".to_owned(),
                    check: lumi_protocol::PostconditionCheck::FileChecksum {
                        path: path.clone(),
                        sha256: sha,
                    },
                }],
                Some(expected_sha256.clone()),
            )
        }
        FileSaveOp::Delete {
            path,
            expected_sha256,
        } => (
            format!("Delete {path}"),
            path.clone(),
            lumi_protocol::capabilities::FILES_DELETE,
            "delete_file",
            Vec::new(),
            expected_sha256.clone(),
        ),
    };

    let overview = projects.overview(project_id)?;
    let root = overview.primary_root;
    let tenant =
        TenantId::parse(DesktopRuntime::LOCAL_TENANT).map_err(|e| format!("local tenant: {e}"))?;
    let principal = user_principal(&tenant);
    // Durable USER Task/Run bound to the project (Spec 30.5): created
    // before normalization, persisted before any intent is recorded.
    let mut task = lumi_protocol::Task {
        task_id: TaskId::generate(),
        tenant_id: tenant.clone(),
        principal: principal.clone(),
        mode: lumi_protocol::TaskMode::Work,
        goal: goal.clone(),
        created_at: Timestamp::now(),
        deadline: None,
        budget: lumi_protocol::Budget::default(),
        privacy_constraints: lumi_protocol::PrivacyConstraint::default(),
        status: TaskStatus::Created,
        requested_outputs: vec![],
        project_binding: Some(ProjectTaskBinding {
            project_id: ProjectId::parse(project_id).map_err(|e| e.to_string())?,
            execution_environment_id: EnvironmentId::generate(),
            workspace_root: root.clone(),
            workspace_kind: WorkspaceKind::ProjectRoot,
        }),
    };
    runtime
        .orchestrator
        .store
        .save_task(&task)
        .map_err(|e| format!("persist save task: {e}"))?;
    task.status = TaskStatus::Running;
    runtime
        .orchestrator
        .store
        .save_task(&task)
        .map_err(|e| format!("persist task RUNNING: {e}"))?;
    let mut run = Run {
        run_id: RunId::generate(),
        task_id: task.task_id.clone(),
        runtime_version: env!("CARGO_PKG_VERSION").to_owned(),
        workflow_version: None,
        selected_providers: vec!["desktop-ui".to_owned()],
        started_at: Timestamp::now(),
        ended_at: None,
        state: RunState::Executing,
        budgets_consumed: ConsumedBudget::default(),
        failure: None,
    };
    runtime
        .orchestrator
        .store
        .save_run(&run)
        .map_err(|e| format!("persist save run: {e}"))?;

    // Narrow grants + executor descriptor for desktop file saves.
    let file_grant = |id: &str, capability: &'static str| CapabilityGrant {
        grant_id: id.to_owned(),
        tenant_id: tenant.clone(),
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
        .grant(file_grant(
            "g-files-create",
            lumi_protocol::capabilities::FILES_CREATE,
        ))
        .grant(file_grant(
            "g-files-edit",
            lumi_protocol::capabilities::FILES_EDIT,
        ))
        .grant(file_grant(
            "g-files-delete",
            lumi_protocol::capabilities::FILES_DELETE,
        ));
    runtime.orchestrator.config.executors = vec![ExecutorDescriptor::new(
        ExecutionTier::ConnectorApi,
        "desktop-file-saves",
        "1.0.0",
        [
            Capability::well_known(lumi_protocol::capabilities::FILES_CREATE),
            Capability::well_known(lumi_protocol::capabilities::FILES_EDIT),
            Capability::well_known(lumi_protocol::capabilities::FILES_DELETE),
        ],
    )];
    runtime.orchestrator.config.device_state = lumi_policy::DeviceExecutionState::Trusted;
    runtime.orchestrator.config.policy_version = "desktop-save-1".to_owned();

    let action = lumi_protocol::ActionProposal::builder(
        lumi_protocol::ActionId::generate(),
        task.task_id.clone(),
        run.run_id.clone(),
        principal.clone(),
        Capability::well_known(capability),
        lumi_protocol::ResourceRef {
            resource_type: lumi_protocol::ResourceType::well_known(ResourceType::FILE),
            id: format!("{project_id}/{path}"),
            sensitivity: None,
        },
        lumi_protocol::Target::canonical(format!(
            "file://{}",
            std::path::Path::new(&root).join(&path).display()
        )),
        operation,
        match op {
            FileSaveOp::Delete { .. } => lumi_protocol::RiskClass::Destructive,
            _ => lumi_protocol::RiskClass::LocalWrite,
        },
    )
    .arguments(serde_json::json!({
        "path": path,
        "expected_sha256": expected_sha,
    }))
    .postconditions(postconditions)
    .build()
    .map_err(|e| format!("normalize save action: {e}"))?;

    // Verification reads the REAL file under the project boundary.
    let env = lumi_audit::FixtureEnvironment::new().with_workspace(std::path::PathBuf::from(&root));

    let mut consumed = lumi_protocol::ConsumedBudget::default();
    let task_id = task.task_id.clone();
    let op_for_executor = op.clone();
    let projects_ref = projects;
    let mut executor = |authorized: &lumi_protocol::ActionProposal| {
        let result =
            projects_ref.mutate(
                project_id,
                task_id.as_str(),
                |files| match &op_for_executor {
                    FileSaveOp::Create { path, .. } | FileSaveOp::CreateBinary { path, .. } => {
                        files
                            .create_file(Path::new(path), &payload_bytes)
                            .map(|o| vec![o])
                    }
                    FileSaveOp::Edit {
                        path,
                        expected_sha256,
                        ..
                    }
                    | FileSaveOp::EditBinary {
                        path,
                        expected_sha256,
                        ..
                    } => files
                        .edit_file(Path::new(path), expected_sha256, &payload_bytes)
                        .map(|o| vec![o]),
                    FileSaveOp::Delete {
                        path,
                        expected_sha256,
                    } => files
                        .delete_file(Path::new(path), expected_sha256.as_deref(), false)
                        .map(|o| vec![o]),
                },
            );
        let now = Timestamp::now();
        match result {
            Ok(outcomes) => lumi_protocol::ExecutionResult {
                action_id: authorized.action_id.clone(),
                task_id: authorized.task_id.clone(),
                run_id: authorized.run_id.clone(),
                status: lumi_protocol::ExecutionStatus::Success,
                started_at: now,
                ended_at: Timestamp::now(),
                grounding: Some(lumi_protocol::Grounding::DeterministicApi),
                observation_ids: outcomes
                    .iter()
                    .map(|o| o.sha256_after.clone().unwrap_or_default())
                    .collect(),
                error: None,
            },
            Err(e) => lumi_protocol::ExecutionResult {
                action_id: authorized.action_id.clone(),
                task_id: authorized.task_id.clone(),
                run_id: authorized.run_id.clone(),
                status: lumi_protocol::ExecutionStatus::Failed,
                started_at: now,
                ended_at: Timestamp::now(),
                grounding: Some(lumi_protocol::Grounding::DeterministicApi),
                observation_ids: vec![],
                error: Some(lumi_protocol::ErrorEnvelope::new(
                    lumi_protocol::FailureCategory::Filesystem,
                    e.to_string(),
                )),
            },
        }
    };

    let outcome = runtime.orchestrator.execute_step(
        &action,
        None,
        &lumi_protocol::Budget::default(),
        &mut consumed,
        &env,
        &mut executor,
    );
    let _ = projects_ref;

    use lumi_orchestrator::StepOutcome;
    let mut terminal =
        |status: TaskStatus, failure: Option<lumi_protocol::ErrorEnvelope>| -> Result<(), String> {
            task.status = status;
            run.ended_at = Some(Timestamp::now());
            run.state = RunState::Verifying;
            run.budgets_consumed = consumed;
            run.failure = failure;
            if let Err(e) = runtime.orchestrator.store.save_run(&run) {
                return Err(format!("persist save run outcome: {e}"));
            }
            if let Err(e) = runtime.orchestrator.store.save_task(&task) {
                return Err(format!("persist save task outcome: {e}"));
            }
            Ok(())
        };

    match outcome {
        StepOutcome::VerifiedSuccess { .. } => {
            terminal(TaskStatus::Completed, None)?;
            Ok(())
        }
        StepOutcome::Denied { reason } => {
            terminal(
                TaskStatus::Failed,
                Some(lumi_protocol::ErrorEnvelope::new(
                    lumi_protocol::FailureCategory::PolicyDenyExpected,
                    reason.clone(),
                )),
            )?;
            Err(format!("policy denied the save: {reason}"))
        }
        StepOutcome::Unverified { detail, .. } => {
            terminal(
                TaskStatus::Failed,
                Some(lumi_protocol::ErrorEnvelope::new(
                    lumi_protocol::FailureCategory::Postcondition,
                    detail.clone(),
                )),
            )?;
            Err(format!("save did not verify: {detail}"))
        }
        StepOutcome::ApprovalNeeded { reason, .. } => {
            // The UI delete confirm dialog is the user-intent surface;
            // a policy-level approval demand is surfaced honestly and
            // the durable task stays WAITING_APPROVAL for the queue.
            task.status = TaskStatus::WaitingApproval;
            if let Err(e) = runtime.orchestrator.store.save_task(&task) {
                return Err(format!("persist waiting task: {e}"));
            }
            Err(format!("approval required: {reason}"))
        }
        StepOutcome::Ambiguous { .. } => {
            terminal(
                TaskStatus::Ambiguous,
                Some(lumi_protocol::ErrorEnvelope::new(
                    lumi_protocol::FailureCategory::AmbiguousState,
                    "save outcome unknown; verify externally".to_owned(),
                )),
            )?;
            Err("save ended ambiguous; verify the file externally".to_owned())
        }
        StepOutcome::Stopped { reason } => {
            // The cancel token stops with reason "cancelled"; a terminal
            // executor error (retry policy) also arrives as Stopped, and
            // must map to FAILED, not Cancelled.
            let cancelled = reason == "cancelled" || reason.starts_with("cancelled");
            let (status, category) = if cancelled {
                (
                    TaskStatus::Cancelled,
                    lumi_protocol::FailureCategory::UserCancel,
                )
            } else {
                (
                    TaskStatus::Failed,
                    lumi_protocol::FailureCategory::Filesystem,
                )
            };
            terminal(
                status,
                Some(lumi_protocol::ErrorEnvelope::new(category, reason.clone())),
            )?;
            Err(format!("save stopped: {reason}"))
        }
        StepOutcome::Retryable { error } => {
            terminal(
                TaskStatus::Failed,
                Some(lumi_protocol::ErrorEnvelope::new(
                    lumi_protocol::FailureCategory::Filesystem,
                    error.message.clone(),
                )),
            )?;
            Err(format!("save failed: {}", error.message))
        }
    }
}
