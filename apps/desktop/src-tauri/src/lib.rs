//! Lumi desktop app: a thin viewport onto runtime-owned state.
//!
//! The shell reads a durable local runtime snapshot. Categories the runtime
//! cannot prove remain unknown; the UI cannot create policy, credentials,
//! approvals, or executor state.

use lumi_desktop::{
    gated_file_save, ConnectionRecord,
    ConnectionsStore, DesktopRuntime, FileSaveOp, KillSwitchState, OperationsSnapshot,
    ProjectService, ProviderSession,
};
use lumi_secrets::{KeyringBackend, SecretBroker};
use lumi_state::CancelToken;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Manager;
use std::path::PathBuf;

mod engagement_commands;
use engagement_commands::*;

/// Shared application state managed by Tauri.
pub struct AppState {
    /// Durable local runtime and its real orchestrator cancellation token.
    /// Arc so the task worker can hold it across a blocking run while the
    /// rest of the shell degrades to lock-free reads.
    pub runtime: Arc<Mutex<DesktopRuntime>>,
    /// Durable project registry and project-backed services (spec 26).
    pub projects: Mutex<ProjectService>,
    pub cancel: CancelToken,
    pub stop_marker: std::path::PathBuf,
    /// Single-flight guard for task execution.
    pub running: Arc<AtomicBool>,
    /// The user's in-memory provider session. Never persisted; the
    /// credential never leaves this process boundary.
    pub provider: Arc<Mutex<Option<ProviderSession>>>,
    pub provider_remembered: AtomicBool,
    /// Durable state file path, for lock-free task reads while a run
    /// holds the runtime mutex.
    pub state_path: std::path::PathBuf,
    /// Project connection registry (metadata) — Spec 29.
    pub connections: Arc<Mutex<ConnectionsStore>>,
    /// Secrets broker: credential values live here (OS keyring) and are
    /// resolved at the narrowest boundary only.
    pub broker: Arc<SecretBroker<KeyringBackend>>,
    pub engagement: Arc<Mutex<Result<lumi_desktop::engagement::EngagementStore, String>>>,
    pub browser_config: Option<lumi_desktop::browser_tools::BrowserConfig>,
    pub browser_ready: AtomicBool,
    pub automation_revocations: Mutex<std::collections::BTreeMap<String, Arc<AtomicBool>>>,
    pub native_adapter: Option<Arc<lumi_native::CuaDriverAdapter>>,
}

fn discover_native_adapter() -> Option<Arc<lumi_native::CuaDriverAdapter>> {
    #[cfg(target_os = "macos")]
    {
        lumi_native::CuaDriverAdapter::connect_installed(
            lumi_native::RuntimeGeneration::default(),
        )
        .ok()
        .flatten()
        .map(Arc::new)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

impl AppState {
    #[must_use]
    pub fn new(
        runtime: DesktopRuntime,
        projects: ProjectService,
        state_dir: PathBuf,
    ) -> Self {
        let cancel = runtime.cancellation_token();
        let stop_marker = runtime.stop_marker_path().to_path_buf();
        let state_path = runtime.state_path().to_path_buf();
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            projects: Mutex::new(projects),
            cancel,
            stop_marker,
            running: Arc::new(AtomicBool::new(false)),
            provider: Arc::new(Mutex::new(None)),
            provider_remembered: AtomicBool::new(false),
            state_path,
            connections: Arc::new(Mutex::new(ConnectionsStore::new(
                state_dir.join("connections.json"),
            ))),
            broker: Arc::new(SecretBroker::new(KeyringBackend::with_service(
                "Lumi Agents",
            ))),
            engagement: Arc::new(Mutex::new(lumi_desktop::engagement::EngagementStore::open(
                state_dir.join("engagement.json"),
            ))),
            browser_config: lumi_desktop::browser_tools::BrowserConfig::discover(None),
            browser_ready: AtomicBool::new(false),
            automation_revocations: Mutex::new(std::collections::BTreeMap::new()),
            native_adapter: discover_native_adapter(),
        }
    }

    /// The UI and orchestrator share this exact cancellation token.
    #[must_use]
    pub fn cancellation_token(&self) -> CancelToken {
        self.cancel.clone()
    }
}

/// Kill-switch status derived from the shared cancellation token.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KillSwitchDto {
    pub state: KillSwitchState,
    pub running: bool,
}

fn kill_switch_dto(cancel: &CancelToken) -> KillSwitchDto {
    let stopped = cancel.is_cancelled();
    KillSwitchDto {
        state: if stopped {
            KillSwitchState::Stopped
        } else {
            KillSwitchState::Running
        },
        running: !stopped,
    }
}

#[tauri::command]
fn get_kill_switch(state: tauri::State<AppState>) -> KillSwitchDto {
    kill_switch_dto(&state.cancellation_token())
}

#[tauri::command]
fn emergency_stop(state: tauri::State<AppState>) -> Result<KillSwitchDto, String> {
    // Do not lock the runtime here. A worker may be holding the runtime
    // mutex while executing; the shared atomic token must interrupt it.
    state.cancel.cancel();
    persist_stop_marker(&state.stop_marker)?;
    Ok(kill_switch_dto(&state.cancellation_token()))
}

fn persist_stop_marker(path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create stop marker directory: {e}"))?;
    }
    std::fs::write(path, b"stopped\n").map_err(|e| format!("persist emergency stop: {e}"))
}

/// Returns the latest runtime-produced snapshot. `null` fields mean the
/// runtime has not supplied that category of state; they are not empty or
/// successful values.
#[tauri::command]
fn get_operations_snapshot(state: tauri::State<AppState>) -> OperationsSnapshot {
    match state.runtime.try_lock() {
        Ok(mut runtime) => runtime.snapshot(),
        // A task worker holds the runtime mutex. That is exactly the
        // "executing" state, reported honestly instead of blocking the
        // UI's snapshot poll for the run's whole duration.
        Err(_) => OperationsSnapshot {
            connection: lumi_desktop::api::ConnectionState::Connected,
            execution: lumi_desktop::api::ExecutionState::Executing,
            queue: None,
            progress: None,
            pending_approvals: None,
            exceptions: None,
            evidence: None,
            economics: None,
            permissions: None,
        },
    }
}


// ===== Project commands (spec 26 §26.29) =====
//
// Thin IPC adapters: each locks the shared service, delegates to the
// lumi-desktop project service, and maps errors to strings. No business
// logic and no policy live here.

#[tauri::command]
fn project_open_folder(
    state: tauri::State<'_, AppState>,
    path: String,
    display_name: Option<String>,
) -> Result<lumi_desktop::OpenedProject, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .open_folder(&path, display_name)
}

#[tauri::command]
fn project_list_recent(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<lumi_desktop::ProjectSummary>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .list_recent()
}

#[tauri::command]
fn project_overview(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<lumi_desktop::ProjectOverview, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .overview(&project_id)
}

#[tauri::command]
fn project_relink(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<lumi_desktop::ProjectSummary, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .relink(&project_id, &path)
}

#[tauri::command]
fn project_remove(state: tauri::State<'_, AppState>, project_id: String) -> Result<(), String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .remove(&project_id)
}

#[tauri::command]
fn file_list(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<Vec<lumi_project::ListedEntry>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .file_list(&project_id, &path)
}

#[tauri::command]
fn file_read(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<lumi_desktop::FileContent, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .file_read(&project_id, &path)
}

#[tauri::command]
fn file_read_base64(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<lumi_desktop::FileContentBase64, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .file_read_base64(&project_id, &path)
}

#[tauri::command]
fn file_create(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
    content: String,
) -> Result<(), String> {
    // Spec 30.12: the UI save is a standalone consequential operation —
    // gate it (USER Task/Run + ActionProposal + verification).
    let mut runtime = state
        .runtime
        .try_lock()
        .map_err(|_| "A task is running; saves are paused until it finishes.".to_owned())?;
    let mut projects = state
        .projects
        .lock()
        .expect("project service lock poisoned");
    gated_file_save(
        &mut runtime,
        &mut projects,
        &project_id,
        &FileSaveOp::Create { path, content },
    )
}

#[tauri::command]
fn file_create_base64(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
    content_base64: String,
) -> Result<(), String> {
    // Spec 30.12: binary Office artifacts save through the same gate —
    // USER Task/Run + ActionProposal + checksum postconditions.
    let mut runtime = state
        .runtime
        .try_lock()
        .map_err(|_| "A task is running; saves are paused until it finishes.".to_owned())?;
    let mut projects = state
        .projects
        .lock()
        .expect("project service lock poisoned");
    gated_file_save(
        &mut runtime,
        &mut projects,
        &project_id,
        &FileSaveOp::CreateBinary {
            path,
            content_base64,
        },
    )
}

#[tauri::command]
fn file_edit_base64(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
    expected_sha256: String,
    content_base64: String,
) -> Result<(), String> {
    let mut runtime = state
        .runtime
        .try_lock()
        .map_err(|_| "A task is running; saves are paused until it finishes.".to_owned())?;
    let mut projects = state
        .projects
        .lock()
        .expect("project service lock poisoned");
    gated_file_save(
        &mut runtime,
        &mut projects,
        &project_id,
        &FileSaveOp::EditBinary {
            path,
            expected_sha256,
            content_base64,
        },
    )
}

#[tauri::command]
fn file_edit(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
    expected_sha256: String,
    content: String,
) -> Result<(), String> {
    let mut runtime = state
        .runtime
        .try_lock()
        .map_err(|_| "A task is running; saves are paused until it finishes.".to_owned())?;
    let mut projects = state
        .projects
        .lock()
        .expect("project service lock poisoned");
    gated_file_save(
        &mut runtime,
        &mut projects,
        &project_id,
        &FileSaveOp::Edit {
            path,
            expected_sha256,
            content,
        },
    )
}

#[tauri::command]
fn file_delete(
    state: tauri::State<'_, AppState>,
    project_id: String,
    path: String,
    expected_sha256: Option<String>,
) -> Result<(), String> {
    let mut runtime = state
        .runtime
        .try_lock()
        .map_err(|_| "A task is running; saves are paused until it finishes.".to_owned())?;
    let mut projects = state
        .projects
        .lock()
        .expect("project service lock poisoned");
    gated_file_save(
        &mut runtime,
        &mut projects,
        &project_id,
        &FileSaveOp::Delete {
            path,
            expected_sha256,
        },
    )
}

#[tauri::command]
fn file_search(
    state: tauri::State<'_, AppState>,
    project_id: String,
    query: String,
    mode: String,
) -> Result<Vec<lumi_project::SearchHit>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .file_search(&project_id, &query, &mode)
}

#[tauri::command]
fn git_status(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Option<lumi_project::GitStatus>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .git_status(&project_id)
}

#[tauri::command]
fn git_log(
    state: tauri::State<'_, AppState>,
    project_id: String,
    limit: usize,
) -> Result<Vec<lumi_project::CommitInfo>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .git_log(&project_id, limit)
}

#[tauri::command]
fn git_branches(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<String>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .git_branches(&project_id)
}

#[tauri::command]
fn git_create_branch(
    state: tauri::State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<(), String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .git_create_branch(&project_id, &name)
}

#[tauri::command]
fn git_switch(
    state: tauri::State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<(), String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .git_switch(&project_id, &name)
}

#[tauri::command]
fn git_commit(
    state: tauri::State<'_, AppState>,
    project_id: String,
    message: String,
    paths: Vec<String>,
) -> Result<String, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .git_commit(&project_id, &message, &paths)
}

#[tauri::command]
fn task_create(
    state: tauri::State<'_, AppState>,
    project_id: String,
    goal: String,
) -> Result<lumi_protocol::Task, String> {
    let projects = state
        .projects
        .lock()
        .expect("project service lock poisoned");
    let mut runtime = state
        .runtime
        .lock()
        .expect("desktop runtime lock poisoned");
    projects.task_create(&mut runtime, &project_id, &goal)
}

#[tauri::command]
fn task_list(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<lumi_protocol::Task>, String> {
    let projects = state
        .projects
        .lock()
        .expect("project service lock poisoned");
    let mut runtime = state
        .runtime
        .lock()
        .expect("desktop runtime lock poisoned");
    projects.task_list(&mut runtime, &project_id)
}

#[tauri::command]
fn validation_run(
    state: tauri::State<'_, AppState>,
    project_id: String,
    task_id: String,
    command: String,
) -> Result<lumi_project::ValidationRecord, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .validation_run(&project_id, &task_id, &command)
}

#[tauri::command]
fn change_set(
    state: tauri::State<'_, AppState>,
    project_id: String,
    task_id: String,
) -> Result<lumi_project::ChangeSet, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .change_set(&project_id, &task_id)
}

#[tauri::command]
fn change_sets(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<lumi_project::ChangeSet>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .change_sets(&project_id)
}

#[tauri::command]
fn project_clone(
    state: tauri::State<'_, AppState>,
    source: String,
    destination_parent: String,
    display_name: Option<String>,
) -> Result<lumi_desktop::OpenedProject, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .clone_repository(&source, &destination_parent, display_name)
}

#[tauri::command]
fn memory_remember(
    state: tauri::State<'_, AppState>,
    project_id: String,
    submission: lumi_desktop::MemorySubmission,
) -> Result<(), String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .memory_remember(&project_id, &submission)
}

#[tauri::command]
fn memory_list(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<(lumi_project::MemoryRecord, bool)>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .memory_list(&project_id)
}

#[tauri::command]
fn memory_invalidate(
    state: tauri::State<'_, AppState>,
    project_id: String,
    memory_id: String,
    reason: String,
) -> Result<(), String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .memory_invalidate(&project_id, &memory_id, &reason)
}

#[tauri::command]
fn artifacts_list(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<lumi_project::ArtifactEntry>, String> {
    state
        .projects
        .lock()
        .expect("project service lock poisoned")
        .artifacts_list(&project_id)
}


// ===== Evidence read model (audit ledger → Evidence tab) =====

/// Evidence-tab payload. `running` is true while a task worker holds the
/// runtime lock: the ledger is being written right now, so entries are
/// withheld rather than shown stale.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceDto {
    pub running: bool,
    pub entries: Vec<lumi_desktop::api::EvidenceSummaryEntry>,
}

#[tauri::command]
fn project_evidence(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<EvidenceDto, String> {
    let project_id =
        lumi_protocol::ProjectId::parse(&project_id).map_err(|e| e.to_string())?;
    match state.runtime.try_lock() {
        Ok(runtime) => Ok(EvidenceDto {
            running: false,
            entries: runtime.project_evidence(&project_id)?,
        }),
        // A run holds the lock: the ledger is mid-write.
        Err(_) => Ok(EvidenceDto {
            running: true,
            entries: Vec::new(),
        }),
    }
}

// ===== Task execution (Work mode, wired to the real planning loop) =====
//
// The provider credential lives in AppState memory for the app session
// only — never persisted, never logged. Runs are single-flight: one
// worker holds the runtime mutex for the run's duration while the rest
// of the shell degrades to lock-free reads and honest "executing"
// snapshots.

/// Redacted provider config for the UI (never includes the key).
#[derive(Debug, Clone, Serialize)]
pub struct ProviderConfigDto {
    pub configured: bool,
    pub family: Option<String>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub remembered: bool,
}

#[tauri::command]
fn provider_get_config(state: tauri::State<'_, AppState>) -> ProviderConfigDto {
    let provider = state.provider.lock().expect("provider lock poisoned");
    match provider.as_ref() {
        Some(session) => ProviderConfigDto {
            configured: true,
            family: Some(session.family.clone()),
            endpoint: Some(session.endpoint.clone()),
            model: Some(session.model.clone()),
            remembered: state.provider_remembered.load(Ordering::SeqCst),
        },
        None => ProviderConfigDto {
            configured: false,
            family: None,
            endpoint: None,
            model: None,
            remembered: false,
        },
    }
}

#[tauri::command]
fn provider_set_config(
    state: tauri::State<'_, AppState>,
    family: String,
    endpoint: String,
    model: String,
    api_key: String,
    remember: bool,
) -> Result<ProviderConfigDto, String> {
    let family = family.trim().to_lowercase();
    if !matches!(family.as_str(), "openai" | "openai-compatible") {
        return Err(format!("unsupported provider family {family:?} (use openai or openai-compatible)"));
    }
    let endpoint = endpoint.trim().trim_end_matches('/').to_owned();
    if endpoint.is_empty() || model.trim().is_empty() || api_key.trim().is_empty() {
        return Err("endpoint, model, and API key are all required".to_owned());
    }
    let session = ProviderSession {
        family,
        endpoint,
        model: model.trim().to_owned(),
        api_key: api_key.trim().to_owned(),
    };
    lumi_desktop::provider_storage::validate_provider(&session)?;
    if remember {
        lumi_desktop::provider_storage::remember_os_provider(&session)?;
    } else if state.provider_remembered.load(Ordering::SeqCst) {
        lumi_desktop::provider_storage::forget_os_provider()?;
    }
    let dto = ProviderConfigDto {
        configured: true,
        family: Some(session.family.clone()),
        endpoint: Some(session.endpoint.clone()),
        model: Some(session.model.clone()),
        remembered: remember,
    };
    *state.provider.lock().expect("provider lock poisoned") = Some(session);
    state.provider_remembered.store(remember, Ordering::SeqCst);
    Ok(dto)
}

#[tauri::command]
fn provider_clear_config(state: tauri::State<'_, AppState>) -> Result<ProviderConfigDto, String> {
    if state.provider_remembered.load(Ordering::SeqCst) {
        lumi_desktop::provider_storage::forget_os_provider()?;
    }
    *state.provider.lock().expect("provider lock poisoned") = None;
    state.provider_remembered.store(false, Ordering::SeqCst);
    Ok(ProviderConfigDto {
        configured: false,
        family: None,
        endpoint: None,
        model: None,
        remembered: false,
    })
}

#[tauri::command]
fn task_run(state: tauri::State<'_, AppState>, task_id: String) -> Result<engagement_commands::TaskRunDto, String> {
    engagement_commands::start_task(&state, &task_id)
}


// ===== Project connections (Spec 29) =====
//
// Metadata via ConnectionsStore (JSON beside runtime state); credential
// values via the secrets broker (OS keyring). The UI never receives or
// persists credential values — only references.

#[tauri::command]
fn connections_list(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ConnectionRecord>, String> {
    Ok(state.connections.lock().expect("connections lock poisoned").list(&project_id))
}

#[tauri::command]
fn connections_connect(
    state: tauri::State<'_, AppState>,
    project_id: String,
    name: String,
    kind: String,
    endpoint: String,
    credential: String,
) -> Result<ConnectionRecord, String> {
    let connections = state.connections.lock().expect("connections lock poisoned");
    connections.connect(
        &state.broker,
        &project_id,
        &name,
        &kind,
        &endpoint,
        &credential,
    )
}

#[tauri::command]
fn connections_verify(
    state: tauri::State<'_, AppState>,
    project_id: String,
    connection_id: String,
) -> Result<bool, String> {
    let connections = state.connections.lock().expect("connections lock poisoned");
    connections.verify(&state.broker, &project_id, &connection_id)
}

#[tauri::command]
fn connections_disconnect(
    state: tauri::State<'_, AppState>,
    project_id: String,
    connection_id: String,
) -> Result<bool, String> {
    state
        .connections
        .lock()
        .expect("connections lock poisoned")
        .disconnect(&state.broker, &project_id, &connection_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            // The single-user desktop runs under one fixed local tenant so
            // project-bound tasks and snapshots share one scope (spec 26).
            let local_tenant = lumi_protocol::TenantId::parse(DesktopRuntime::LOCAL_TENANT)
                .map_err(|e| std::io::Error::other(format!("local tenant id: {e}")))?;
            let mut runtime = DesktopRuntime::new_for_tenant(
                state_dir.join("runtime-state.json"),
                local_tenant,
            )
            .map_err(std::io::Error::other)?;
            // A prior session may have died mid-run. Re-arm those tasks
            // (RUNNING → FAILED with a crash envelope) before anything
            // can execute again, so the user can explicitly re-run them.
            match runtime.recover_interrupted_tasks() {
                Ok(recovered) if !recovered.is_empty() => {
                    log::info!(
                        "recovered {} task(s) interrupted by a previous session",
                        recovered.len()
                    );
                }
                Ok(_) => {}
                Err(error) => return Err(std::io::Error::other(error.to_string()).into()),
            }
            let projects = ProjectService::open(&state_dir).map_err(std::io::Error::other)?;
            let mut state = AppState::new(runtime, projects, state_dir);
            match lumi_desktop::provider_storage::restore_os_provider() {
                Ok(Some(session)) => {
                    *state
                        .provider
                        .lock()
                        .map_err(|_| std::io::Error::other("provider lock poisoned"))? = Some(session);
                    state.provider_remembered.store(true, Ordering::SeqCst);
                }
                Ok(None) => {}
                Err(error) => log::warn!("provider recovery unavailable: {error}"),
            }
            state.browser_config = lumi_desktop::browser_tools::BrowserConfig::discover(
                app.path().resource_dir().ok().as_deref(),
            );
            app.manage(state);
            engagement_commands::start_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_kill_switch,
            emergency_stop,
            get_operations_snapshot,
            project_open_folder,
            project_list_recent,
            project_overview,
            project_relink,
            project_remove,
            file_list,
            file_read,
            file_read_base64,
            file_create,
            file_edit,
            file_delete,
            file_search,
            git_status,
            git_log,
            git_branches,
            git_create_branch,
            git_switch,
            git_commit,
            task_create,
            task_list,
            validation_run,
            change_set,
            change_sets,
            project_clone,
            memory_remember,
            memory_list,
            memory_invalidate,
            artifacts_list,
            project_evidence,
            connections_list,
            connections_connect,
            connections_disconnect,
            connections_verify,
            file_create_base64,
            file_edit_base64,
            provider_get_config,
            provider_set_config,
            provider_clear_config,
            task_run,
            engagement_snapshot,
            engagement_check_browser,
            engagement_set_browser_origins,
            engagement_create_task,
            automation_list,
            automation_preview,
            automation_save,
            automation_toggle,
            automation_delete,
            automation_run_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_uses_the_shared_runtime_token() {
        let path =
            std::env::temp_dir().join(format!("lumi-desktop-app-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("stopped"));
        let runtime = DesktopRuntime::new(path.clone()).unwrap();
        let token = runtime.cancellation_token();
        let projects_dir = std::env::temp_dir().join(format!(
            "lumi-desktop-app-projects-{}",
            std::process::id()
        ));
        let projects = ProjectService::open(&projects_dir).unwrap();
        let state = AppState::new(runtime, projects, projects_dir.clone());
        assert!(!state.cancellation_token().is_cancelled());
        let _ = std::fs::remove_dir_all(&projects_dir);
        state.cancel.cancel();
        assert!(token.is_cancelled());
        assert_eq!(kill_switch_dto(&token).state, KillSwitchState::Stopped);
        assert!(!kill_switch_dto(&token).running);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("stopped"));
    }

    #[test]
    fn initial_operations_view_is_unknown() {
        let path = std::env::temp_dir().join(format!(
            "lumi-desktop-app-unknown-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("stopped"));
        let projects_dir = std::env::temp_dir().join(format!(
            "lumi-desktop-app-projects-unknown-{}",
            std::process::id()
        ));
        let projects = ProjectService::open(&projects_dir).unwrap();
        let state = AppState::new(
            DesktopRuntime::new(path.clone()).unwrap(),
            projects,
            path.with_extension("connections"),
        );
        let snapshot = state.runtime.lock().unwrap().snapshot();
        assert_eq!(
            snapshot.connection,
            lumi_desktop::ConnectionState::Connected
        );
        assert_eq!(
            snapshot.execution,
            lumi_desktop::ExecutionState::Unavailable
        );
        assert_eq!(snapshot.queue, Some(vec![]));
        assert!(snapshot.progress.is_none());
        assert!(snapshot.pending_approvals.is_none());
        assert!(snapshot.economics.is_none());
        assert!(snapshot.permissions.is_none());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("stopped"));
        let _ = std::fs::remove_dir_all(&projects_dir);
    }
}
