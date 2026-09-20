//! Desktop runner end-to-end certification: a persisted project-bound
//! task runs through the REAL DesktopRuntime orchestrator (deny-by-
//! default until the runner wires project-scoped grants), executes on a
//! real git repository, and lands durably — with the fixture planner
//! standing in only for the model provider.

use lumi_agent::{FixturePlanner, TaskRunStatus};
use lumi_desktop::{is_runnable, run_desktop_task, DesktopRuntime, ProjectTaskSpec};
use lumi_models::request::ModelMessage;
use lumi_protocol::{TaskStatus, TenantId, Timestamp};
use lumi_state::StateStore as _;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn depot(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-desktop-runner-{tag}-{}-{}",
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

fn git(repo: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()
        .expect("git available");
    assert!(status.success(), "git {args:?} failed");
}

fn seed_repo(source: &std::path::Path) {
    std::fs::create_dir_all(source).unwrap();
    std::fs::write(
        source.join("README.md"),
        "# Ledger Project\n\nQuarterly reconciliation lives here.\n",
    )
    .unwrap();
    git(source, &["init", "-q"]);
    git(
        source,
        &[
            "-c",
            "user.email=r@example.test",
            "-c",
            "user.name=R",
            "add",
            "-A",
        ],
    );
    git(
        source,
        &[
            "-c",
            "user.email=r@example.test",
            "-c",
            "user.name=R",
            "commit",
            "-q",
            "-m",
            "seed",
        ],
    );
}

const SCENARIO: &str = r##"{
  "scenario": "desktop-note",
  "description": "Runner e2e scenario.",
  "goal": "Read README.md, write notes/desktop-run.md.",
  "turns": [
    { "type": "tool_calls", "calls": [
      { "name": "read_file", "arguments": { "path": "README.md" } }
    ]},
    { "type": "tool_calls", "calls": [
      { "name": "write_file", "arguments": {
          "path": "notes/desktop-run.md",
          "content": "# Desktop run\n\nExecuted through the desktop gate.\n" } }
    ]},
    { "type": "tool_calls", "calls": [
      { "name": "run_shell", "arguments": { "command": "git", "args": ["status", "--porcelain"] } }
    ]},
    { "type": "answer", "content": "Note written and verified with git status." }
  ],
  "expected_files": ["notes/desktop-run.md"]
}"##;

#[test]
fn desktop_runner_executes_persisted_task_on_a_real_repo() {
    let guard = DepotGuard(depot("ok"));
    let repo = guard.0.join("repo");
    seed_repo(&repo);
    let state_path = guard.0.join("runtime-state.json");

    let mut runtime = lumi_desktop::DesktopRuntime::new_for_tenant(
        state_path,
        TenantId::parse(DesktopRuntime::LOCAL_TENANT).unwrap(),
    )
    .unwrap();
    let spec = ProjectTaskSpec {
        tenant_id: TenantId::parse(DesktopRuntime::LOCAL_TENANT).unwrap(),
        principal: lumi_agent::workflow_principal(DesktopRuntime::LOCAL_TENANT, "u-desktop"),
        goal: "Read README.md, write notes/desktop-run.md.".to_owned(),
        project_id: lumi_protocol::ProjectId::generate(),
        environment_id: lumi_protocol::EnvironmentId::generate(),
        workspace_root: repo.display().to_string(),
        workspace_kind: lumi_protocol::WorkspaceKind::ProjectRoot,
    };
    let task = runtime.create_project_task(&spec).unwrap();
    assert_eq!(task.status, TaskStatus::Created);
    assert!(is_runnable(&task.status));

    let fixture = lumi_agent::parse_fixture(SCENARIO).unwrap();
    let planner = FixturePlanner::new(fixture);
    let outcome = run_desktop_task(&mut runtime, &task, &planner).unwrap();

    assert_eq!(outcome.status, TaskRunStatus::Completed, "{outcome:?}");
    assert!(
        repo.join("notes/desktop-run.md").is_file(),
        "the work must exist in the project repository"
    );

    // Durable outcome: task COMPLETED, run closed, audit intact.
    let persisted = runtime.load_task(&task.task_id).unwrap();
    assert_eq!(persisted.status, TaskStatus::Completed);
    assert!(matches!(
        runtime.orchestrator.audit.verify_chain(),
        lumi_audit::ChainVerification::Intact { .. }
    ));

    // The Evidence read model surfaces the run's gated actions with
    // trust labels from the ledger — write_file verified, shell executed.
    let evidence = runtime.project_evidence(&spec.project_id).unwrap();
    assert!(
        evidence
            .iter()
            .any(|entry| entry.operation == "write_file" && entry.trust_label == "Verified"),
        "evidence must contain the verified write: {evidence:?}"
    );
    assert!(
        evidence
            .iter()
            .any(|entry| entry.operation == "run_shell" && entry.trust_label == "Verified"),
        "shell action evidence present: {evidence:?}"
    );
}

#[test]
fn desktop_runner_refuses_unrunnable_and_unbound_tasks() {
    let guard = DepotGuard(depot("refuse"));
    let repo = guard.0.join("repo");
    seed_repo(&repo);
    let state_path = guard.0.join("runtime-state.json");
    let mut runtime = lumi_desktop::DesktopRuntime::new_for_tenant(
        state_path,
        TenantId::parse(DesktopRuntime::LOCAL_TENANT).unwrap(),
    )
    .unwrap();

    let spec = ProjectTaskSpec {
        tenant_id: TenantId::parse(DesktopRuntime::LOCAL_TENANT).unwrap(),
        principal: lumi_agent::workflow_principal(DesktopRuntime::LOCAL_TENANT, "u-desktop"),
        goal: "read only".to_owned(),
        project_id: lumi_protocol::ProjectId::generate(),
        environment_id: lumi_protocol::EnvironmentId::generate(),
        workspace_root: repo.display().to_string(),
        workspace_kind: lumi_protocol::WorkspaceKind::ProjectRoot,
    };
    let task = runtime.create_project_task(&spec).unwrap();

    // A RUNNING record belongs to a live/crashed prior run — never blind-rerun.
    let mut running = task.clone();
    running.status = TaskStatus::Running;
    assert!(!is_runnable(&running.status));
    let planner = FixturePlanner::new(lumi_agent::parse_fixture(SCENARIO).unwrap());
    let err = run_desktop_task(&mut runtime, &running, &planner).unwrap_err();
    assert!(err.contains("RUNNING"), "{err}");
}

#[test]
fn desktop_runner_recovers_a_task_interrupted_by_restart() {
    let guard = DepotGuard(depot("interrupted"));
    let repo = guard.0.join("repo");
    seed_repo(&repo);
    let state_path = guard.0.join("runtime-state.json");
    let tenant = TenantId::parse(DesktopRuntime::LOCAL_TENANT).unwrap();

    // Session A: create the task and fabricate the durable state a crash
    // mid-run leaves behind — task RUNNING, run record open.
    let task_id = {
        let mut runtime =
            lumi_desktop::DesktopRuntime::new_for_tenant(state_path.clone(), tenant.clone())
                .unwrap();
        let spec = ProjectTaskSpec {
            tenant_id: tenant.clone(),
            principal: lumi_agent::workflow_principal(DesktopRuntime::LOCAL_TENANT, "u-desktop"),
            goal: "Read README.md, write notes/desktop-run.md.".to_owned(),
            project_id: lumi_protocol::ProjectId::generate(),
            environment_id: lumi_protocol::EnvironmentId::generate(),
            workspace_root: repo.display().to_string(),
            workspace_kind: lumi_protocol::WorkspaceKind::ProjectRoot,
        };
        let task = runtime.create_project_task(&spec).unwrap();
        let mut running = task.clone();
        running.status = TaskStatus::Running;
        runtime.save_task(&running).unwrap();
        runtime
            .orchestrator
            .store
            .save_run(&lumi_protocol::Run {
                run_id: lumi_protocol::RunId::generate(),
                task_id: task.task_id.clone(),
                runtime_version: "0.1.0".to_owned(),
                workflow_version: None,
                selected_providers: vec!["fixture".to_owned()],
                started_at: Timestamp::now(),
                ended_at: None,
                state: lumi_protocol::RunState::Executing,
                budgets_consumed: lumi_protocol::ConsumedBudget::default(),
                failure: None,
            })
            .unwrap();
        task.task_id
    };
    // Session A's runtime was dropped above — that IS the restart.

    // Session B: restart over the same durable state. Startup recovery
    // must re-arm the interrupted task instead of leaving it RUNNING.
    let mut runtime_b =
        lumi_desktop::DesktopRuntime::new_for_tenant(state_path.clone(), tenant.clone()).unwrap();
    let recovered = runtime_b.recover_interrupted_tasks().unwrap();
    assert_eq!(recovered.len(), 1, "the stale RUNNING task is recovered");
    assert_eq!(recovered[0].status, TaskStatus::Failed);
    assert!(is_runnable(&recovered[0].status), "FAILED re-arms the task");

    // The recovered task re-runs through the real gate to completion.
    let task_b = runtime_b.load_task(&task_id).unwrap();
    assert_eq!(task_b.status, TaskStatus::Failed);
    let planner = FixturePlanner::new(lumi_agent::parse_fixture(SCENARIO).unwrap());
    let outcome = run_desktop_task(&mut runtime_b, &task_b, &planner).unwrap();
    assert_eq!(outcome.status, TaskRunStatus::Completed, "{outcome:?}");
    assert!(repo.join("notes/desktop-run.md").is_file());
    assert_eq!(
        runtime_b.load_task(&task_id).unwrap().status,
        TaskStatus::Completed
    );

    // The interrupted attempt's run record stays closed with its crash
    // envelope — the failure is visible, never silently rewritten.
    let persisted = runtime_b.orchestrator.store.read().unwrap();
    let interrupted: Vec<_> = persisted
        .runs
        .iter()
        .filter(|run| {
            run.task_id == task_id
                && run
                    .failure
                    .as_ref()
                    .is_some_and(|f| f.message.contains("interrupted"))
        })
        .collect();
    assert_eq!(interrupted.len(), 1);
    assert!(interrupted[0].ended_at.is_some());
}

#[test]
fn desktop_runner_rerun_carries_durable_progress_from_the_prior_attempt() {
    let guard = DepotGuard(depot("resume"));
    let repo = guard.0.join("repo");
    seed_repo(&repo);
    let state_path = guard.0.join("runtime-state.json");
    let tenant = TenantId::parse(DesktopRuntime::LOCAL_TENANT).unwrap();

    let mut runtime =
        lumi_desktop::DesktopRuntime::new_for_tenant(state_path, tenant.clone()).unwrap();
    let spec = ProjectTaskSpec {
        tenant_id: tenant.clone(),
        principal: lumi_agent::workflow_principal(DesktopRuntime::LOCAL_TENANT, "u-desktop"),
        goal: "Read README.md, write notes/desktop-run.md.".to_owned(),
        project_id: lumi_protocol::ProjectId::generate(),
        environment_id: lumi_protocol::EnvironmentId::generate(),
        workspace_root: repo.display().to_string(),
        workspace_kind: lumi_protocol::WorkspaceKind::ProjectRoot,
    };
    let project_id = spec.project_id.clone();
    let task = runtime.create_project_task(&spec).unwrap();

    // Attempt 1: interrupted after the read (fixture exhausted mid-run).
    let truncated = r##"{
        "scenario": "truncated",
        "goal": "Read README.md, write notes/desktop-run.md.",
        "turns": [
            { "type": "tool_calls", "calls": [
                { "name": "read_file", "arguments": { "path": "README.md" } }
            ]}
        ]
    }"##;
    let planner = FixturePlanner::new(lumi_agent::parse_fixture(truncated).unwrap());
    let first = run_desktop_task(&mut runtime, &task, &planner).unwrap();
    assert_eq!(first.status, TaskRunStatus::Failed);
    assert!(is_runnable(&TaskStatus::Failed), "failed re-arms");

    // Attempt 2 (after re-arm): the planner must RECEIVE the previous
    // attempt's verified progress, then finish the remaining work.
    let full = lumi_agent::parse_fixture(SCENARIO).unwrap();
    let planner = FixturePlanner::new(full);
    let second = run_desktop_task(&mut runtime, &task, &planner).unwrap();
    assert_eq!(second.status, TaskRunStatus::Completed, "{second:?}");

    let received = planner.received();
    let first_user = received
        .first()
        .and_then(|turn| {
            turn.iter().find_map(|m| match m {
                ModelMessage::User { content } => Some(content.clone()),
                _ => None,
            })
        })
        .unwrap_or_default();
    assert!(
        first_user.contains("DURABLE PROGRESS FROM THE PREVIOUS ATTEMPT"),
        "re-run must carry durable progress: {first_user}"
    );
    assert!(
        first_user.contains("read_file"),
        "progress must name the verified prior action: {first_user}"
    );

    // The work completed and is inspectable in the evidence read model.
    assert!(repo.join("notes/desktop-run.md").is_file());
    let evidence = runtime.project_evidence(&project_id).unwrap();
    assert!(
        evidence
            .iter()
            .any(|entry| entry.operation == "write_file" && entry.trust_label == "Verified"),
        "both attempts' verified actions surface as evidence: {evidence:?}"
    );
}

#[test]
fn ui_file_saves_traverse_the_gate_and_verify() {
    let guard = DepotGuard(depot("gated-save"));
    let repo = guard.0.join("repo");
    seed_repo(&repo);
    let state_path = guard.0.join("runtime-state.json");
    let tenant = TenantId::parse(DesktopRuntime::LOCAL_TENANT).unwrap();

    let mut runtime =
        lumi_desktop::DesktopRuntime::new_for_tenant(state_path.clone(), tenant.clone()).unwrap();
    let projects_dir = guard.0.join("projects");
    std::fs::create_dir_all(&projects_dir).unwrap();
    let mut projects = lumi_desktop::ProjectService::open(&projects_dir).unwrap();
    let opened = projects
        .open_folder(repo.to_str().unwrap(), Some("Gate Repo".to_owned()))
        .unwrap();
    let project_id = opened.project.project_id.clone();

    // Create through the gate.
    lumi_desktop::gated_file_save(
        &mut runtime,
        &mut projects,
        &project_id,
        &lumi_desktop::FileSaveOp::Create {
            path: "docs/gated.txt".to_owned(),
            content: "gated save".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.join("docs/gated.txt")).unwrap(),
        "gated save"
    );

    // Edit through the gate with the correct checksum.
    lumi_desktop::gated_file_save(
        &mut runtime,
        &mut projects,
        &project_id,
        &lumi_desktop::FileSaveOp::Edit {
            path: "docs/gated.txt".to_owned(),
            expected_sha256: lumi_protocol::canonical::sha256_hex(b"gated save"),
            content: "gated save v2".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.join("docs/gated.txt")).unwrap(),
        "gated save v2"
    );

    // Stale checksum: the executor refuses, the run fails honestly, and
    // the file keeps its on-disk (newer) content.
    let err = lumi_desktop::gated_file_save(
        &mut runtime,
        &mut projects,
        &project_id,
        &lumi_desktop::FileSaveOp::Edit {
            path: "docs/gated.txt".to_owned(),
            expected_sha256: "stale".to_owned(),
            content: "lost write".to_owned(),
        },
    )
    .unwrap_err();
    assert!(err.contains("stale write"), "{err}");
    assert_eq!(
        std::fs::read_to_string(repo.join("docs/gated.txt")).unwrap(),
        "gated save v2",
        "the newer on-disk content wins (fail closed)"
    );

    // Durable trail: every save has a task/run record and the ledger
    // verified each successful write.
    let persisted = runtime.orchestrator.store.read().unwrap();
    let save_tasks = persisted
        .tasks
        .iter()
        .filter(|t| {
            t.goal.starts_with("Edit docs/gated.txt") || t.goal.starts_with("Create docs/gated.txt")
        })
        .count();
    assert!(save_tasks >= 3, "each save is its own durable task");
    let project_id = lumi_protocol::ProjectId::parse(&opened.project.project_id).unwrap();
    let evidence = runtime.project_evidence(&project_id).unwrap();
    assert!(
        evidence
            .iter()
            .any(|e| e.operation == "edit_file" && e.trust_label == "Verified"),
        "gated edits surface as verified evidence: {evidence:?}"
    );
    assert!(matches!(
        runtime.orchestrator.audit.verify_chain(),
        lumi_audit::ChainVerification::Intact { .. }
    ));
}
