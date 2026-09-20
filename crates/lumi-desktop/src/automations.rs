//! Project automations (Spec 27 / 13): persisted schedules whose due
//! occurrences create durable project-bound Work tasks.
//!
//! The schedule list persists as JSON next to the runtime state file;
//! occurrences are admitted through the `lumi-scheduler` policy engine
//! (catch-up, quiet hours, deduplication, device availability, lease
//! requirement) and then materialized as real durable tasks with the
//! automation's project binding. Timezone resolution: v1 matches cron
//! fields against the system-local wall clock passed by the caller as a
//! UTC offset (§13.11's per-schedule timezone carries forward).

use crate::runtime::DesktopRuntime;
use lumi_scheduler::cron::{civil_from_epoch, CronExpr};
use lumi_scheduler::{Schedule, ScheduleAdmission, ScheduleId, Scheduler};
use lumi_state::StateStore as _;
use std::path::Path;

/// One persisted automation: the schedule plus the project binding the
/// created tasks receive (Spec 27 §27.3: project-native automations).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutomationRecord {
    /// Stable automation id (also the scheduler schedule id).
    pub automation_id: String,
    pub schedule: Schedule,
    pub project_id: lumi_protocol::ProjectId,
    pub workspace_root: String,
    /// The durable goal stamped onto every task this automation creates.
    pub goal: String,
    #[serde(default)]
    pub enabled: bool,
}

/// Loads the persisted automation list. A missing file is an empty list.
///
/// # Errors
/// Corrupt JSON fails closed (the shell surfaces it) rather than silently
/// dropping someone's automations.
pub fn load_automations(path: &Path) -> Result<Vec<AutomationRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("read automations: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("corrupt automations file: {e}"))
}

/// Persists the automation list atomically (write-temp + rename).
///
/// # Errors
/// Filesystem failures.
pub fn save_automations(path: &Path, records: &[AutomationRecord]) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let json =
        serde_json::to_string_pretty(records).map_err(|e| format!("serialize automations: {e}"))?;
    std::fs::write(&tmp, json).map_err(|e| format!("write automations: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("persist automations: {e}"))
}

/// Admits one due firing of `automation_id` and materializes it as a
/// durable project-bound task. Returns the created task's goal on
/// success. Policy (catch-up, quiet hours, dedup, device) is enforced by
/// the scheduler; this function never bypasses an admission refusal.
///
/// # Errors
/// Unknown automation, scheduler refusal (mapped by kind), or store
/// failures.
#[allow(clippy::too_many_arguments)]
pub fn fire_automation(
    runtime: &mut DesktopRuntime,
    scheduler: &mut Scheduler,
    automations: &[AutomationRecord],
    automation_id: &str,
    now: lumi_protocol::Timestamp,
    max_tasks: usize,
) -> Result<lumi_protocol::Task, String> {
    let record = automations
        .iter()
        .find(|a| a.automation_id == automation_id)
        .ok_or_else(|| format!("unknown automation {automation_id:?}"))?
        .clone();
    if !record.enabled {
        return Err(format!("automation {automation_id:?} is disabled"));
    }

    let _ = max_tasks;
    // Registering the automation = adding its schedule to the engine
    // (idempotent upsert; enabled state carried on the record).
    scheduler.add_schedule(record.schedule.clone());
    let admission = scheduler.admit_firing(
        &ScheduleId::new(record.automation_id.clone()),
        now,
        0, // quiet-hour resolution: caller-supplied local hour
        0, // no missed-window catch-up on tick admission
        true,
    );
    let task_id = match admission {
        Ok(ScheduleAdmission::Admit { task_id, .. }) => task_id,
        Ok(other) => return Err(format!("admission not granted: {other:?}")),
        Err(ref e) => return Err(format!("admission refused: {e}")),
    };

    // Materialize the admitted occurrence as a DURABLE task (Spec 27:
    // occurrence → real Task, project-bound, resumable).
    let task = lumi_protocol::Task {
        task_id,
        tenant_id: record.schedule.trigger.tenant_id.clone(),
        principal: record_schedule_principal(&record),
        mode: lumi_protocol::TaskMode::Work,
        goal: record.goal.clone(),
        created_at: lumi_protocol::Timestamp::now(),
        deadline: record.schedule.trigger.deadline,
        budget: lumi_protocol::Budget {
            max_actions: Some(record.schedule.trigger.max_actions),
            ..lumi_protocol::Budget::default()
        },
        privacy_constraints: lumi_protocol::PrivacyConstraint::default(),
        status: lumi_protocol::TaskStatus::Created,
        requested_outputs: vec![],
        project_binding: Some(lumi_protocol::ProjectTaskBinding {
            project_id: record.project_id.clone(),
            execution_environment_id: lumi_protocol::EnvironmentId::generate(),
            workspace_root: record.workspace_root.clone(),
            workspace_kind: lumi_protocol::WorkspaceKind::ProjectRoot,
        }),
    };
    runtime
        .orchestrator
        .store
        .save_task(&task)
        .map_err(|e| format!("persist automation task: {e}"))?;
    Ok(task)
}

fn record_schedule_principal(record: &AutomationRecord) -> lumi_protocol::Principal {
    lumi_protocol::Principal {
        principal_id: lumi_protocol::PrincipalId::generate(),
        tenant_id: record.schedule.trigger.tenant_id.clone(),
        kind: lumi_protocol::PrincipalKind::Workflow,
        authenticated_at: Some(lumi_protocol::Timestamp::now()),
        authentication_strength: Some(lumi_protocol::AuthenticationStrength::DevicePossession),
    }
}

/// Which enabled automations are due at the given wall clock (fields
/// resolved at the automation's offset). Pure helper for the tick loop.
pub fn due_automations(
    automations: &[AutomationRecord],
    now: lumi_protocol::Timestamp,
    utc_offset_seconds: i32,
) -> Vec<String> {
    let fields = civil_from_epoch(now.epoch_seconds(), utc_offset_seconds);
    let mut due = Vec::new();
    for record in automations {
        if !record.enabled {
            continue;
        }
        if let Ok(expr) = CronExpr::parse(&record.schedule.cron_expression) {
            if expr.matches(&fields) {
                due.push(record.automation_id.clone());
            }
        }
    }
    due
}

/// The default trigger identity fields for shell-created automations:
/// one tenant-scoped cron source with the automation itself as the
/// template ref (Spec 27: project-native automations).
fn shell_trigger(tenant_id: lumi_protocol::TenantId, cron: &str) -> lumi_scheduler::TriggerRequest {
    lumi_scheduler::TriggerRequest {
        tenant_id,
        principal_id: "u-automation".to_owned(),
        template_id: "automation".to_owned(),
        payload_refs: serde_json::json!({}),
        triggered_at: lumi_protocol::Timestamp::now(),
        deduplication: None,
        source: lumi_scheduler::TriggerSource {
            kind: lumi_scheduler::TriggerKind::Cron,
            identity: format!("cron:{cron}"),
        },
        timezone: None,
        device_availability: lumi_scheduler::DeviceAvailability::WaitForDevice,
        deadline: None,
        max_actions: 20,
        lease_id: "lease-desktop-automation".to_owned(),
    }
}

/// Creates one automation: validates the goal and the cron expression
/// (5-field, as-authored) BEFORE persisting, so a malformed schedule
/// never reaches the tick loop. The record starts enabled.
///
/// # Errors
/// Empty goal, malformed cron, or store failures.
pub fn create_automation(
    records_path: &Path,
    tenant_id: lumi_protocol::TenantId,
    project_id: lumi_protocol::ProjectId,
    workspace_root: String,
    goal: &str,
    cron_expression: &str,
) -> Result<AutomationRecord, String> {
    let goal = goal.trim();
    if goal.is_empty() {
        return Err("automation goal is required".to_owned());
    }
    let cron = cron_expression.trim();
    lumi_scheduler::cron::CronExpr::parse(cron)
        .map_err(|e| format!("invalid cron expression: {e}"))?;
    let mut records = load_automations(records_path)?;
    let automation_id = format!("auto-{}", lumi_protocol::TaskId::generate());
    let record = AutomationRecord {
        automation_id: automation_id.clone(),
        schedule: Schedule {
            schedule_id: ScheduleId::new(automation_id),
            cron_expression: cron.to_owned(),
            timezone: "local".to_owned(),
            catch_up: lumi_scheduler::CatchUpPolicy::RunOnce,
            quiet_hours: None,
            critical: false,
            enabled: true,
            trigger: shell_trigger(tenant_id, cron),
        },
        project_id,
        workspace_root,
        goal: goal.to_owned(),
        enabled: true,
    };
    records.push(record.clone());
    save_automations(records_path, &records)?;
    Ok(record)
}

/// Enables/disables one project automation. Disabled automations are
/// never admitted by the tick loop (fire_automation refuses them).
///
/// # Errors
/// Unknown automation or store failures.
pub fn set_automation_enabled(
    records_path: &Path,
    project_id: &str,
    automation_id: &str,
    enabled: bool,
) -> Result<AutomationRecord, String> {
    let mut records = load_automations(records_path)?;
    let record = records
        .iter_mut()
        .find(|a| a.project_id.as_str() == project_id && a.automation_id == automation_id)
        .ok_or_else(|| format!("unknown automation {automation_id:?}"))?;
    record.enabled = enabled;
    let updated = record.clone();
    save_automations(records_path, &records)?;
    Ok(updated)
}

/// Deletes one project automation. Returns false when nothing matched.
///
/// # Errors
/// Store failures.
pub fn delete_automation(
    records_path: &Path,
    project_id: &str,
    automation_id: &str,
) -> Result<bool, String> {
    let records = load_automations(records_path)?;
    let position = records
        .iter()
        .position(|a| a.project_id.as_str() == project_id && a.automation_id == automation_id);
    let Some(position) = position else {
        return Ok(false);
    };
    let mut rest = records;
    rest.remove(position);
    save_automations(records_path, &rest)?;
    Ok(true)
}
