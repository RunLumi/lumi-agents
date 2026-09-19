//! Durable state storage and the resume procedure (spec 02 §2.5, §2.8,
//! §18.11).
//!
//! The [`StateStore`] persists tasks, runs, checkpoints, pre-action
//! checkpoints, and the side-effect journal. [`JsonStateStore`] writes
//! whole-state snapshots atomically (temp file + rename), so a crash mid
//! write leaves the previous snapshot intact. [`resume`] implements the
//! spec 02 §2.8 procedure as a pure, testable function over loaded state.

use crate::checkpoint::{Checkpoint, PreActionCheckpoint};
use crate::journal::{AmbiguousResolution, SideEffectJournal, SideEffectStatus};
use lumi_protocol::{ActionId, RunId, Task, TaskStatus, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Storage errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound(String),
    Io(String),
    Serde(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(what) => write!(f, "not found: {what}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Serde(e) => write!(f, "serialization error: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// Durable state store contract.
pub trait StateStore {
    fn save_task(&mut self, task: &Task) -> Result<(), StoreError>;
    fn load_task(&self, task_id: &lumi_protocol::TaskId) -> Result<Task, StoreError>;
    fn save_run(&mut self, run: &lumi_protocol::Run) -> Result<(), StoreError>;
    fn load_run(&self, run_id: &RunId) -> Result<lumi_protocol::Run, StoreError>;
    /// Saves the latest checkpoint for a run (replaces prior).
    fn save_checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), StoreError>;
    fn load_checkpoint(&self, run_id: &RunId) -> Result<Checkpoint, StoreError>;
    /// Saves the latest pre-action checkpoint for a run.
    fn save_pre_action(&mut self, pre: &PreActionCheckpoint) -> Result<(), StoreError>;
    fn load_pre_action(&self, run_id: &RunId) -> Result<PreActionCheckpoint, StoreError>;
    fn save_journal(&mut self, journal: &SideEffectJournal) -> Result<(), StoreError>;
    fn load_journal(&self) -> Result<SideEffectJournal, StoreError>;
}

/// Full durable state document (serialized by [`JsonStateStore`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedState {
    pub tasks: Vec<Task>,
    pub runs: Vec<lumi_protocol::Run>,
    pub checkpoints: Vec<Checkpoint>,
    pub pre_actions: Vec<(RunId, PreActionCheckpoint)>,
    pub journal: Vec<crate::journal::SideEffectRecord>,
}

/// In-memory reference store (also the unit-test surface).
#[derive(Debug, Default, Clone)]
pub struct InMemoryStateStore {
    state: PersistedState,
}

impl InMemoryStateStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Direct read access (for assertions and orchestration).
    #[must_use]
    pub const fn state(&self) -> &PersistedState {
        &self.state
    }
}

impl StateStore for InMemoryStateStore {
    fn save_task(&mut self, task: &Task) -> Result<(), StoreError> {
        self.state.tasks.retain(|t| t.task_id != task.task_id);
        self.state.tasks.push(task.clone());
        Ok(())
    }

    fn load_task(&self, task_id: &lumi_protocol::TaskId) -> Result<Task, StoreError> {
        self.state
            .tasks
            .iter()
            .find(|t| &t.task_id == task_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("task {task_id}")))
    }

    fn save_run(&mut self, run: &lumi_protocol::Run) -> Result<(), StoreError> {
        self.state.runs.retain(|r| r.run_id != run.run_id);
        self.state.runs.push(run.clone());
        Ok(())
    }

    fn load_run(&self, run_id: &RunId) -> Result<lumi_protocol::Run, StoreError> {
        self.state
            .runs
            .iter()
            .find(|r| &r.run_id == run_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("run {run_id}")))
    }

    fn save_checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), StoreError> {
        self.state
            .checkpoints
            .retain(|c| c.run_id != checkpoint.run_id);
        self.state.checkpoints.push(checkpoint.clone());
        Ok(())
    }

    fn load_checkpoint(&self, run_id: &RunId) -> Result<Checkpoint, StoreError> {
        self.state
            .checkpoints
            .iter()
            .find(|c| &c.run_id == run_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("checkpoint for {run_id}")))
    }

    fn save_pre_action(&mut self, pre: &PreActionCheckpoint) -> Result<(), StoreError> {
        // Pre-action checkpoints are keyed by run; the run id lives inside
        // the serialized action. We key by parsing action JSON's run_id is
        // awkward — instead callers pass the run-scoped pre-action via the
        // index map. Here we replace by matching run ids.
        let run_id = pre.run_id();
        self.state.pre_actions.retain(|(rid, _)| rid != &run_id);
        self.state.pre_actions.push((run_id, pre.clone()));
        Ok(())
    }

    fn load_pre_action(&self, run_id: &RunId) -> Result<PreActionCheckpoint, StoreError> {
        self.state
            .pre_actions
            .iter()
            .find(|(rid, _)| rid == run_id)
            .map(|(_, pre)| pre.clone())
            .ok_or_else(|| StoreError::NotFound(format!("pre-action checkpoint for {run_id}")))
    }

    fn save_journal(&mut self, journal: &SideEffectJournal) -> Result<(), StoreError> {
        self.state.journal = journal.all().iter().map(|r| (*r).clone()).collect();
        Ok(())
    }

    fn load_journal(&self) -> Result<SideEffectJournal, StoreError> {
        let mut journal = SideEffectJournal::new();
        journal.restore(self.state.journal.clone());
        Ok(journal)
    }
}

/// File-backed snapshot store with atomic replacement.
pub struct JsonStateStore {
    path: PathBuf,
}

impl JsonStateStore {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Reads the raw document.
    ///
    /// # Errors
    /// Not-found maps to an empty document; other I/O and parse errors
    /// propagate.
    pub fn read(&self) -> Result<PersistedState, StoreError> {
        match std::fs::read(&self.path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| StoreError::Serde(e.to_string()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PersistedState::default()),
            Err(e) => Err(StoreError::Io(e.to_string())),
        }
    }

    /// Atomically replaces the document: write to `<path>.tmp`, fsync,
    /// rename over the target. A crash mid-write never corrupts the prior
    /// snapshot.
    fn write(&self, state: &PersistedState) -> Result<(), StoreError> {
        use std::io::Write;
        let bytes =
            serde_json::to_vec_pretty(state).map_err(|e| StoreError::Serde(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StoreError::Io(e.to_string()))?;
        }
        {
            let mut file =
                std::fs::File::create(&tmp).map_err(|e| StoreError::Io(e.to_string()))?;
            file.write_all(&bytes)
                .map_err(|e| StoreError::Io(e.to_string()))?;
            file.sync_all().map_err(|e| StoreError::Io(e.to_string()))?;
        }
        std::fs::rename(&tmp, &self.path).map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(())
    }
}

impl StateStore for JsonStateStore {
    fn save_task(&mut self, task: &Task) -> Result<(), StoreError> {
        let mut state = self.read()?;
        state.tasks.retain(|t| t.task_id != task.task_id);
        state.tasks.push(task.clone());
        self.write(&state)
    }

    fn load_task(&self, task_id: &lumi_protocol::TaskId) -> Result<Task, StoreError> {
        self.read()?
            .tasks
            .into_iter()
            .find(|t| &t.task_id == task_id)
            .ok_or_else(|| StoreError::NotFound(format!("task {task_id}")))
    }

    fn save_run(&mut self, run: &lumi_protocol::Run) -> Result<(), StoreError> {
        let mut state = self.read()?;
        state.runs.retain(|r| r.run_id != run.run_id);
        state.runs.push(run.clone());
        self.write(&state)
    }

    fn load_run(&self, run_id: &RunId) -> Result<lumi_protocol::Run, StoreError> {
        self.read()?
            .runs
            .into_iter()
            .find(|r| &r.run_id == run_id)
            .ok_or_else(|| StoreError::NotFound(format!("run {run_id}")))
    }

    fn save_checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), StoreError> {
        let mut state = self.read()?;
        state.checkpoints.retain(|c| c.run_id != checkpoint.run_id);
        state.checkpoints.push(checkpoint.clone());
        self.write(&state)
    }

    fn load_checkpoint(&self, run_id: &RunId) -> Result<Checkpoint, StoreError> {
        self.read()?
            .checkpoints
            .into_iter()
            .find(|c| &c.run_id == run_id)
            .ok_or_else(|| StoreError::NotFound(format!("checkpoint for {run_id}")))
    }

    fn save_pre_action(&mut self, pre: &PreActionCheckpoint) -> Result<(), StoreError> {
        let mut state = self.read()?;
        let run_id = pre.run_id();
        state.pre_actions.retain(|(rid, _)| rid != &run_id);
        state.pre_actions.push((run_id, pre.clone()));
        self.write(&state)
    }

    fn load_pre_action(&self, run_id: &RunId) -> Result<PreActionCheckpoint, StoreError> {
        self.read()?
            .pre_actions
            .into_iter()
            .find(|(rid, _)| rid == run_id)
            .map(|(_, pre)| pre)
            .ok_or_else(|| StoreError::NotFound(format!("pre-action checkpoint for {run_id}")))
    }

    fn save_journal(&mut self, journal: &SideEffectJournal) -> Result<(), StoreError> {
        let mut state = self.read()?;
        state.journal = journal.all().iter().map(|r| (*r).clone()).collect();
        self.write(&state)
    }

    fn load_journal(&self) -> Result<SideEffectJournal, StoreError> {
        let mut journal = SideEffectJournal::new();
        journal.restore(self.read()?.journal);
        Ok(journal)
    }
}

/// What the runtime should do after loading durable state.
#[derive(Debug, Clone, PartialEq)]
pub enum ResumeAction {
    /// Continue execution at the checkpointed step position.
    ContinueFrom { step_position: u32 },
    /// A side effect's outcome is unknown: verify its external
    /// postconditions through the verifier before anything else.
    ReverifySideEffect { action_id: ActionId },
    /// The pending approval expired or is invalid: re-enter approval.
    ReRequestApproval { reason: String },
    /// Resume is blocked for a structural reason (version drift, revoked
    /// device, incompatible checkpoint). Human decision required.
    Blocked { reason: String },
}

/// Inputs to the resume procedure (spec 02 §2.8).
pub struct ResumeContext<'a> {
    pub now: Timestamp,
    /// Runtime version that would resume the run.
    pub runtime_version: &'a str,
    /// Policy version this runtime would enforce.
    pub policy_version: &'a str,
    /// Policy version the run was started under (recorded by the
    /// orchestrator on the run/checkpoint).
    pub policy_version_at_start: &'a str,
    /// Whether the device is currently able to execute (registered,
    /// capabilities present). Revoked devices block resume entirely.
    pub device_ready: bool,
    /// Result of validating the run's pending approval, if any.
    pub pending_approval: Option<lumi_policy::ApprovalValidation>,
    /// External-state resolutions for unresolved side effects, when the
    /// caller has already verified them.
    pub verified_resolutions: HashMap<ActionId, AmbiguousResolution>,
}

/// Implements spec 02 §2.8 resume as a pure function over loaded state.
///
/// The returned plan is ordered: blocked-check first, then ambiguity
/// resolution, then approval, then continuation.
#[allow(clippy::too_many_lines)]
pub fn resume(
    task: &Task,
    run: &lumi_protocol::Run,
    checkpoint: Option<&Checkpoint>,
    journal: &SideEffectJournal,
    ctx: &ResumeContext<'_>,
) -> Vec<ResumeAction> {
    let mut plan = Vec::new();

    // 2. Structural compatibility gates.
    if !ctx.device_ready {
        plan.push(ResumeAction::Blocked {
            reason: "device not ready (revoked or missing capabilities)".to_owned(),
        });
        return plan;
    }
    if ctx.policy_version_at_start != ctx.policy_version {
        plan.push(ResumeAction::Blocked {
            reason: format!(
                "policy version changed since start: {} -> {}",
                ctx.policy_version_at_start, ctx.policy_version
            ),
        });
        return plan;
    }
    if run.runtime_version != ctx.runtime_version {
        plan.push(ResumeAction::Blocked {
            reason: format!(
                "run started under runtime {} but resuming with {}",
                run.runtime_version, ctx.runtime_version
            ),
        });
        return plan;
    }
    if task.status.is_terminal() {
        return plan; // nothing to resume
    }

    // 5. Pending approvals: expired or invalid approvals route back to
    // the approval queue, never silently continue.
    if task.status == TaskStatus::WaitingApproval {
        match &ctx.pending_approval {
            Some(lumi_policy::ApprovalValidation::Invalid { reason }) => {
                plan.push(ResumeAction::ReRequestApproval {
                    reason: reason.as_str().to_owned(),
                });
                return plan;
            }
            Some(lumi_policy::ApprovalValidation::Valid { .. }) => {}
            None => {
                plan.push(ResumeAction::ReRequestApproval {
                    reason: "approval state missing after restart".to_owned(),
                });
                return plan;
            }
        }
    }

    // 6. Unresolved side effects: verify external state before any
    // continuation (spec 02 §2.7, spec 18 §18.11). A caller-supplied
    // resolution counts as verified; the orchestrator applies it to the
    // journal via `resolve_ambiguous` before continuing.
    let unresolved = journal.unresolved_for_run(&run.run_id);
    let mut ambiguity_resolved = true;
    for record in unresolved {
        match ctx.verified_resolutions.get(&record.action_id) {
            Some(AmbiguousResolution::StillUnknown) | None => {
                ambiguity_resolved = false;
                plan.push(ResumeAction::ReverifySideEffect {
                    action_id: record.action_id.clone(),
                });
            }
            Some(_) => {}
        }
    }
    if !ambiguity_resolved {
        // Ambiguity blocks continuation until resolved.
        return plan;
    }

    // Any journal record that is Proposed/Ambiguous and already resolved
    // as ConfirmedApplied must NOT be re-executed: continuation proceeds
    // from after that action (the checkpoint already reflects it).
    match task.status {
        TaskStatus::Running | TaskStatus::Queued | TaskStatus::Created => {
            plan.push(ResumeAction::ContinueFrom {
                step_position: checkpoint.map_or(0, |c| c.step_position),
            });
        }
        TaskStatus::Paused => {
            // Pause stays paused: user resumes explicitly. We surface the
            // position so the UI can offer resume.
            plan.push(ResumeAction::ContinueFrom {
                step_position: checkpoint.map_or(0, |c| c.step_position),
            });
        }
        TaskStatus::WaitingUser | TaskStatus::WaitingExternal | TaskStatus::Ambiguous => {
            plan.push(ResumeAction::Blocked {
                reason: format!("task is in human/external wait state {:?}", task.status),
            });
        }
        TaskStatus::WaitingApproval
        | TaskStatus::Completed
        | TaskStatus::Failed
        | TaskStatus::Cancelled => {
            // WaitingApproval handled above; terminal handled above.
        }
    }

    plan
}

impl PreActionCheckpoint {
    /// The run this pre-action checkpoint belongs to (parsed from the
    /// serialized action).
    #[must_use]
    pub fn run_id(&self) -> RunId {
        lumi_protocol::ActionProposal::from_json(&self.action)
            .map(|a| a.run_id)
            .unwrap_or_else(|_| RunId::parse("unknown").unwrap_or_else(|_| RunId::generate()))
    }
}

/// Status helper used by the orchestrator to decide whether a journaled
/// side effect can be retried without duplication risk.
#[must_use]
pub fn side_effect_retryable(status: SideEffectStatus) -> bool {
    matches!(status, SideEffectStatus::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::SideEffectRecord;
    use lumi_protocol::{
        AuthenticationStrength, ConsumedBudget, Principal, PrincipalId, PrincipalKind, TenantId,
    };
    use std::collections::BTreeSet;

    fn ts(secs: i64) -> Timestamp {
        Timestamp::from_epoch(secs, 0).unwrap()
    }

    fn task(status: TaskStatus) -> Task {
        Task {
            task_id: lumi_protocol::TaskId::parse("task-1").unwrap(),
            tenant_id: TenantId::parse("t-1").unwrap(),
            principal: Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(ts(0)),
                authentication_strength: Some(AuthenticationStrength::Mfa),
            },
            mode: lumi_protocol::TaskMode::Workflow,
            goal: "fixture".to_owned(),
            created_at: ts(0),
            deadline: None,
            budget: lumi_protocol::Budget::default(),
            privacy_constraints: lumi_protocol::PrivacyConstraint::default(),
            status,
            requested_outputs: vec![],
        }
    }

    fn run() -> lumi_protocol::Run {
        lumi_protocol::Run {
            run_id: RunId::parse("run-1").unwrap(),
            task_id: lumi_protocol::TaskId::parse("task-1").unwrap(),
            runtime_version: "0.1.0".to_owned(),
            workflow_version: None,
            selected_providers: vec![],
            started_at: ts(0),
            ended_at: None,
            state: lumi_protocol::RunState::Recovering,
            budgets_consumed: ConsumedBudget::default(),
            failure: None,
        }
    }

    fn checkpoint() -> Checkpoint {
        Checkpoint {
            run_id: RunId::parse("run-1").unwrap(),
            step_position: 3,
            completed_step_ids: BTreeSet::new(),
            pending_actions: vec![],
            last_verified_side_effect: None,
            retry_counters: Default::default(),
            consumed_budgets: ConsumedBudget::default(),
            model_context_refs: vec![],
            workflow_state_refs: vec![],
            timestamp: ts(5),
        }
    }

    fn ctx<'a>() -> ResumeContext<'a> {
        ResumeContext {
            now: ts(100),
            runtime_version: "0.1.0",
            policy_version: "1.0.0",
            policy_version_at_start: "1.0.0",
            device_ready: true,
            pending_approval: None,
            verified_resolutions: HashMap::new(),
        }
    }

    #[test]
    fn clean_restart_continues_from_checkpoint() {
        let plan = resume(
            &task(TaskStatus::Running),
            &run(),
            Some(&checkpoint()),
            &SideEffectJournal::new(),
            &ctx(),
        );
        assert_eq!(plan, vec![ResumeAction::ContinueFrom { step_position: 3 }]);
    }

    #[test]
    fn ambiguous_side_effect_blocks_until_verified() {
        let mut journal = SideEffectJournal::new();
        let action = lumi_protocol::ActionProposal::builder(
            lumi_protocol::ActionId::parse("a-1").unwrap(),
            lumi_protocol::TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            Principal {
                principal_id: PrincipalId::parse("u-1").unwrap(),
                tenant_id: TenantId::parse("t-1").unwrap(),
                kind: PrincipalKind::User,
                authenticated_at: Some(ts(0)),
                authentication_strength: Some(AuthenticationStrength::Mfa),
            },
            lumi_protocol::Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            lumi_protocol::ResourceRef {
                resource_type: lumi_protocol::ResourceType::well_known(
                    lumi_protocol::ResourceType::EMAIL_MESSAGE,
                ),
                id: "x".to_owned(),
                sensitivity: None,
            },
            lumi_protocol::Target::canonical("mailto:c@example.test"),
            "send_customer_email",
            lumi_protocol::RiskClass::Communication,
        )
        .unwrap();
        journal.propose(&action, ts(1));
        // Crash before result persistence: record is Proposed.
        let plan = resume(
            &task(TaskStatus::Running),
            &run(),
            Some(&checkpoint()),
            &journal,
            &ctx(),
        );
        assert_eq!(
            plan,
            vec![ResumeAction::ReverifySideEffect {
                action_id: action.action_id.clone()
            }]
        );

        // With a confirmed-not-applied resolution, continuation proceeds.
        let mut resolutions = HashMap::new();
        resolutions.insert(
            action.action_id.clone(),
            AmbiguousResolution::ConfirmedNotApplied,
        );
        let ctx2 = ResumeContext {
            verified_resolutions: resolutions,
            ..ctx()
        };
        let plan = resume(
            &task(TaskStatus::Running),
            &run(),
            Some(&checkpoint()),
            &journal,
            &ctx2,
        );
        assert_eq!(plan, vec![ResumeAction::ContinueFrom { step_position: 3 }]);
    }

    #[test]
    fn policy_version_change_blocks_resume() {
        let ctx2 = ResumeContext {
            policy_version: "1.1.0",
            ..ctx()
        };
        let plan = resume(
            &task(TaskStatus::Running),
            &run(),
            None,
            &SideEffectJournal::new(),
            &ctx2,
        );
        assert!(matches!(plan.as_slice(), [ResumeAction::Blocked { .. }]));
    }

    #[test]
    fn expired_approval_requeues_for_approval() {
        let ctx2 = ResumeContext {
            pending_approval: Some(lumi_policy::ApprovalValidation::Invalid {
                reason: lumi_policy::ApprovalInvalidReason::Expired,
            }),
            ..ctx()
        };
        let plan = resume(
            &task(TaskStatus::WaitingApproval),
            &run(),
            None,
            &SideEffectJournal::new(),
            &ctx2,
        );
        assert_eq!(
            plan,
            vec![ResumeAction::ReRequestApproval {
                reason: "approval expired".to_owned()
            }]
        );
    }

    #[test]
    fn revoked_device_blocks_resume() {
        let ctx2 = ResumeContext {
            device_ready: false,
            ..ctx()
        };
        let plan = resume(
            &task(TaskStatus::Running),
            &run(),
            None,
            &SideEffectJournal::new(),
            &ctx2,
        );
        assert!(matches!(plan.as_slice(), [ResumeAction::Blocked { .. }]));
    }

    #[test]
    fn json_store_round_trips_atomically() {
        let dir = std::env::temp_dir().join(format!("lumi-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let _ = std::fs::remove_file(&path);
        let mut store = JsonStateStore::new(path.clone());
        let t = task(TaskStatus::Running);
        store.save_task(&t).unwrap();
        store.save_run(&run()).unwrap();
        store.save_checkpoint(&checkpoint()).unwrap();
        let mut journal = SideEffectJournal::new();
        let record = SideEffectRecord {
            action_id: lumi_protocol::ActionId::parse("a-9").unwrap(),
            run_id: RunId::parse("run-1").unwrap(),
            idempotency_key: None,
            action_digest: "d".to_owned(),
            status: SideEffectStatus::Succeeded,
            proposed_at: ts(1),
            resolved_at: Some(ts(2)),
            result_digest: Some("r".to_owned()),
            failure: None,
        };
        journal.restore(vec![record]);
        store.save_journal(&journal).unwrap();

        let reloaded: Box<dyn StateStore> = Box::new(JsonStateStore::new(path.clone()));
        assert_eq!(reloaded.load_task(&t.task_id).unwrap(), t);
        assert_eq!(reloaded.load_run(&run().run_id).unwrap(), run());
        assert_eq!(
            reloaded
                .load_checkpoint(&RunId::parse("run-1").unwrap())
                .unwrap(),
            checkpoint()
        );
        assert_eq!(
            reloaded
                .load_journal()
                .unwrap()
                .unresolved_for_run(&RunId::parse("run-1").unwrap())
                .len(),
            0
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
