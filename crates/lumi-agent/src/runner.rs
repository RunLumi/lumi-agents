//! Durable task runner: binds one persisted task to the planning loop.
//!
//! This is the integration seam between the work-mode [`AgentLoop`] and
//! durable state: the task (with its project binding) is loaded from the
//! state store, transitioned to RUNNING with a recorded [`Run`], executed
//! through the orchestrator gate, and the terminal outcome (completed,
//! waiting approval, failed) is persisted back. Creating a task never
//! implies work: only this runner moves a task forward, and every
//! transition it writes is observable in the durable store.

use crate::loop_impl::{AgentLoop, AgentRunOutcome};
use crate::planner::Planner;
use crate::tools::AgentTool;
use lumi_audit::VerificationEnvironment;
use lumi_orchestrator::Orchestrator;
use lumi_protocol::{
    ConsumedBudget, ErrorEnvelope, Run, RunId, RunState, Task, TaskId, TaskStatus, Timestamp,
};
use lumi_state::StateStore;
use lumi_workspaces::Workspace;

/// Terminal (or pausing) status of one runner invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRunStatus {
    /// The planner produced a final answer after verified steps.
    Completed,
    /// A step needs a scoped human approval before it can execute.
    WaitingApproval,
    /// Terminal failure (policy denial, unverified step, budget/turn
    /// exhaustion, ambiguity, planner error).
    Failed,
}

/// What one runner invocation did, for callers (desktop shell, dogfood
/// harness, evals) that surface progress and evidence.
#[derive(Debug, Clone)]
pub struct TaskRunOutcome {
    pub status: TaskRunStatus,
    pub run_id: RunId,
    /// Final answer text when [`TaskRunStatus::Completed`].
    pub answer: Option<String>,
    /// Failure reason when [`TaskRunStatus::Failed`]; the approval
    /// request detail when [`TaskRunStatus::WaitingApproval`].
    pub failure: Option<String>,
    /// The exact normalized action awaiting approval (present only for
    /// [`TaskRunStatus::WaitingApproval`]): the host requests a scoped
    /// human decision against THIS action, digest-bound.
    pub pending_action: Option<Box<lumi_protocol::ActionProposal>>,
    pub turns: u32,
    pub budgets: ConsumedBudget,
}

/// Runs one persisted task end-to-end through the planning loop.
///
/// Transitions persisted here: task `→ RUNNING` (+ [`Run`] record) before
/// the first model call; `→ COMPLETED` / `→ WAITING_APPROVAL` / `→ FAILED`
/// after the loop returns. A `WAITING_APPROVAL` outcome leaves the run
/// open (no `ended_at`): the host obtains the scoped human approval and
/// resumes through [`AgentLoop::continue_with_approval`].
///
/// # Errors
/// Durable store failures (sticky: the caller sees the error and may
/// retry the persistence step; the loop itself reports its own failures
/// through [`TaskRunOutcome::Failed`]).
#[allow(clippy::too_many_arguments)] // explicit wiring beats a config struct here
pub fn run_persisted_task<S: StateStore>(
    orchestrator: &mut Orchestrator<S>,
    planner: &dyn Planner,
    tools: Vec<Box<dyn AgentTool + '_>>,
    workspace: &Workspace,
    env: &dyn VerificationEnvironment,
    task: &Task,
    resume_context: Option<&str>,
    run_id: RunId,
) -> Result<TaskRunOutcome, String> {
    let mut persisted = task.clone();
    persisted.status = TaskStatus::Running;
    orchestrator
        .store
        .save_task(&persisted)
        .map_err(|e| format!("persist task RUNNING: {e}"))?;

    let mut run = Run {
        run_id: run_id.clone(),
        task_id: persisted.task_id.clone(),
        runtime_version: env!("CARGO_PKG_VERSION").to_owned(),
        workflow_version: None,
        // The planner's provider selection is recorded by the model layer
        // per call; the runner records the instance family it was handed.
        selected_providers: vec![planner.provider_family().to_owned()],
        started_at: Timestamp::now(),
        ended_at: None,
        state: RunState::Executing,
        budgets_consumed: ConsumedBudget::default(),
        failure: None,
    };
    orchestrator
        .store
        .save_run(&run)
        .map_err(|e| format!("persist run: {e}"))?;

    let goal = persisted.goal.clone();
    let mut loop_borrow = AgentLoop::new(
        orchestrator,
        planner,
        tools,
        workspace,
        env,
        persisted.principal.clone(),
        persisted.task_id.clone(),
        run_id.clone(),
        persisted.budget.clone(),
        &goal,
        resume_context,
    );
    let outcome = loop_borrow.run();
    let budgets = *loop_borrow.consumed();
    let turns = loop_borrow.turns();
    drop(loop_borrow);

    let mapped = match outcome {
        AgentRunOutcome::Completed { answer, turns } => {
            persisted.status = TaskStatus::Completed;
            run.ended_at = Some(Timestamp::now());
            run.state = RunState::Verifying;
            run.budgets_consumed = budgets;
            TaskRunOutcome {
                status: TaskRunStatus::Completed,
                run_id,
                answer: Some(answer),
                failure: None,
                pending_action: None,
                turns,
                budgets,
            }
        }
        AgentRunOutcome::WaitingApproval {
            action,
            action_digest,
            reason,
        } => {
            persisted.status = TaskStatus::WaitingApproval;
            run.budgets_consumed = budgets;
            // Run stays open: an approved resume continues this run.
            TaskRunOutcome {
                status: TaskRunStatus::WaitingApproval,
                run_id,
                answer: None,
                failure: Some(format!("approval {action_digest} required: {reason}")),
                pending_action: Some(action),
                turns,
                budgets,
            }
        }
        AgentRunOutcome::Failed { category, reason } => {
            persisted.status = TaskStatus::Failed;
            run.ended_at = Some(Timestamp::now());
            run.state = RunState::Verifying;
            run.budgets_consumed = budgets;
            run.failure = Some(ErrorEnvelope::new(category, reason.clone()));
            TaskRunOutcome {
                status: TaskRunStatus::Failed,
                run_id,
                answer: None,
                failure: Some(reason),
                pending_action: None,
                turns,
                budgets,
            }
        }
    };
    orchestrator
        .store
        .save_run(&run)
        .map_err(|e| format!("persist run outcome: {e}"))?;
    orchestrator
        .store
        .save_task(&persisted)
        .map_err(|e| format!("persist task outcome: {e}"))?;
    Ok(mapped)
}

/// Loads one persisted task by id.
///
/// # Errors
/// Durable store failure or unknown task.
pub fn load_task<S: StateStore>(
    orchestrator: &Orchestrator<S>,
    task_id: &TaskId,
) -> Result<Task, String> {
    orchestrator
        .store
        .load_task(task_id)
        .map_err(|e| format!("load task: {e}"))
}
