//! Durable local runtime adapter for the desktop operations console.
//!
//! This module owns a real `Orchestrator<JsonStateStore>` with a deny-by-
//! default configuration. It does not invent actions, approvals, evidence,
//! permissions, or economics when the durable runtime has not supplied them.

use crate::api::{
    ConnectionState, DesktopBackend, EconomicsSummary, EvidenceSummaryEntry, ExecutionState,
    OperationsSnapshot,
};
use lumi_audit::{AuditEventKind, PolicyOutcome, VerificationStatus};
use lumi_handoff::completion::ArtifactSummary;
use lumi_handoff::exception::{ExceptionCard, SafeChoice};
use lumi_handoff::{ProgressStep, ProgressView, TaskPhase, TrustLanguage};
use lumi_orchestrator::{Orchestrator, OrchestratorConfig, TierPolicy};
use lumi_policy::CapabilityRegistry;
use lumi_protocol::{ExecutionStatus, Run, Task, TaskId, TaskStatus, TenantId, Timestamp};
use lumi_state::{JsonStateStore, RetryPolicy, SideEffectStatus};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Local runtime state used by the Tauri shell.
pub struct DesktopRuntime {
    /// The existing orchestration choke point. The desktop stop control
    /// cancels this exact token; it does not maintain a second boolean flag.
    pub orchestrator: Orchestrator<JsonStateStore>,
    state_path: PathBuf,
    stop_marker: PathBuf,
    tenant_scope: Option<TenantId>,
    trusted_scope: bool,
    backend: DesktopBackend,
}

impl DesktopRuntime {
    /// Opens or creates a durable local state store with no grants and no
    /// executors. An empty runtime is safe and honest: it cannot execute work
    /// until a trusted host wires capabilities and executors into it.
    pub fn new(state_path: PathBuf) -> Result<Self, String> {
        let persisted = JsonStateStore::new(state_path.clone())
            .read()
            .map_err(|e| format!("read runtime state: {e}"))?;
        let tenant_scope = infer_single_tenant(&persisted.tasks)?;
        Self::new_with_scope(state_path, tenant_scope, false)
    }

    /// Opens a runtime with a tenant scope supplied by a trusted host. This
    /// is the explicit multi-tenant-store escape hatch; all snapshots remain
    /// filtered to this tenant.
    pub fn new_for_tenant(state_path: PathBuf, tenant_id: TenantId) -> Result<Self, String> {
        Self::new_with_scope(state_path, Some(tenant_id), true)
    }

    fn new_with_scope(
        state_path: PathBuf,
        tenant_scope: Option<TenantId>,
        trusted_scope: bool,
    ) -> Result<Self, String> {
        let config = OrchestratorConfig {
            device_state: lumi_policy::DeviceExecutionState::Unregistered,
            registry: CapabilityRegistry::new(vec![]),
            pre_authorizations: vec![],
            retry: RetryPolicy::default(),
            policy_version: "unknown".to_owned(),
            executors: vec![],
            tier_policy: TierPolicy::default(),
        };
        let orchestrator = Orchestrator::restore(config, JsonStateStore::new(state_path.clone()));
        let stop_marker = state_path.with_extension("stopped");
        let stopped_on_disk = stop_marker.exists();
        let mut runtime = Self {
            orchestrator,
            state_path,
            stop_marker,
            tenant_scope,
            trusted_scope,
            backend: DesktopBackend::new(),
        };
        if stopped_on_disk {
            runtime.orchestrator.cancel.cancel();
        }
        runtime.refresh_snapshot()?;
        Ok(runtime)
    }

    /// Path of the durable local state file, useful for diagnostics/tests.
    #[must_use]
    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    /// Sidecar marker used to preserve a local emergency stop across restart.
    #[must_use]
    pub fn stop_marker_path(&self) -> &Path {
        &self.stop_marker
    }

    /// Tenant scope used for every local queue, journal, and audit query.
    #[must_use]
    pub fn tenant_scope(&self) -> Option<&TenantId> {
        self.tenant_scope.as_ref()
    }

    /// Returns the exact cancellation token shared with the orchestrator.
    #[must_use]
    pub fn cancellation_token(&self) -> lumi_state::CancelToken {
        self.orchestrator.cancel.clone()
    }

    /// Stops new admissions through the orchestrator's existing cancellation
    /// gate. This preserves durable state and is idempotent.
    pub fn stop(&self) -> Result<(), String> {
        self.orchestrator.cancel.cancel();
        persist_stop_marker(&self.stop_marker)
    }

    /// Refreshes the view from the current durable state and in-memory audit
    /// ledger. Missing categories remain unknown in the returned snapshot.
    pub fn refresh_snapshot(&mut self) -> Result<OperationsSnapshot, String> {
        let persisted = self
            .orchestrator
            .store
            .read()
            .map_err(|e| format!("read runtime state: {e}"))?;
        if !self.trusted_scope {
            let inferred = infer_single_tenant(&persisted.tasks)?;
            if inferred != self.tenant_scope {
                return Err("runtime tenant scope changed; trusted rebind required".to_owned());
            }
        }
        let (tasks, runs, journal) = scoped_state(
            persisted.tasks,
            persisted.runs,
            persisted.journal,
            self.tenant_scope.as_ref(),
        )?;
        let snapshot = snapshot_from_state(&tasks, &runs, &journal, &self.orchestrator);
        self.backend.replace_snapshot(snapshot.clone());
        Ok(snapshot)
    }

    /// Refreshes and returns the current operations view. A read failure is
    /// represented as disconnected state rather than stale success.
    #[must_use]
    pub fn snapshot(&mut self) -> OperationsSnapshot {
        match self.refresh_snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => OperationsSnapshot {
                connection: ConnectionState::Disconnected,
                execution: if self.orchestrator.cancel.is_cancelled() {
                    ExecutionState::Stopped
                } else {
                    ExecutionState::Unavailable
                },
                ..OperationsSnapshot::default()
            },
        }
    }
}

fn persist_stop_marker(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create stop marker directory: {e}"))?;
    }
    std::fs::write(path, b"stopped\n").map_err(|e| format!("persist emergency stop: {e}"))
}

fn infer_single_tenant(tasks: &[Task]) -> Result<Option<TenantId>, String> {
    let tenants: BTreeSet<_> = tasks.iter().map(|task| task.tenant_id.clone()).collect();
    match tenants.len() {
        0 => Ok(None),
        1 => Ok(tenants.into_iter().next()),
        _ => {
            Err("runtime state contains multiple tenants without a trusted tenant scope".to_owned())
        }
    }
}

type ScopedState = (Vec<Task>, Vec<Run>, Vec<lumi_state::SideEffectRecord>);

fn scoped_state(
    tasks: Vec<Task>,
    runs: Vec<Run>,
    journal: Vec<lumi_state::SideEffectRecord>,
    tenant_scope: Option<&TenantId>,
) -> Result<ScopedState, String> {
    if tenant_scope.is_none() && (!tasks.is_empty() || !runs.is_empty() || !journal.is_empty()) {
        return Err("runtime state has work but no trusted tenant scope".to_owned());
    }
    let Some(tenant_scope) = tenant_scope else {
        return Ok((tasks, runs, journal));
    };
    let tasks: Vec<_> = tasks
        .into_iter()
        .filter(|task| &task.tenant_id == tenant_scope)
        .collect();
    let task_ids: BTreeSet<TaskId> = tasks.iter().map(|task| task.task_id.clone()).collect();
    let runs: Vec<_> = runs
        .into_iter()
        .filter(|run| task_ids.contains(&run.task_id))
        .collect();
    let run_ids: BTreeSet<_> = runs.iter().map(|run| run.run_id.clone()).collect();
    let journal: Vec<_> = journal
        .into_iter()
        .filter(|record| run_ids.contains(&record.run_id))
        .collect();
    Ok((tasks, runs, journal))
}

fn snapshot_from_state(
    tasks: &[Task],
    runs: &[Run],
    journal: &[lumi_state::SideEffectRecord],
    orchestrator: &Orchestrator<JsonStateStore>,
) -> OperationsSnapshot {
    let mut queue = tasks
        .iter()
        .map(|task| progress_for(task, runs, journal, orchestrator))
        .collect::<Vec<_>>();
    queue.sort_by(|a, b| a.task_id.cmp(&b.task_id));

    let progress = queue
        .iter()
        .find(|view| {
            !matches!(
                view.phase,
                TaskPhase::Completed | TaskPhase::Failed | TaskPhase::Cancelled
            )
        })
        .cloned()
        .or_else(|| queue.first().cloned());

    let exceptions = exceptions_for(journal);
    let evidence = progress.as_ref().and_then(|view| {
        tasks
            .iter()
            .find(|task| task.task_id.as_str() == view.task_id)
            .and_then(|task| evidence_for(orchestrator, &task.task_id, &task.tenant_id))
    });

    let economics = if runs.is_empty() {
        None
    } else {
        Some(EconomicsSummary {
            verified_work_units: None,
            model_cost_micro_usd: Some(
                runs.iter()
                    .map(|run| run.budgets_consumed.model_cost_micro_usd)
                    .sum(),
            ),
            human_intervention_minutes: None,
            released_human_minutes: None,
        })
    };

    OperationsSnapshot {
        connection: ConnectionState::Connected,
        execution: if orchestrator.cancel.is_cancelled() {
            ExecutionState::Stopped
        } else {
            // The local shell currently has no grants or executors attached.
            ExecutionState::Unavailable
        },
        queue: Some(queue),
        progress,
        // ApprovalLedger is in-memory and no normalized action/approver is
        // attached to this shell yet. `None` is honest; an empty list would
        // claim that the queue was queried and found empty.
        pending_approvals: None,
        exceptions: Some(exceptions),
        evidence,
        economics,
        // OS permission probing belongs to the platform adapter and is not
        // available from this portable runtime module.
        permissions: None,
    }
}

fn progress_for(
    task: &Task,
    runs: &[Run],
    journal: &[lumi_state::SideEffectRecord],
    orchestrator: &Orchestrator<JsonStateStore>,
) -> ProgressView {
    let run = runs
        .iter()
        .filter(|candidate| candidate.task_id == task.task_id)
        .max_by_key(|candidate| candidate.started_at);
    let run_id = run.map(|candidate| candidate.run_id.as_str());
    let task_exceptions = journal
        .iter()
        .filter(|record| {
            run_id == Some(record.run_id.as_str())
                && !matches!(record.status, SideEffectStatus::Succeeded)
        })
        .map(exception_for)
        .collect::<Vec<_>>();
    let evidence = evidence_for(orchestrator, &task.task_id, &task.tenant_id).unwrap_or_default();
    let steps = evidence
        .iter()
        .map(|entry| ProgressStep {
            label: entry.operation.clone(),
            trust: trust_from_label(&entry.trust_label),
            system: None,
        })
        .collect();
    let elapsed_minutes =
        u32::try_from(Timestamp::now().duration_since(&task.created_at).as_secs() / 60)
            .unwrap_or(u32::MAX);
    let (budget_used, budget_total) = run
        .map(|current| {
            (
                current.budgets_consumed.actions,
                task.budget.max_actions.unwrap_or(0),
            )
        })
        .unwrap_or((0, task.budget.max_actions.unwrap_or(0)));

    ProgressView {
        task_id: task.task_id.as_str().to_owned(),
        phase: phase_for(task.status, &evidence, task_exceptions.is_empty()),
        goal: task.goal.clone(),
        steps,
        active_system: None,
        pending_approvals: vec![],
        exceptions: task_exceptions,
        artifacts: Vec::<ArtifactSummary>::new(),
        elapsed_minutes,
        budget_used,
        budget_total,
        evidence_summary: if task.status == TaskStatus::Completed {
            "Completion reported; whole-task verification requires review".to_owned()
        } else if evidence.is_empty() {
            "Evidence unavailable".to_owned()
        } else {
            format!("{} action evidence records", evidence.len())
        },
    }
}

fn phase_for(
    status: TaskStatus,
    _evidence: &[EvidenceSummaryEntry],
    _no_unresolved_exceptions: bool,
) -> TaskPhase {
    match status {
        TaskStatus::Created | TaskStatus::Queued => TaskPhase::Queued,
        TaskStatus::Running => TaskPhase::Executing,
        TaskStatus::WaitingApproval => TaskPhase::WaitingApproval,
        TaskStatus::WaitingUser => TaskPhase::WaitingUser,
        TaskStatus::WaitingExternal => TaskPhase::WaitingExternal,
        TaskStatus::Paused => TaskPhase::Paused,
        // Action evidence does not prove the entire task's required work set.
        // Until a task-level completion oracle is connected, a persisted
        // completion claim requires review, even if some actions verified.
        TaskStatus::Completed => TaskPhase::WaitingUser,
        TaskStatus::Failed => TaskPhase::Failed,
        TaskStatus::Ambiguous => TaskPhase::Ambiguous,
        TaskStatus::Cancelled => TaskPhase::Cancelled,
    }
}

fn exception_for(record: &lumi_state::SideEffectRecord) -> ExceptionCard {
    let (what_blocked, why, consequences) = match record.status {
        SideEffectStatus::Proposed => (
            "A consequential action has durable intent but no recorded outcome".to_owned(),
            "The runtime may have stopped before persisting execution result".to_owned(),
            "Outcome is unknown; do not retry before external verification".to_owned(),
        ),
        SideEffectStatus::Ambiguous => (
            "A consequential action has an ambiguous outcome".to_owned(),
            "The executor could not prove whether the external effect occurred".to_owned(),
            "Outcome is unknown; do not retry before external verification".to_owned(),
        ),
        SideEffectStatus::Failed => (
            "A consequential action failed".to_owned(),
            record
                .failure
                .as_ref()
                .map(|failure| format!("{}: {}", failure.category.as_str(), failure.message))
                .unwrap_or_else(|| "The runtime recorded a failed outcome".to_owned()),
            "No verified successful effect is recorded".to_owned(),
        ),
        SideEffectStatus::Succeeded => (
            "No exception".to_owned(),
            "The side effect is verified".to_owned(),
            "No unresolved consequence".to_owned(),
        ),
    };
    ExceptionCard {
        what_blocked,
        why,
        safe_choices: vec![
            SafeChoice {
                label: "Verify external state".to_owned(),
                description: "Inspect the affected system before any retry".to_owned(),
            },
            SafeChoice {
                label: "Leave paused".to_owned(),
                description: "Keep the task blocked until a human resolves the state".to_owned(),
            },
        ],
        consequences,
        evidence_refs: Vec::new(),
        suggested_next_step: "Verify the external state before continuing".to_owned(),
    }
}

fn exceptions_for(journal: &[lumi_state::SideEffectRecord]) -> Vec<ExceptionCard> {
    journal
        .iter()
        .filter(|record| !matches!(record.status, SideEffectStatus::Succeeded))
        .map(exception_for)
        .collect()
}

fn evidence_for(
    orchestrator: &Orchestrator<JsonStateStore>,
    task_id: &TaskId,
    tenant_id: &TenantId,
) -> Option<Vec<EvidenceSummaryEntry>> {
    let events = orchestrator.audit.events_for(tenant_id);
    let entries = events
        .iter()
        .filter_map(|event| {
            if &event.task_id != task_id {
                return None;
            }
            let AuditEventKind::Action(details) = &event.kind else {
                return None;
            };
            let trust = match &details.policy {
                PolicyOutcome::Denied { .. } => TrustLanguage::Failed,
                PolicyOutcome::ApprovalRequired { .. } => TrustLanguage::Planned,
                PolicyOutcome::Allowed { .. } => {
                    match (details.execution_status, details.verification) {
                        (Some(ExecutionStatus::Success), VerificationStatus::Passed)
                        | (Some(ExecutionStatus::Success), VerificationStatus::NotRequired) => {
                            TrustLanguage::Verified
                        }
                        (_, VerificationStatus::Failed) | (Some(ExecutionStatus::Failed), _) => {
                            TrustLanguage::Failed
                        }
                        (_, VerificationStatus::Ambiguous)
                        | (Some(ExecutionStatus::Ambiguous), _) => TrustLanguage::Ambiguous,
                        (Some(ExecutionStatus::Cancelled), _) => TrustLanguage::Cancelled,
                        _ => TrustLanguage::Planned,
                    }
                }
            };
            Some(DesktopBackend::build_evidence_entry(
                &details.action_id,
                &details.operation,
                &trust,
                Some(details.target.canonical.as_str()),
            ))
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

fn trust_from_label(label: &str) -> TrustLanguage {
    if label.starts_with("Verified") {
        TrustLanguage::Verified
    } else if label.starts_with("Ambiguous") {
        TrustLanguage::Ambiguous
    } else if label.starts_with("Failed") {
        TrustLanguage::Failed
    } else if label.starts_with("Cancelled") {
        TrustLanguage::Cancelled
    } else if label.starts_with("Executed") {
        TrustLanguage::Executed
    } else {
        TrustLanguage::Planned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_verified_action_cannot_certify_whole_task_completion() {
        let evidence = vec![EvidenceSummaryEntry {
            action_id: "one-step".to_owned(),
            operation: "read_one_record".to_owned(),
            trust_label: "Verified".to_owned(),
            verification: Some("postconditions passed".to_owned()),
            target: None,
        }];
        assert_eq!(
            phase_for(TaskStatus::Completed, &evidence, true),
            TaskPhase::WaitingUser
        );
    }
    use lumi_protocol::{
        AuthenticationStrength, Budget, ConsumedBudget, Principal, PrincipalId, PrincipalKind,
        PrivacyConstraint, RunId, RunState, TaskId, TaskMode, TenantId,
    };
    use lumi_state::StateStore;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "lumi-desktop-runtime-{name}-{}.json",
            std::process::id()
        ))
    }

    fn clean_path(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("stopped"));
    }

    fn task(status: TaskStatus) -> Task {
        Task {
            task_id: TaskId::parse("task-runtime").unwrap(),
            tenant_id: TenantId::parse("tenant-runtime").unwrap(),
            principal: Principal {
                principal_id: PrincipalId::parse("human-runtime").unwrap(),
                tenant_id: TenantId::parse("tenant-runtime").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(Timestamp::UNIX_EPOCH),
                authentication_strength: Some(AuthenticationStrength::Mfa),
            },
            mode: TaskMode::Workflow,
            goal: "Persisted runtime task".to_owned(),
            created_at: Timestamp::UNIX_EPOCH,
            deadline: None,
            budget: Budget {
                max_actions: Some(4),
                ..Budget::default()
            },
            privacy_constraints: PrivacyConstraint::default(),
            status,
            requested_outputs: vec![],
            project_binding: None,
        }
    }

    fn run() -> Run {
        Run {
            run_id: RunId::parse("run-runtime").unwrap(),
            task_id: TaskId::parse("task-runtime").unwrap(),
            runtime_version: "0.1.0".to_owned(),
            workflow_version: None,
            selected_providers: vec![],
            started_at: Timestamp::UNIX_EPOCH,
            ended_at: None,
            state: RunState::Executing,
            budgets_consumed: ConsumedBudget {
                actions: 2,
                ..ConsumedBudget::default()
            },
            failure: None,
        }
    }

    #[test]
    fn seeded_state_reopens_into_queue_and_progress() {
        let path = temp_path("seeded");
        clean_path(&path);
        let mut store = JsonStateStore::new(path.clone());
        let task = task(TaskStatus::Running);
        store.save_task(&task).unwrap();
        store.save_run(&run()).unwrap();

        let mut runtime = DesktopRuntime::new(path.clone()).unwrap();
        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.connection, ConnectionState::Connected);
        assert_eq!(snapshot.queue.as_ref().unwrap().len(), 1);
        assert_eq!(snapshot.progress.as_ref().unwrap().task_id, "task-runtime");
        assert_eq!(snapshot.progress.as_ref().unwrap().budget_used, 2);
        assert_eq!(snapshot.progress.as_ref().unwrap().budget_total, 4);
        clean_path(&path);
    }

    #[test]
    fn mixed_tenant_state_requires_scope_and_filters_queries() {
        let path = temp_path("mixed");
        clean_path(&path);
        let mut store = JsonStateStore::new(path.clone());
        store.save_task(&task(TaskStatus::Running)).unwrap();
        let mut other = task(TaskStatus::Queued);
        other.task_id = TaskId::parse("task-other").unwrap();
        other.tenant_id = TenantId::parse("tenant-other").unwrap();
        other.principal.tenant_id = TenantId::parse("tenant-other").unwrap();
        store.save_task(&other).unwrap();

        assert!(DesktopRuntime::new(path.clone()).is_err());
        let mut scoped = DesktopRuntime::new_for_tenant(
            path.clone(),
            TenantId::parse("tenant-runtime").unwrap(),
        )
        .unwrap();
        let snapshot = scoped.snapshot();
        assert_eq!(snapshot.queue.as_ref().unwrap().len(), 1);
        assert_eq!(snapshot.queue.as_ref().unwrap()[0].task_id, "task-runtime");
        clean_path(&path);
    }

    #[test]
    fn stop_blocks_execute_step_through_real_orchestrator_token() {
        let path = temp_path("stop");
        clean_path(&path);
        let mut runtime = DesktopRuntime::new(path.clone()).unwrap();
        let action = lumi_protocol::ActionProposal::builder(
            lumi_protocol::ActionId::parse("a-stop").unwrap(),
            TaskId::parse("task-runtime").unwrap(),
            RunId::parse("run-runtime").unwrap(),
            task(TaskStatus::Running).principal,
            lumi_protocol::Capability::well_known(lumi_protocol::capabilities::FILES_READ),
            lumi_protocol::ResourceRef {
                resource_type: lumi_protocol::ResourceType::well_known(
                    lumi_protocol::ResourceType::FILE,
                ),
                id: "workspace/file.txt".to_owned(),
                sensitivity: None,
            },
            lumi_protocol::Target::canonical("file://workspace/file.txt"),
            "read_file",
            lumi_protocol::RiskClass::Read,
        )
        .arguments(serde_json::json!({}))
        .unwrap();
        runtime.stop().unwrap();
        let mut consumed = ConsumedBudget::default();
        let outcome = runtime.orchestrator.execute_step(
            &action,
            None,
            &Budget::default(),
            &mut consumed,
            &lumi_audit::UnavailableEnvironment,
            &mut |_action| unreachable!("cancelled before executor"),
        );
        assert!(matches!(
            outcome,
            lumi_orchestrator::StepOutcome::Stopped { .. }
        ));
        drop(runtime);
        let restored = DesktopRuntime::new(path.clone()).unwrap();
        assert!(restored.cancellation_token().is_cancelled());
        clean_path(&path);
    }
}
