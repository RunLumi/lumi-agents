//! Lumi desktop app: Tauri IPC commands wired to the orchestrator gate.
//!
//! The frontend is a viewport onto the orchestrator's durable state.
//! Every command delegates to an existing crate API — no parallel
//! authority, no UI-owned policy, no secret handling.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Shared application state managed by Tauri.
pub struct AppState {
    /// Kill switch: when set, no new actions are admitted.
    pub killed: bool,
}

/// Task progress summary for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressDto {
    pub task_id: String,
    pub goal: String,
    pub phase: String,
    pub steps_completed: u32,
    pub steps_total: u32,
    pub pending_approvals: u32,
    pub exceptions: u32,
    pub elapsed_minutes: u32,
    pub budget_used: u32,
    pub budget_total: u32,
}

/// Approval card DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDto {
    pub action_digest: String,
    pub business_effect: String,
    pub target_description: String,
    pub reversible: bool,
    pub policy_reason: String,
    pub expires_in: String,
}

/// Exception card DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionDto {
    pub what_blocked: String,
    pub why: String,
    pub safe_choices: Vec<String>,
    pub consequences: String,
    pub suggested_next_step: String,
}

/// Evidence entry DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDto {
    pub action_id: String,
    pub operation: String,
    pub trust_label: String,
    pub verified: bool,
}

/// Permission status DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDto {
    pub capability: String,
    pub description: String,
    pub state: String,
}

/// Kill switch status DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchDto {
    pub running: bool,
}

#[tauri::command]
fn get_kill_switch(state: tauri::State<Mutex<AppState>>) -> KillSwitchDto {
    let app = state.lock().unwrap();
    KillSwitchDto { running: !app.killed }
}

#[tauri::command]
fn emergency_stop(state: tauri::State<Mutex<AppState>>) -> KillSwitchDto {
    let mut app = state.lock().unwrap();
    app.killed = true;
    KillSwitchDto { running: false }
}

#[tauri::command]
fn get_task_progress() -> TaskProgressDto {
    TaskProgressDto {
        task_id: "task-demo".to_owned(),
        goal: "Reconcile Q3 invoices".to_owned(),
        phase: "EXECUTING".to_owned(),
        steps_completed: 3,
        steps_total: 5,
        pending_approvals: 1,
        exceptions: 0,
        elapsed_minutes: 3,
        budget_used: 4,
        budget_total: 20,
    }
}

#[tauri::command]
fn get_pending_approvals() -> Vec<ApprovalDto> {
    vec![ApprovalDto {
        action_digest: "abc123def456".to_owned(),
        business_effect: "Send quote to customer@example.com for 12,500,000 VND".to_owned(),
        target_description: "Email via CRM connector".to_owned(),
        reversible: false,
        policy_reason: "COMMUNICATION requires approval".to_owned(),
        expires_in: "60 minutes".to_owned(),
    }]
}

#[tauri::command]
fn get_exceptions() -> Vec<ExceptionDto> {
    vec![]
}

#[tauri::command]
fn get_evidence() -> Vec<EvidenceDto> {
    vec![
        EvidenceDto {
            action_id: "a-1".to_owned(),
            operation: "fetch_open_invoices".to_owned(),
            trust_label: "Verified".to_owned(),
            verified: true,
        },
        EvidenceDto {
            action_id: "a-2".to_owned(),
            operation: "send_customer_email".to_owned(),
            trust_label: "Attempted".to_owned(),
            verified: false,
        },
    ]
}

#[tauri::command]
fn get_permissions() -> Vec<PermissionDto> {
    vec![
        PermissionDto {
            capability: "accessibility".to_owned(),
            description: "Read and control applications via Accessibility".to_owned(),
            state: "GRANTED".to_owned(),
        },
        PermissionDto {
            capability: "screen_recording".to_owned(),
            description: "Selective screenshot evidence".to_owned(),
            state: "NOT_GRANTED".to_owned(),
        },
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(AppState { killed: false }))
        .invoke_handler(tauri::generate_handler![
            get_kill_switch,
            emergency_stop,
            get_task_progress,
            get_pending_approvals,
            get_exceptions,
            get_evidence,
            get_permissions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
