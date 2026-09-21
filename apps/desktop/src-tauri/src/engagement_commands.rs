//! Project engagement IPC and the local scheduler lifecycle. No alternate
//! executor: scheduled and foreground work call the same selected task runner.
use super::AppState;
use lumi_desktop::engagement::{
    self, Automation, AutomationInput, CreateRequest, EngagementStore, TaskOptions, ToolId,
    MAX_RUN_SECONDS,
};
use lumi_models::ureq_transport::UreqTransport;
use lumi_protocol::{Task, TaskId, TaskStatus};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
pub struct TaskRunDto {
    pub started: bool,
    pub task_id: String,
}

pub struct RunningGuard(pub(crate) Arc<AtomicBool>);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn with_store<T>(
    state: &AppState,
    f: impl FnOnce(&mut EngagementStore) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state
        .engagement
        .lock()
        .map_err(|_| "engagement state lock poisoned")?;
    f(guard.as_mut().map_err(|e| e.clone())?)
}
fn check_project(state: &AppState, project: &str) -> Result<(), String> {
    state
        .projects
        .try_lock()
        .map_err(|_| "project is busy; retry shortly")?
        .overview(project)?;
    Ok(())
}
fn provider_ready(state: &AppState) -> bool {
    state.provider.lock().is_ok_and(|p| p.is_some())
}

#[derive(Serialize)]
pub struct CapabilityDto {
    id: ToolId,
    state: &'static str,
    reason: &'static str,
    authority_class: &'static str,
    interactive: bool,
    unattended: bool,
}
#[derive(Serialize)]
pub struct EngagementSnapshot {
    capabilities: Vec<CapabilityDto>,
    browser_origins: Vec<String>,
    provider_configured: bool,
    stopped: bool,
}
fn snapshot(state: &AppState, project: &str) -> Result<EngagementSnapshot, String> {
    check_project(state, project)?;
    let origins = with_store(state, |s| s.origins(project))?;
    let browser_ready = state.browser_ready.load(Ordering::SeqCst) && !origins.is_empty();
    let capabilities = lumi_desktop::engagement::capability_catalog()
        .into_iter()
        .map(|descriptor| {
            let (state, reason) = match descriptor.id {
                ToolId::Files => ("ready", "files_ready"),
                ToolId::Shell => ("ready", "shell_manual"),
                ToolId::Browser if browser_ready => ("ready", "browser_ready"),
                ToolId::Browser if origins.is_empty() => ("setup_required", "browser_scope"),
                ToolId::Browser => ("setup_required", "browser_setup"),
                ToolId::Chrome => ("unavailable", "chrome_unavailable"),
                ToolId::Computer if state.native_adapter.is_some() => ("ready", "computer_ready"),
                ToolId::Computer => ("setup_required", "computer_setup"),
            };
            CapabilityDto {
                id: descriptor.id,
                state,
                reason,
                authority_class: descriptor.authority_class,
                interactive: descriptor.interactive,
                unattended: descriptor.unattended,
            }
        })
        .collect();
    Ok(EngagementSnapshot {
        capabilities,
        browser_origins: origins,
        provider_configured: provider_ready(state),
        stopped: state.cancel.is_cancelled(),
    })
}
#[tauri::command]
pub fn engagement_snapshot(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<EngagementSnapshot, String> {
    snapshot(&state, &project_id)
}
#[tauri::command]
pub fn engagement_check_browser(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<EngagementSnapshot, String> {
    check_project(&state, &project_id)?;
    if state.running.load(Ordering::SeqCst) {
        return Err("finish the active task before checking browser setup".into());
    }
    state.browser_ready.store(false, Ordering::SeqCst);
    state
        .browser_config
        .as_ref()
        .ok_or("matching Node/Playwright browser worker is not installed")?
        .probe()?;
    state.browser_ready.store(true, Ordering::SeqCst);
    snapshot(&state, &project_id)
}
#[tauri::command]
pub fn engagement_set_browser_origins(
    state: tauri::State<'_, AppState>,
    project_id: String,
    origins: Vec<String>,
) -> Result<EngagementSnapshot, String> {
    check_project(&state, &project_id)?;
    let _permit = reserve(&state)?;
    with_store(&state, |store| store.set_origins(&project_id, origins))?;
    snapshot(&state, &project_id)
}
fn validate_options(
    state: &AppState,
    project: &str,
    goal: &str,
    options: &TaskOptions,
) -> Result<(), String> {
    let selected =
        engagement::normalized_tools(goal, &options.tools, options.automation_id.is_some())?;
    if selected.contains(&ToolId::Chrome) {
        return Err("authenticated Chrome sessions are not enabled in this build".into());
    }
    if selected.contains(&ToolId::Browser) {
        if !state.browser_ready.load(Ordering::SeqCst) {
            return Err("check managed browser setup before running this task".into());
        }
        state
            .browser_config
            .as_ref()
            .ok_or("managed browser unavailable")?
            .verify()?;
        let allowed = with_store(state, |s| s.origins(project))?;
        if options.browser_origins.is_empty()
            || options
                .browser_origins
                .iter()
                .any(|origin| !allowed.contains(origin))
        {
            return Err(
                "browser website access changed; create a task with the current project scope"
                    .into(),
            );
        }
    }
    if selected.contains(&ToolId::Computer) && state.native_adapter.is_none() {
        return Err("install the reviewed Cua Driver 0.28.2 before running computer tasks".into());
    }
    if options
        .authorization_until
        .is_some_and(|until| until <= engagement::now())
    {
        return Err("run authorization expired".into());
    }
    Ok(())
}
#[tauri::command]
pub fn engagement_create_task(
    state: tauri::State<'_, AppState>,
    request: CreateRequest,
) -> Result<Task, String> {
    engagement::validate_request(&request)?;
    check_project(&state, &request.project_id)?;
    let _permit = reserve(&state)?;
    if let Some(id) = with_store(&state, |s| s.existing_request(&request))? {
        return state
            .runtime
            .try_lock()
            .map_err(|_| "runtime is busy")?
            .load_task(&TaskId::parse(id).map_err(|e| e.to_string())?);
    }
    let options = TaskOptions {
        tools: engagement::normalized_tools(&request.goal, &request.tools, false)?,
        browser_origins: with_store(&state, |s| s.origins(&request.project_id))?,
        automation_id: None,
        authorization_until: None,
    };
    validate_options(&state, &request.project_id, &request.goal, &options)?;
    let task = {
        let projects = state.projects.try_lock().map_err(|_| "project is busy")?;
        let mut runtime = state.runtime.try_lock().map_err(|_| "runtime is busy")?;
        projects.task_create(&mut runtime, &request.project_id, &request.goal)?
    };
    // A failed metadata commit leaves a non-running task. It never starts I/O.
    with_store(&state, |s| {
        s.attach_task(&request, task.task_id.as_str(), options)
    })?;
    Ok(task)
}
fn reserve(state: &AppState) -> Result<RunningGuard, String> {
    if state.cancel.is_cancelled() {
        return Err("agent is stopped; no new execution is admitted".into());
    }
    state
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| "another task is running; retry after it finishes")?;
    Ok(RunningGuard(Arc::clone(&state.running)))
}
fn revoke(state: &AppState, id: &str) -> Result<(), String> {
    if let Some(flag) = state
        .automation_revocations
        .lock()
        .map_err(|_| "revocation registry poisoned")?
        .remove(id)
    {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(())
}

pub fn start_task(state: &AppState, task_id: &str) -> Result<TaskRunDto, String> {
    let permit = reserve(state)?;
    let task = state
        .runtime
        .try_lock()
        .map_err(|_| "runtime is busy")?
        .load_task(&TaskId::parse(task_id).map_err(|e| e.to_string())?)?;
    if task.status == TaskStatus::Completed {
        return Ok(TaskRunDto {
            started: true,
            task_id: task_id.into(),
        });
    }
    let options = with_store(state, |store| store.task_options(task_id))?;
    // Automation reruns must mint a new occurrence, not reuse an old lease.
    if options.automation_id.is_some() {
        return Err("use Run now on the automation to create a new auditable occurrence".into());
    }
    launch(state, task, options, permit)
}
fn launch(
    state: &AppState,
    task: Task,
    options: TaskOptions,
    permit: RunningGuard,
) -> Result<TaskRunDto, String> {
    if !lumi_desktop::is_runnable(&task.status) {
        return Err("task cannot be run in its current state".into());
    }
    let binding = task
        .project_binding
        .as_ref()
        .ok_or("task has no project binding")?;
    let overview = state
        .projects
        .try_lock()
        .map_err(|_| "project is busy")?
        .overview(binding.project_id.as_str())?;
    if overview.primary_root != binding.workspace_root {
        return Err("project location changed; create a task in the current project".into());
    }
    validate_options(state, binding.project_id.as_str(), &task.goal, &options)?;
    let session = state
        .provider
        .lock()
        .map_err(|_| "provider session unavailable")?
        .clone()
        .ok_or("configure a model provider for this app session")?;
    let revoked = if let Some(id) = &options.automation_id {
        let flag = state
            .automation_revocations
            .lock()
            .map_err(|_| "revocation registry poisoned")?
            .get(id)
            .cloned()
            .ok_or("automation authority was revoked")?;
        if flag.load(Ordering::SeqCst) {
            return Err("automation authority was revoked".into());
        }
        with_store(state, |s| {
            s.record_outcome(task.task_id.as_str(), "RUNNING", None)
        })?;
        flag
    } else {
        Arc::new(AtomicBool::new(false))
    };
    let result = TaskRunDto {
        started: true,
        task_id: task.task_id.to_string(),
    };
    let runtime = Arc::clone(&state.runtime);
    let store = Arc::clone(&state.engagement);
    let config = state.browser_config.clone();
    let state_native = state.native_adapter.clone();
    let cancel = state.cancel.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let outcome = match runtime.lock() {
            Ok(mut runtime) => lumi_desktop::engagement_runner::run_selected_task(
                &mut runtime,
                &task,
                &options,
                &session,
                &UreqTransport,
                config.as_ref(),
                state_native.as_ref(),
                revoked,
            ),
            Err(_) => Err("runtime lock poisoned".into()),
        };
        let (status, note) = match outcome {
            Ok(outcome) => match outcome.status {
                lumi_desktop::engagement_runner::TaskRunStatus::Completed => ("COMPLETED", None),
                lumi_desktop::engagement_runner::TaskRunStatus::WaitingApproval => {
                    ("WAITING_APPROVAL", outcome.failure)
                }
                lumi_desktop::engagement_runner::TaskRunStatus::Failed => {
                    ("FAILED", outcome.failure)
                }
            },
            Err(error) => ("FAILED", Some(error)),
        };
        if options.automation_id.is_some() {
            let saved = store
                .lock()
                .map_err(|_| "engagement lock poisoned".to_owned())
                .and_then(|mut s| {
                    s.as_mut().map_err(|e| e.clone())?.record_outcome(
                        task.task_id.as_str(),
                        status,
                        note,
                    )
                });
            if saved.is_err() {
                cancel.cancel();
                log::error!("automation outcome persistence failed; admission stopped");
            }
        }
    });
    Ok(result)
}

#[derive(Serialize)]
pub struct AutomationList {
    items: Vec<Automation>,
    provider_configured: bool,
    stopped: bool,
    scheduler_available: bool,
}
#[tauri::command]
pub fn automation_list(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<AutomationList, String> {
    check_project(&state, &project_id)?;
    Ok(AutomationList {
        items: with_store(&state, |s| s.list(&project_id))?,
        provider_configured: provider_ready(&state),
        stopped: state.cancel.is_cancelled(),
        scheduler_available: true,
    })
}
#[tauri::command]
pub fn automation_preview(input: AutomationInput) -> Result<Vec<i64>, String> {
    input.preview(engagement::now())
}
#[tauri::command]
pub fn automation_save(
    state: tauri::State<'_, AppState>,
    project_id: String,
    input: AutomationInput,
) -> Result<Automation, String> {
    check_project(&state, &project_id)?;
    let id = input.automation_id.clone();
    with_store(&state, |s| {
        let saved = s.save(&project_id, input, engagement::now())?;
        if let Some(id) = &id {
            revoke(&state, id)?;
        }
        Ok(saved)
    })
}
#[tauri::command]
pub fn automation_toggle(
    state: tauri::State<'_, AppState>,
    project_id: String,
    automation_id: String,
    revision: u64,
    enabled: bool,
) -> Result<(), String> {
    check_project(&state, &project_id)?;
    with_store(&state, |s| {
        s.toggle(
            &project_id,
            &automation_id,
            revision,
            enabled,
            engagement::now(),
        )?;
        revoke(&state, &automation_id)
    })
}
#[tauri::command]
pub fn automation_delete(
    state: tauri::State<'_, AppState>,
    project_id: String,
    automation_id: String,
    revision: u64,
) -> Result<(), String> {
    check_project(&state, &project_id)?;
    with_store(&state, |s| {
        s.remove(&project_id, &automation_id, revision)?;
        revoke(&state, &automation_id)
    })
}
#[derive(Serialize)]
pub struct AutomationRunDto {
    task_id: String,
}
#[tauri::command]
pub fn automation_run_now(
    state: tauri::State<'_, AppState>,
    project_id: String,
    automation_id: String,
    request_id: String,
) -> Result<AutomationRunDto, String> {
    if request_id.is_empty() || request_id.len() > 128 {
        return Err("invalid request identity".into());
    }
    let automation = with_store(&state, |s| s.get(&project_id, &automation_id))?;
    let occurrence = format!("{automation_id}:manual:{request_id}");
    if let Some(prior) = automation
        .runs
        .iter()
        .find(|r| r.occurrence_id == occurrence)
    {
        return prior
            .task_id
            .clone()
            .map(|task_id| AutomationRunDto { task_id })
            .ok_or_else(|| "this occurrence needs review; it is not safe to replay".into());
    }
    let permit = reserve(&state)?;
    let result = dispatch_automation(&state, automation, occurrence, true, permit)?;
    Ok(AutomationRunDto {
        task_id: result.task_id,
    })
}
fn dispatch_automation(
    state: &AppState,
    automation: Automation,
    occurrence: String,
    manual: bool,
    permit: RunningGuard,
) -> Result<TaskRunDto, String> {
    if !provider_ready(state) {
        return Err("configure a model provider for this app session".into());
    }
    let now = engagement::now();
    let id = automation.id()?.to_owned();
    let options = TaskOptions {
        tools: engagement::normalized_tools(&automation.input.goal, &automation.input.tools, true)?,
        browser_origins: with_store(state, |s| s.origins(&automation.project_id))?,
        automation_id: Some(id.clone()),
        authorization_until: if manual {
            Some(now + MAX_RUN_SECONDS)
        } else {
            automation.input.authorization_until
        },
    };
    check_project(state, &automation.project_id)?;
    validate_options(
        state,
        &automation.project_id,
        &automation.input.goal,
        &options,
    )?;
    with_store(state, |s| {
        if s.get(&automation.project_id, &id)?.revision() != automation.revision() {
            return Err("automation changed before admission; refresh and retry".into());
        }
        s.claim(
            &automation.project_id,
            &id,
            &occurrence,
            if manual {
                now
            } else {
                automation.next_run_at.unwrap_or(now)
            },
            now,
            manual,
        )?;
        state
            .automation_revocations
            .lock()
            .map_err(|_| "revocation registry poisoned")?
            .insert(id.clone(), Arc::new(AtomicBool::new(false)));
        Ok(())
    })?;
    let task = {
        let projects = state.projects.try_lock().map_err(|_| "project is busy")?;
        let mut runtime = state.runtime.try_lock().map_err(|_| "runtime is busy")?;
        projects.task_create(&mut runtime, &automation.project_id, &automation.input.goal)?
    };
    with_store(state, |s| {
        s.bind_occurrence(
            &automation.project_id,
            &id,
            &occurrence,
            task.task_id.as_str(),
            options.clone(),
        )
    })?;
    let task_id = task.task_id.to_string();
    match launch(state, task, options, permit) {
        Ok(result) => Ok(result),
        Err(error) => {
            with_store(state, |s| {
                s.record_outcome(&task_id, "BLOCKED", Some(error.clone()))
            })?;
            Err(error)
        }
    }
}
pub fn start_scheduler(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(15));
        let state = app.state::<AppState>();
        if !provider_ready(&state) || state.cancel.is_cancelled() {
            continue;
        }
        let Ok(permit) = reserve(&state) else {
            continue;
        };
        let item = with_store(&state, |s| s.due(engagement::now()));
        match item {
            Ok(Some(automation)) => {
                let Ok(id) = automation.id() else { continue };
                let occurrence = format!("{id}:{}", automation.next_run_at.unwrap_or(0));
                // Missing setup does not consume an occurrence or weaken scope.
                let project = automation.project_id.clone();
                let automation_id = automation.id().unwrap_or_default().to_owned();
                let revision = automation.revision();
                if let Err(reason) =
                    dispatch_automation(&state, automation, occurrence, false, permit)
                {
                    // Persist the blocker and continue serving other schedules. A
                    // newer user edit must never be overwritten by this failure.
                    let parked = with_store(&state, |store| {
                        if store.pause_blocked_if_current(
                            &project,
                            &automation_id,
                            revision,
                            reason,
                        )? {
                            revoke(&state, &automation_id)?;
                        }
                        Ok(())
                    });
                    if parked.is_err() {
                        state.cancel.cancel();
                    }
                }
            }
            Ok(None) => {}
            Err(_) => {
                state.cancel.cancel();
                log::error!("scheduler state unavailable; admission stopped");
            }
        }
    });
}
