//! Lumi desktop app: a thin viewport onto runtime-owned state.
//!
//! The shell reads a durable local runtime snapshot. Categories the runtime
//! cannot prove remain unknown; the UI cannot create policy, credentials,
//! approvals, or executor state.

use lumi_desktop::{DesktopRuntime, KillSwitchState, OperationsSnapshot};
use lumi_state::CancelToken;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

/// Shared application state managed by Tauri.
pub struct AppState {
    /// Durable local runtime and its real orchestrator cancellation token.
    pub runtime: Mutex<DesktopRuntime>,
    pub cancel: CancelToken,
    pub stop_marker: std::path::PathBuf,
}

impl AppState {
    #[must_use]
    pub fn new(runtime: DesktopRuntime) -> Self {
        let cancel = runtime.cancellation_token();
        let stop_marker = runtime.stop_marker_path().to_path_buf();
        Self {
            runtime: Mutex::new(runtime),
            cancel,
            stop_marker,
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
    state
        .runtime
        .lock()
        .expect("desktop runtime lock poisoned")
        .snapshot()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let runtime = DesktopRuntime::new(state_dir.join("runtime-state.json"))
                .map_err(std::io::Error::other)?;
            app.manage(AppState::new(runtime));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_kill_switch,
            emergency_stop,
            get_operations_snapshot,
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
        let state = AppState::new(runtime);
        assert!(!state.cancellation_token().is_cancelled());
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
        let state = AppState::new(DesktopRuntime::new(path.clone()).unwrap());
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
    }
}
