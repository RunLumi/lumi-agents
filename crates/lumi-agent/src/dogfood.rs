//! Real-repo dogfood pass: the delegation loop executed against a real
//! git repository, verified independently of anything the loop claims.
//!
//! The harness stages a throwaway clone of a real repository into a
//! task workspace, creates a durable project-bound task in the same
//! `JsonStateStore` the desktop shell uses, runs the fixture-driven
//! planning loop through the orchestrator gate, and then evaluates the
//! result with its own postcondition checks — file existence, git
//! working-tree evidence, durable transitions, audit-chain integrity,
//! and zero policy denials. The loop's own completion claim is never
//! accepted as proof.
//!
//! Run the CLI: `cargo run -p lumi-evals --bin lumi-dogfood -- \
//!   --repo /path/to/repo --fixture scenario.json --out report.json`

use crate::{
    load_fixture, run_persisted_task, FixturePlanner, Planner, ReadFileTool, RunShellTool,
    TaskRunStatus, WriteFileTool,
};
use lumi_audit::{AuditEventKind, ChainVerification, FixtureEnvironment, PolicyOutcome};
use lumi_orchestrator::{ExecutorDescriptor, Orchestrator, OrchestratorConfig};
use lumi_policy::{CapabilityGrant, GrantSource};
use lumi_protocol::{
    Budget, Capability, ExecutionTier, ProjectId, ProjectTaskBinding, RunId, TaskId, TaskMode,
    TaskStatus, TenantId, Timestamp, WorkspaceKind,
};
use lumi_state::{JsonStateStore, StateStore};
use lumi_workspaces::{CleanupPolicy, Workspace};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One independently evaluated acceptance check.
#[derive(Debug, Clone, Serialize)]
pub struct DogfoodCheck {
    pub check: String,
    pub passed: bool,
    pub detail: String,
}

/// Evidence record for one dogfood run.
#[derive(Debug, Clone, Serialize)]
pub struct DogfoodReport {
    pub scenario: String,
    pub source_repo: String,
    pub goal: String,
    /// True only when EVERY check below passed.
    pub verified: bool,
    pub loop_status: String,
    pub answer: Option<String>,
    pub turns: u32,
    pub actions_executed: u32,
    pub policy_denials: u64,
    pub audit_chain: String,
    pub completed_at_unix: u64,
    /// Set for live-provider runs: the model that planned the work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub checks: Vec<DogfoodCheck>,
}

/// Everything the pass needs; the work directory is caller-owned scratch
/// (the CLI creates and cleans a unique one).
#[derive(Debug, Clone)]
pub struct DogfoodConfig {
    pub source_repo: PathBuf,
    pub fixture_path: PathBuf,
    pub work_dir: PathBuf,
    /// `Some((family, model))` for live-provider runs; recorded on the
    /// report so evaluation evidence names the model that produced it.
    pub provider: Option<(String, String)>,
}

fn git_args(repo: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

fn run_command(mut cmd: Command, what: &str) -> Result<String, String> {
    let output = cmd
        .output()
        .map_err(|e| format!("{what}: spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{what}: exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Executes the dogfood pass with a live model planner (production
/// planner over the caller's provider session/transport). The fixture
/// still supplies the goal brief and expected files; the MODEL plans.
pub fn run_dogfood_live(
    config: &DogfoodConfig,
    planner: &dyn Planner,
) -> Result<DogfoodReport, String> {
    run_dogfood_with_planner(config, planner)
}

/// Executes the dogfood pass with the deterministic fixture planner.
/// `# Errors` — staging or store failures are returned; evaluation
/// failures are REPORTED (verified=false), not errors: a failing product
/// is evidence, not a harness crash.
pub fn run_dogfood(config: &DogfoodConfig) -> Result<DogfoodReport, String> {
    let fixture = load_fixture(&config.fixture_path).map_err(|e| e.to_string())?;
    let planner = FixturePlanner::new(fixture);
    run_dogfood_with_planner(config, &planner)
}

fn run_dogfood_with_planner(
    config: &DogfoodConfig,
    planner: &dyn Planner,
) -> Result<DogfoodReport, String> {
    let fixture = load_fixture(&config.fixture_path).map_err(|e| e.to_string())?;
    let scenario = fixture.scenario.clone();
    let goal = fixture.goal.clone();
    let expected_files = fixture.expected_files.clone();

    let ws_dir = config.work_dir.join("ws");
    std::fs::create_dir_all(&ws_dir).map_err(|e| format!("create work dir: {e}"))?;
    let workspace = Workspace::create(
        &ws_dir,
        &TaskId::generate(),
        "u-dogfood",
        CleanupPolicy::Manual,
    )
    .map_err(|e| format!("create workspace: {e}"))?;

    // Stage a throwaway clone of the REAL repository — the original is
    // never a write target.
    run_command(
        {
            let mut cmd = Command::new("git");
            cmd.args([
                "clone",
                "-q",
                config.source_repo.to_str().ok_or("repo path not utf-8")?,
                workspace
                    .root()
                    .join("repo")
                    .to_str()
                    .ok_or("workspace path not utf-8")?,
            ])
            .env("GIT_TERMINAL_PROMPT", "0");
            cmd
        },
        "git clone",
    )?;

    // Durable local runtime state (the desktop shell's store).
    let state_path = config.work_dir.join("state.json");
    let store = JsonStateStore::new(state_path);

    let tenant = TenantId::parse("tenant-dogfood").unwrap();
    let grant = |id: &str, capability: &'static str| CapabilityGrant {
        grant_id: id.to_owned(),
        tenant_id: tenant.clone(),
        principal_id: None,
        capability: Capability::well_known(capability),
        resource_scope: lumi_policy::ResourceScope::all_of([
            lumi_protocol::ResourceType::well_known(lumi_protocol::ResourceType::FILE),
        ]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: GrantSource::OrganizationPolicy,
    };
    let orch_config = OrchestratorConfig {
        registry: lumi_policy::CapabilityRegistry::default()
            .grant(grant("g-read", lumi_protocol::capabilities::FILES_READ))
            .grant(grant("g-create", lumi_protocol::capabilities::FILES_CREATE))
            .grant(grant("g-shell", lumi_protocol::capabilities::SHELL_EXECUTE)),
        device_state: lumi_policy::DeviceExecutionState::Trusted,
        pre_authorizations: vec![],
        retry: lumi_state::RetryPolicy::default(),
        policy_version: "1.0.0".to_owned(),
        executors: vec![ExecutorDescriptor::new(
            ExecutionTier::ConnectorApi,
            "workspace-tools",
            "1.0.0",
            [
                Capability::well_known(lumi_protocol::capabilities::FILES_READ),
                Capability::well_known(lumi_protocol::capabilities::FILES_CREATE),
                Capability::well_known(lumi_protocol::capabilities::SHELL_EXECUTE),
            ],
        )],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
    };
    let mut orchestrator = Orchestrator::new(orch_config, store);

    let task = lumi_protocol::Task {
        task_id: TaskId::generate(),
        tenant_id: tenant.clone(),
        principal: crate::workflow_principal("tenant-dogfood", "u-dogfood"),
        mode: TaskMode::Work,
        goal: goal.clone(),
        created_at: Timestamp::now(),
        deadline: None,
        budget: Budget::default(),
        privacy_constraints: lumi_protocol::PrivacyConstraint::default(),
        status: TaskStatus::Created,
        requested_outputs: vec![],
        project_binding: Some(ProjectTaskBinding {
            project_id: ProjectId::generate(),
            execution_environment_id: lumi_protocol::EnvironmentId::generate(),
            workspace_root: workspace.root().display().to_string(),
            workspace_kind: WorkspaceKind::TempStaging,
        }),
    };
    orchestrator
        .store
        .save_task(&task)
        .map_err(|e| e.to_string())?;

    let env = FixtureEnvironment::new().with_workspace(workspace.root().to_path_buf());
    let outcome = run_persisted_task(
        &mut orchestrator,
        planner,
        vec![
            Box::new(ReadFileTool),
            Box::new(WriteFileTool),
            Box::new(RunShellTool),
        ],
        &workspace,
        &env,
        &task,
        RunId::generate(),
    )?;

    // ---- Independent evaluation (the harness's own oracles) ----
    let mut checks = Vec::new();
    let record = |checks: &mut Vec<DogfoodCheck>, check: &str, passed: bool, detail: String| {
        checks.push(DogfoodCheck {
            check: check.to_owned(),
            passed,
            detail,
        });
    };

    let loop_completed = outcome.status == TaskRunStatus::Completed;
    record(
        &mut checks,
        "loop_completed",
        loop_completed,
        match (&outcome.status, &outcome.failure) {
            (TaskRunStatus::Completed, _) => "planner reached a verified final answer".to_owned(),
            (status, Some(reason)) => format!("{status:?}: {reason}"),
            (status, None) => format!("{status:?}"),
        },
    );

    for expected in &expected_files {
        let path = workspace.root().join(expected);
        let exists = path.is_file();
        record(
            &mut checks,
            "expected_file",
            exists,
            format!("{expected}: {}", if exists { "present" } else { "MISSING" }),
        );
    }

    let status_out = run_command(
        git_args(&workspace.root().join("repo"), &["status", "--porcelain"]),
        "git status",
    )
    .unwrap_or_default();
    record(
        &mut checks,
        "git_working_tree_dirty",
        !status_out.trim().is_empty(),
        if status_out.trim().is_empty() {
            "clone working tree unchanged — no real effect landed".to_owned()
        } else {
            format!("git status:\n{}", status_out.trim())
        },
    );

    let persisted = orchestrator.store.read().map_err(|e| e.to_string())?;
    let persisted_task = persisted
        .tasks
        .iter()
        .find(|t| t.task_id == task.task_id)
        .ok_or("persisted task missing")?;
    record(
        &mut checks,
        "durable_task_completed",
        persisted_task.status == TaskStatus::Completed,
        format!("durable status: {:?}", persisted_task.status),
    );
    record(
        &mut checks,
        "run_record_closed",
        persisted.runs.len() == 1 && persisted.runs[0].ended_at.is_some(),
        format!("{} run record(s) persisted", persisted.runs.len()),
    );
    let actions_executed = persisted
        .runs
        .first()
        .map(|r| r.budgets_consumed.actions)
        .unwrap_or(0);

    let denials = orchestrator
        .audit
        .events_for(&tenant)
        .iter()
        .filter_map(|event| match &event.kind {
            AuditEventKind::Action(details) => match &details.policy {
                PolicyOutcome::Denied { .. } => Some(1_u64),
                _ => None,
            },
            _ => None,
        })
        .sum();
    record(
        &mut checks,
        "zero_policy_denials",
        denials == 0,
        format!("{denials} denied action event(s)"),
    );

    let chain_ok = matches!(
        orchestrator.audit.verify_chain(),
        ChainVerification::Intact { .. }
    );
    record(
        &mut checks,
        "audit_chain_intact",
        chain_ok,
        "hash-chained audit ledger verified".to_owned(),
    );

    let verified = checks.iter().all(|c| c.passed);
    Ok(DogfoodReport {
        scenario,
        source_repo: config.source_repo.display().to_string(),
        goal,
        verified,
        loop_status: format!("{:?}", outcome.status),
        answer: outcome.answer,
        turns: outcome.turns,
        actions_executed,
        policy_denials: denials,
        audit_chain: if chain_ok { "intact" } else { "BROKEN" }.to_owned(),
        provider: config.provider.as_ref().map(|(f, _)| f.clone()),
        model: config.provider.as_ref().map(|(_, m)| m.clone()),
        completed_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        checks,
    })
}
