//! Automations wiring certification (Spec 27/13): admitted schedule
//! firings materialize as durable project-bound Work tasks; policy
//! refusals (dedup, disabled, unknown) fail without side effects.

use lumi_desktop::{
    due_automations, fire_automation, load_automations, save_automations, AutomationRecord,
};
use lumi_protocol::{ProjectId, TaskStatus, TenantId, Timestamp};
use lumi_scheduler::{Schedule, ScheduleId, Scheduler};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn depot(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-automations-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct DepotGuard(PathBuf);

impl Drop for DepotGuard {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn automation(id: &str, cron: &str, enabled: bool) -> AutomationRecord {
    AutomationRecord {
        automation_id: id.to_owned(),
        schedule: Schedule {
            schedule_id: ScheduleId::new(id),
            cron_expression: cron.to_owned(),
            timezone: "Asia/Ho_Chi_Minh".to_owned(),
            catch_up: lumi_scheduler::CatchUpPolicy::RunOnce,
            quiet_hours: None,
            critical: false,
            enabled,
            trigger: lumi_scheduler::TriggerRequest {
                tenant_id: TenantId::parse("tenant-automations").unwrap(),
                principal_id: "u-automation".to_owned(),
                template_id: "ledger-summary".to_owned(),
                payload_refs: serde_json::json!({"ledger": "ledger.csv"}),
                triggered_at: Timestamp::UNIX_EPOCH,
                deduplication: None,
                source: lumi_scheduler::TriggerSource {
                    kind: lumi_scheduler::TriggerKind::Cron,
                    identity: format!("cron:{cron}"),
                },
                timezone: None,
                device_availability: lumi_scheduler::DeviceAvailability::WaitForDevice,
                deadline: None,
                max_actions: 20,
                lease_id: "lease-test".to_owned(),
            },
        },
        project_id: ProjectId::generate(),
        workspace_root: "/tmp/does-not-exist".to_owned(),
        goal: "Draft the ledger summary".to_owned(),
        enabled,
    }
}

#[test]
fn admitted_firing_materializes_a_durable_project_task() {
    let guard = DepotGuard(depot("ok"));
    let state_path = guard.0.join("runtime-state.json");
    let mut runtime = lumi_desktop::DesktopRuntime::new_for_tenant(
        state_path.clone(),
        TenantId::parse("tenant-automations").unwrap(),
    )
    .unwrap();
    let mut scheduler = Scheduler::new(true);

    let record = automation("nightly-ledger", "30 6 * * *", true);
    let project_id = record.project_id.clone();
    let automations = vec![record.clone()];
    save_automations(&state_path.with_extension("automations.json"), &automations).unwrap();

    // The saved list round-trips.
    let loaded = load_automations(&state_path.with_extension("automations.json")).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].automation_id, "nightly-ledger");

    // The cron matches at 06:30 local.
    let fields = lumi_scheduler::cron::civil_from_epoch(1_789_947_000, 7 * 3600);
    let expr = lumi_scheduler::cron::CronExpr::parse("30 6 * * *").unwrap();
    assert!(expr.matches(&fields));
    assert_eq!(
        due_automations(
            &automations,
            Timestamp::from_epoch(1_789_947_000, 0).unwrap(),
            7 * 3600
        ),
        vec!["nightly-ledger"]
    );

    let task = fire_automation(
        &mut runtime,
        &mut scheduler,
        &automations,
        "nightly-ledger",
        Timestamp::from_epoch(1_789_947_000, 0).unwrap(),
        20,
    )
    .unwrap();

    // Durable, project-bound, CREATED — resumable through the normal
    // task path, not a ghost admission.
    let persisted = runtime.load_task(&task.task_id).unwrap();
    assert_eq!(persisted.status, TaskStatus::Created);
    assert_eq!(persisted.goal, "Draft the ledger summary");
    let binding = persisted.project_binding.as_ref().unwrap();
    assert_eq!(binding.project_id, project_id);

    // Non-matching cron at this minute: nothing due.
    let due = due_automations(
        &automations,
        Timestamp::from_epoch(1_789_920_660, 0).unwrap(),
        7 * 3600,
    );
    assert!(due.is_empty());
}

#[test]
fn automation_firing_respects_policy_refusals() {
    let guard = DepotGuard(depot("refuse"));
    let state_path = guard.0.join("runtime-state.json");
    let mut runtime = lumi_desktop::DesktopRuntime::new_for_tenant(
        state_path.clone(),
        TenantId::parse("tenant-automations").unwrap(),
    )
    .unwrap();
    let mut scheduler = Scheduler::new(true);

    let mut disabled = automation("disabled-auto", "0 6 * * *", false);
    disabled.enabled = false;
    let automations = vec![disabled.clone()];
    let now = Timestamp::from_epoch(1_789_920_600, 0).unwrap();

    let err = fire_automation(
        &mut runtime,
        &mut scheduler,
        &automations,
        "disabled-auto",
        now,
        20,
    )
    .unwrap_err();
    assert!(err.contains("disabled"), "{err}");

    // Unknown automation id refuses without side effects.
    let err =
        fire_automation(&mut runtime, &mut scheduler, &automations, "ghost", now, 20).unwrap_err();
    assert!(err.contains("unknown"), "{err}");

    // No task was persisted by the refused admissions.
    let persisted = runtime.orchestrator.store.read().unwrap();
    assert!(persisted.tasks.is_empty(), "no side effects on refusal");
}

#[test]
fn automation_crud_roundtrip_validates_cron_and_scopes_per_project() {
    let guard = DepotGuard(depot("crud"));
    let records_path = guard.0.join("automations.json");
    let tenant = TenantId::parse("tenant-automations").unwrap();
    let project_a = ProjectId::generate();
    let project_b = ProjectId::generate();

    // Malformed cron refuses BEFORE persisting anything.
    let err = lumi_desktop::create_automation(
        &records_path,
        tenant.clone(),
        project_a.clone(),
        "/tmp/ws-a".to_owned(),
        "Summarize the ledger",
        "not a cron",
    )
    .unwrap_err();
    assert!(err.contains("invalid cron"), "{err}");
    assert!(
        !records_path.exists(),
        "nothing persisted for a refused create"
    );

    // Empty goal refuses too.
    assert!(lumi_desktop::create_automation(
        &records_path,
        tenant.clone(),
        project_a.clone(),
        "/tmp/ws-a".to_owned(),
        "  ",
        "30 6 * * *",
    )
    .is_err());

    // Valid create round-trips.
    let record = lumi_desktop::create_automation(
        &records_path,
        tenant.clone(),
        project_a.clone(),
        "/tmp/ws-a".to_owned(),
        "Summarize the ledger",
        "30 6 * * *",
    )
    .unwrap();
    assert!(record.enabled);
    assert_eq!(record.goal, "Summarize the ledger");

    // Project scoping: project B sees nothing; disable/delete by id on
    // the wrong project refuse honestly.
    assert!(lumi_desktop::set_automation_enabled(
        &records_path,
        project_b.as_str(),
        &record.automation_id,
        false
    )
    .is_err());
    assert!(!lumi_desktop::delete_automation(
        &records_path,
        project_b.as_str(),
        &record.automation_id
    )
    .unwrap());

    // Disable flips the flag and persists.
    let updated = lumi_desktop::set_automation_enabled(
        &records_path,
        project_a.as_str(),
        &record.automation_id,
        false,
    )
    .unwrap();
    assert!(!updated.enabled);
    let reloaded = lumi_desktop::load_automations(&records_path).unwrap();
    assert_eq!(reloaded.len(), 1);
    assert!(!reloaded[0].enabled);

    // Delete removes.
    assert!(lumi_desktop::delete_automation(
        &records_path,
        project_a.as_str(),
        &record.automation_id
    )
    .unwrap());
    assert!(lumi_desktop::load_automations(&records_path)
        .unwrap()
        .is_empty());
}
