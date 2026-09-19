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
use lumi_protocol::{ActionId, RunId, Task, TaskStatus, TenantId, Timestamp};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

/// Storage errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound(String),
    Io(String),
    Serde(String),
    StaleWriter { expected: u64, actual: u64 },
    GenerationExhausted,
    TransactionBusy(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(what) => write!(f, "not found: {what}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Serde(e) => write!(f, "serialization error: {e}"),
            Self::StaleWriter { expected, actual } => {
                write!(
                    f,
                    "stale state writer: expected generation {expected}, found {actual}"
                )
            }
            Self::GenerationExhausted => write!(f, "state generation exhausted"),
            Self::TransactionBusy(path) => write!(
                f,
                "state transaction lock already held at {path}; operator recovery required"
            ),
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
    /// Loads the durable retry admissions for one action scope.
    fn load_retry_attempts(
        &self,
        tenant_id: &TenantId,
        run_id: &RunId,
        action_id: &ActionId,
    ) -> Result<Option<u32>, StoreError>;
    /// Persists the next retry admission before a retry is returned to the
    /// caller. Implementations must use the same transaction/generation
    /// boundary as other state writes.
    fn save_retry_attempts(
        &mut self,
        tenant_id: &TenantId,
        run_id: &RunId,
        action_id: &ActionId,
        attempts: u32,
    ) -> Result<(), StoreError>;
}

/// Durable retry admission counter scoped to tenant/run/action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryRecord {
    pub tenant_id: TenantId,
    pub run_id: RunId,
    pub action_id: ActionId,
    pub attempts: u32,
}

/// Full durable state document (serialized by [`JsonStateStore`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedState {
    /// Monotonic generation for compare-before-save state transactions.
    #[serde(default)]
    pub journal_generation: u64,
    pub tasks: Vec<Task>,
    pub runs: Vec<lumi_protocol::Run>,
    pub checkpoints: Vec<Checkpoint>,
    pub pre_actions: Vec<(RunId, PreActionCheckpoint)>,
    pub journal: Vec<crate::journal::SideEffectRecord>,
    #[serde(default)]
    pub retry_records: Vec<RetryRecord>,
}

/// In-memory reference store (also the unit-test surface).
///
/// Handles share the same underlying state (like connections to one
/// database), so a "restart" from a clone observes all prior writes.
#[derive(Debug, Default)]
pub struct InMemoryStateStore {
    state: std::rc::Rc<std::cell::RefCell<PersistedState>>,
    observed_generation: Cell<Option<u64>>,
}

impl Clone for InMemoryStateStore {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            // A clone is a new logical writer. It must observe the shared
            // generation before its first mutation.
            observed_generation: Cell::new(None),
        }
    }
}

impl InMemoryStateStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn observe_generation(&self, generation: u64) {
        self.observed_generation.set(Some(generation));
    }

    fn transact(
        &mut self,
        mutate: impl FnOnce(&mut PersistedState) -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        let mut state = self.state.borrow_mut();
        let actual = state.journal_generation;
        if let Some(expected) = self.observed_generation.get() {
            if expected != actual {
                return Err(StoreError::StaleWriter { expected, actual });
            }
        } else {
            self.observed_generation.set(Some(actual));
        }
        let next_generation = actual
            .checked_add(1)
            .ok_or(StoreError::GenerationExhausted)?;
        mutate(&mut state)?;
        state.journal_generation = next_generation;
        self.observed_generation.set(Some(state.journal_generation));
        Ok(())
    }
}

impl StateStore for InMemoryStateStore {
    fn save_task(&mut self, task: &Task) -> Result<(), StoreError> {
        self.transact(|state| {
            state.tasks.retain(|t| t.task_id != task.task_id);
            state.tasks.push(task.clone());
            Ok(())
        })
    }

    fn load_task(&self, task_id: &lumi_protocol::TaskId) -> Result<Task, StoreError> {
        let state = self.state.borrow();
        self.observe_generation(state.journal_generation);
        state
            .tasks
            .iter()
            .find(|t| &t.task_id == task_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("task {task_id}")))
    }

    fn save_run(&mut self, run: &lumi_protocol::Run) -> Result<(), StoreError> {
        self.transact(|state| {
            state.runs.retain(|r| r.run_id != run.run_id);
            state.runs.push(run.clone());
            Ok(())
        })
    }

    fn load_run(&self, run_id: &RunId) -> Result<lumi_protocol::Run, StoreError> {
        let state = self.state.borrow();
        self.observe_generation(state.journal_generation);
        state
            .runs
            .iter()
            .find(|r| &r.run_id == run_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("run {run_id}")))
    }

    fn save_checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), StoreError> {
        self.transact(|state| {
            state.checkpoints.retain(|c| c.run_id != checkpoint.run_id);
            state.checkpoints.push(checkpoint.clone());
            Ok(())
        })
    }

    fn load_checkpoint(&self, run_id: &RunId) -> Result<Checkpoint, StoreError> {
        let state = self.state.borrow();
        self.observe_generation(state.journal_generation);
        state
            .checkpoints
            .iter()
            .find(|c| &c.run_id == run_id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(format!("checkpoint for {run_id}")))
    }

    fn save_pre_action(&mut self, pre: &PreActionCheckpoint) -> Result<(), StoreError> {
        // Pre-action checkpoints are keyed by run id (parsed from the
        // serialized action).
        let run_id = pre.run_id();
        self.transact(|state| {
            state.pre_actions.retain(|(rid, _)| rid != &run_id);
            state.pre_actions.push((run_id, pre.clone()));
            Ok(())
        })
    }

    fn load_pre_action(&self, run_id: &RunId) -> Result<PreActionCheckpoint, StoreError> {
        let state = self.state.borrow();
        self.observe_generation(state.journal_generation);
        state
            .pre_actions
            .iter()
            .find(|(rid, _)| rid == run_id)
            .map(|(_, pre)| pre.clone())
            .ok_or_else(|| StoreError::NotFound(format!("pre-action checkpoint for {run_id}")))
    }

    fn save_journal(&mut self, journal: &SideEffectJournal) -> Result<(), StoreError> {
        self.transact(|state| {
            state.journal = journal.all().iter().map(|r| (*r).clone()).collect();
            Ok(())
        })
    }

    fn load_journal(&self) -> Result<SideEffectJournal, StoreError> {
        let state = self.state.borrow();
        self.observe_generation(state.journal_generation);
        let mut journal = SideEffectJournal::new();
        journal
            .restore(state.journal.clone())
            .map_err(StoreError::Serde)?;
        Ok(journal)
    }

    fn load_retry_attempts(
        &self,
        tenant_id: &TenantId,
        run_id: &RunId,
        action_id: &ActionId,
    ) -> Result<Option<u32>, StoreError> {
        let state = self.state.borrow();
        self.observe_generation(state.journal_generation);
        Ok(state
            .retry_records
            .iter()
            .find(|record| {
                &record.tenant_id == tenant_id
                    && &record.run_id == run_id
                    && &record.action_id == action_id
            })
            .map(|record| record.attempts))
    }

    fn save_retry_attempts(
        &mut self,
        tenant_id: &TenantId,
        run_id: &RunId,
        action_id: &ActionId,
        attempts: u32,
    ) -> Result<(), StoreError> {
        self.transact(|state| {
            state.retry_records.retain(|record| {
                &record.tenant_id != tenant_id
                    || &record.run_id != run_id
                    || &record.action_id != action_id
            });
            state.retry_records.push(RetryRecord {
                tenant_id: tenant_id.clone(),
                run_id: run_id.clone(),
                action_id: action_id.clone(),
                attempts,
            });
            Ok(())
        })
    }
}

struct StateLock {
    path: PathBuf,
}

impl Drop for StateLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// File-backed snapshot store with atomic replacement and compare-before-save
/// transactions. A lock file that survives a crash is deliberately not
/// reclaimed automatically; an operator must remove it after inspection.
pub struct JsonStateStore {
    path: PathBuf,
    observed_generation: Cell<Option<u64>>,
}

impl JsonStateStore {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self {
            path,
            observed_generation: Cell::new(None),
        }
    }

    fn lock_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.lock", self.path.display()))
    }

    fn acquire_lock(&self) -> Result<StateLock, StoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StoreError::Io(e.to_string()))?;
        }
        let lock_path = self.lock_path();
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut lock) => {
                lock.write_all(b"lumi-state transaction lock\n")
                    .map_err(|e| StoreError::Io(e.to_string()))?;
                lock.sync_all().map_err(|e| StoreError::Io(e.to_string()))?;
                Ok(StateLock { path: lock_path })
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(StoreError::TransactionBusy(lock_path.display().to_string()))
            }
            Err(error) => Err(StoreError::Io(error.to_string())),
        }
    }

    fn observe_generation(&self, generation: u64) {
        self.observed_generation.set(Some(generation));
    }

    fn transact(
        &mut self,
        mutate: impl FnOnce(&mut PersistedState) -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        let _lock = self.acquire_lock()?;
        let mut state = self.read()?;
        let actual = state.journal_generation;
        if let Some(expected) = self.observed_generation.get() {
            if expected != actual {
                return Err(StoreError::StaleWriter { expected, actual });
            }
        } else {
            self.observed_generation.set(Some(actual));
        }
        let next_generation = actual
            .checked_add(1)
            .ok_or(StoreError::GenerationExhausted)?;
        mutate(&mut state)?;
        state.journal_generation = next_generation;
        self.write(&state)?;
        self.observed_generation.set(Some(state.journal_generation));
        Ok(())
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
        self.transact(|state| {
            state.tasks.retain(|t| t.task_id != task.task_id);
            state.tasks.push(task.clone());
            Ok(())
        })
    }

    fn load_task(&self, task_id: &lumi_protocol::TaskId) -> Result<Task, StoreError> {
        let state = self.read()?;
        self.observe_generation(state.journal_generation);
        state
            .tasks
            .into_iter()
            .find(|t| &t.task_id == task_id)
            .ok_or_else(|| StoreError::NotFound(format!("task {task_id}")))
    }

    fn save_run(&mut self, run: &lumi_protocol::Run) -> Result<(), StoreError> {
        self.transact(|state| {
            state.runs.retain(|r| r.run_id != run.run_id);
            state.runs.push(run.clone());
            Ok(())
        })
    }

    fn load_run(&self, run_id: &RunId) -> Result<lumi_protocol::Run, StoreError> {
        let state = self.read()?;
        self.observe_generation(state.journal_generation);
        state
            .runs
            .into_iter()
            .find(|r| &r.run_id == run_id)
            .ok_or_else(|| StoreError::NotFound(format!("run {run_id}")))
    }

    fn save_checkpoint(&mut self, checkpoint: &Checkpoint) -> Result<(), StoreError> {
        self.transact(|state| {
            state.checkpoints.retain(|c| c.run_id != checkpoint.run_id);
            state.checkpoints.push(checkpoint.clone());
            Ok(())
        })
    }

    fn load_checkpoint(&self, run_id: &RunId) -> Result<Checkpoint, StoreError> {
        let state = self.read()?;
        self.observe_generation(state.journal_generation);
        state
            .checkpoints
            .into_iter()
            .find(|c| &c.run_id == run_id)
            .ok_or_else(|| StoreError::NotFound(format!("checkpoint for {run_id}")))
    }

    fn save_pre_action(&mut self, pre: &PreActionCheckpoint) -> Result<(), StoreError> {
        let run_id = pre.run_id();
        self.transact(|state| {
            state.pre_actions.retain(|(rid, _)| rid != &run_id);
            state.pre_actions.push((run_id, pre.clone()));
            Ok(())
        })
    }

    fn load_pre_action(&self, run_id: &RunId) -> Result<PreActionCheckpoint, StoreError> {
        let state = self.read()?;
        self.observe_generation(state.journal_generation);
        state
            .pre_actions
            .into_iter()
            .find(|(rid, _)| rid == run_id)
            .map(|(_, pre)| pre)
            .ok_or_else(|| StoreError::NotFound(format!("pre-action checkpoint for {run_id}")))
    }

    fn save_journal(&mut self, journal: &SideEffectJournal) -> Result<(), StoreError> {
        self.transact(|state| {
            state.journal = journal.all().iter().map(|r| (*r).clone()).collect();
            Ok(())
        })
    }

    fn load_journal(&self) -> Result<SideEffectJournal, StoreError> {
        let state = self.read()?;
        self.observe_generation(state.journal_generation);
        let mut journal = SideEffectJournal::new();
        journal.restore(state.journal).map_err(StoreError::Serde)?;
        Ok(journal)
    }

    fn load_retry_attempts(
        &self,
        tenant_id: &TenantId,
        run_id: &RunId,
        action_id: &ActionId,
    ) -> Result<Option<u32>, StoreError> {
        let state = self.read()?;
        self.observe_generation(state.journal_generation);
        Ok(state
            .retry_records
            .iter()
            .find(|record| {
                &record.tenant_id == tenant_id
                    && &record.run_id == run_id
                    && &record.action_id == action_id
            })
            .map(|record| record.attempts))
    }

    fn save_retry_attempts(
        &mut self,
        tenant_id: &TenantId,
        run_id: &RunId,
        action_id: &ActionId,
        attempts: u32,
    ) -> Result<(), StoreError> {
        self.transact(|state| {
            state.retry_records.retain(|record| {
                &record.tenant_id != tenant_id
                    || &record.run_id != run_id
                    || &record.action_id != action_id
            });
            state.retry_records.push(RetryRecord {
                tenant_id: tenant_id.clone(),
                run_id: run_id.clone(),
                action_id: action_id.clone(),
                attempts,
            });
            Ok(())
        })
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
    /// Legacy caller-supplied resolutions. These are observations only and
    /// MUST NOT clear an unresolved durable journal record; a durable journal
    /// transition is the sole proof accepted by resume.
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
    // continuation (spec 02 §2.7, spec 18 §18.11). Caller-supplied
    // resolutions are observations only; they cannot clear an unresolved
    // durable record. The orchestrator must persist a validated transition
    // through `resolve_ambiguity` before a later resume can continue.
    let unresolved = journal.unresolved_for_run(&run.run_id);
    for record in unresolved {
        plan.push(ResumeAction::ReverifySideEffect {
            action_id: record.action_id.clone(),
        });
    }
    if !plan.is_empty() {
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
        ActionId, AuthenticationStrength, Capability, ConsumedBudget, Principal, PrincipalId,
        PrincipalKind, ResourceRef, ResourceType, RiskClass, RunId, Target, TenantId,
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
            project_binding: None,
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

    fn action() -> lumi_protocol::ActionProposal {
        lumi_protocol::ActionProposal::builder(
            ActionId::parse("a-store-journal").unwrap(),
            lumi_protocol::TaskId::parse("task-1").unwrap(),
            RunId::parse("run-1").unwrap(),
            task(TaskStatus::Running).principal,
            Capability::well_known(lumi_protocol::capabilities::EMAIL_SEND),
            ResourceRef {
                resource_type: ResourceType::well_known(ResourceType::EMAIL_MESSAGE),
                id: "outbound/store".to_owned(),
                sensitivity: None,
            },
            Target::canonical("mailto:store@example.test"),
            "send_customer_email",
            RiskClass::Communication,
        )
        .idempotency(lumi_protocol::Idempotency {
            key: Some("store-key".to_owned()),
            semantics: lumi_protocol::IdempotencySemantics::ClientKey,
        })
        .unwrap()
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

        // A caller-supplied resolution cannot clear the durable Proposed
        // record; resume must still require an external re-verification.
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
        assert_eq!(
            plan,
            vec![ResumeAction::ReverifySideEffect {
                action_id: action.action_id.clone()
            }]
        );
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
            task_id: None,
            run_id: RunId::parse("run-1").unwrap(),
            tenant_id: None,
            principal_id: None,
            idempotency_key: None,
            action_digest: "d".to_owned(),
            risk_class: None,
            resource_sensitivity: None,
            idempotency_scope_digest: None,
            verifier_plan_digest: None,
            status: SideEffectStatus::Succeeded,
            proposed_at: ts(1),
            resolved_at: Some(ts(2)),
            result_digest: Some("r".to_owned()),
            failure: None,
        };
        journal.restore(vec![record]).unwrap();
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

    #[test]
    fn in_memory_stale_writer_is_rejected_and_unrelated_write_keeps_journal() {
        let base = InMemoryStateStore::new();
        let mut writer_a = base.clone();
        let mut writer_b = base.clone();
        let action = action();
        let mut journal = SideEffectJournal::new();
        journal.propose(&action, ts(1));

        // Both handles observe generation zero before either writes.
        writer_a.load_journal().unwrap();
        writer_b.load_journal().unwrap();
        writer_a.save_journal(&journal).unwrap();
        assert!(matches!(
            writer_b.save_journal(&journal),
            Err(StoreError::StaleWriter { .. })
        ));

        // A writer that owns the current generation may update an unrelated
        // task without dropping the journal.
        writer_a.save_task(&task(TaskStatus::Running)).unwrap();
        let reloaded = writer_a.load_journal().unwrap();
        assert!(reloaded.get(&action.action_id).is_some());
    }

    #[test]
    fn json_store_stale_writer_is_rejected() {
        let dir = std::env::temp_dir().join(format!(
            "lumi-state-stale-{}-{}",
            std::process::id(),
            Timestamp::now().epoch_seconds()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let mut writer_a = JsonStateStore::new(path.clone());
        let mut writer_b = JsonStateStore::new(path.clone());
        let mut journal = SideEffectJournal::new();
        journal.propose(&action(), ts(1));
        writer_a.load_journal().unwrap();
        writer_b.load_journal().unwrap();
        writer_a.save_journal(&journal).unwrap();
        assert!(matches!(
            writer_b.save_journal(&journal),
            Err(StoreError::StaleWriter { .. })
        ));
        writer_a.save_task(&task(TaskStatus::Running)).unwrap();
        assert_eq!(
            JsonStateStore::new(path.clone())
                .load_journal()
                .unwrap()
                .all()
                .len(),
            1
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn json_store_lock_already_held_fails_without_mutation() {
        let dir = std::env::temp_dir().join(format!(
            "lumi-state-lock-{}-{}",
            std::process::id(),
            Timestamp::now().epoch_seconds()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let lock_path = PathBuf::from(format!("{}.lock", path.display()));
        std::fs::write(&lock_path, b"held by another writer").unwrap();
        let mut store = JsonStateStore::new(path.clone());
        assert!(matches!(
            store.save_task(&task(TaskStatus::Running)),
            Err(StoreError::TransactionBusy(_))
        ));
        assert!(!path.exists(), "busy transaction must not create state");
        std::fs::remove_file(lock_path).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn generation_overflow_fails_before_mutation() {
        let dir = std::env::temp_dir().join(format!(
            "lumi-state-generation-{}-{}",
            std::process::id(),
            Timestamp::now().epoch_seconds()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(
            &path,
            serde_json::json!({
                "journal_generation": u64::MAX,
                "tasks": [],
                "runs": [],
                "checkpoints": [],
                "pre_actions": [],
                "journal": []
            })
            .to_string(),
        )
        .unwrap();
        let mut store = JsonStateStore::new(path.clone());
        store.load_journal().unwrap();
        assert!(matches!(
            store.save_task(&task(TaskStatus::Running)),
            Err(StoreError::GenerationExhausted)
        ));
        let persisted: PersistedState =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(persisted.tasks.is_empty());
        assert_eq!(persisted.journal_generation, u64::MAX);
        std::fs::remove_dir_all(&dir).ok();
    }
}
