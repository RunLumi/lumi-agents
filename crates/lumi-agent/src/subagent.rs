//! Subagent delegation (Spec 24 §24.9–24.11): a bounded child loop with
//! an explicit goal, narrowed capabilities, carved budget, and deadline.
//!
//! Guarantees enforced here (not by convention):
//! - the child only sees tools whose capability is in the allowed set —
//!   a proposal outside the set fails the run honestly (unknown tool);
//! - the child budget is the spec's carve-out, independent of the
//!   parent's remaining budget;
//! - proposed side effects still pass the same orchestrator gate and
//!   parent policy (§24.10) — the child cannot approve anything, and
//!   `ApprovalNeeded` surfaces as an honest failure to the caller.
//!
//! Optional by default (§24.6): the parent chooses when to delegate.

use crate::loop_impl::{AgentLoop, AgentRunOutcome};
use crate::planner::Planner;
use crate::tools::AgentTool;
use lumi_audit::VerificationEnvironment;
use lumi_protocol::{Budget, Capability, Principal, TaskId};
use lumi_state::StateStore;
use lumi_workspaces::Workspace;
use std::collections::BTreeSet;

/// The bounded delegation contract (§24.9): every field is required.
#[derive(Debug, Clone)]
pub struct SubagentSpec {
    /// Explicit, narrow goal (never the parent goal verbatim).
    pub goal: String,
    /// Capability whitelist — the child sees ONLY tools whose capability
    /// is in this set; anything else fails the child honestly.
    pub allowed_capabilities: BTreeSet<Capability>,
    /// Carved-out budget (actions/deadline bounds) — independent of the
    /// parent's remaining budget.
    pub budget: Budget,
    /// Turn deadline (§24.9 deadline).
    pub max_turns: u32,
    /// Structured-output hint appended to the child's instructions
    /// (§24.11: output SHOULD be structured).
    pub output_hint: Option<String>,
}

/// The delegation outcome (§24.11: the parent integrates results).
///
/// Carries the §24.13 economics fields the parent needs to judge
/// whether delegation paid for itself: wall-clock duration, executed
/// action count, and model spend are measured, not self-reported.
#[derive(Debug, Clone)]
pub struct SubagentOutcome {
    pub completed: bool,
    pub answer: Option<String>,
    pub failure: Option<String>,
    pub turns: u32,
    /// Wall-clock duration of the delegation (§24.13 latency).
    pub duration_ms: u64,
    /// Actions the child actually executed (§24.13 cost proxy).
    pub actions: u32,
    /// Model spend attributed to the child (§24.13 cost).
    pub model_cost_micro_usd: u64,
    /// Executed actions that touched vision (§24.13 quality signal:
    /// high vision share suggests a missing semantic adapter).
    pub vision_actions: u32,
    /// External writes the child performed (blast-radius signal).
    pub external_writes: u32,
}

/// Runs one bounded subagent. The caller keeps parent responsibility for
/// integrating the answer and final verification (§24.11).
///
/// Tool narrowing: tools whose capability is outside
/// [`SubagentSpec::allowed_capabilities`] are NOT registered for the
/// child — a proposal naming them fails the child honestly (unknown
/// tool) instead of executing anything.
#[allow(clippy::too_many_arguments)] // explicit delegation beats a config struct
pub fn run_subagent<S: StateStore>(
    orchestrator: &mut lumi_orchestrator::Orchestrator<S>,
    planner: &dyn Planner,
    available_tools: Vec<Box<dyn AgentTool + '_>>,
    spec: &SubagentSpec,
    workspace: &Workspace,
    env: &dyn VerificationEnvironment,
    parent_principal: &Principal,
    parent_task_id: &TaskId,
    run_id: &lumi_protocol::RunId,
) -> SubagentOutcome {
    // Narrow: register only tools whose capability is allowed.
    let tools: Vec<Box<dyn AgentTool + '_>> = available_tools
        .into_iter()
        .filter(|tool| spec.allowed_capabilities.contains(&tool.capability()))
        .collect();

    // Structured-output hint + injection hygiene frame.
    let goal = match &spec.output_hint {
        Some(hint) => format!("{}\n\nOUTPUT REQUIREMENT: {}", spec.goal, hint),
        None => spec.goal.clone(),
    };

    let mut child = AgentLoop::new(
        orchestrator,
        planner,
        tools,
        workspace,
        env,
        parent_principal.clone(),
        parent_task_id.clone(),
        run_id.clone(),
        spec.budget.clone(),
        &goal,
        None,
    );
    child.max_turns = spec.max_turns;

    let started = std::time::Instant::now();
    let outcome = child.run();
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    // §24.13: economics come from the loop's own consumption ledger,
    // not from the model's claims.
    let consumed = *child.consumed();

    match outcome {
        AgentRunOutcome::Completed { answer, turns } => SubagentOutcome {
            completed: true,
            answer: Some(answer),
            failure: None,
            turns,
            duration_ms,
            actions: consumed.actions,
            model_cost_micro_usd: consumed.model_cost_micro_usd,
            vision_actions: consumed.vision_actions,
            external_writes: consumed.external_writes,
        },
        AgentRunOutcome::WaitingApproval { reason, .. } => SubagentOutcome {
            completed: false,
            answer: None,
            // §24.10: a subagent cannot approve its own consequential
            // action — the pause is surfaced to the parent as a failure
            // of this delegation, for the human queue.
            failure: Some(format!("subagent needs approval: {reason}")),
            turns: child.turns(),
            duration_ms,
            actions: consumed.actions,
            model_cost_micro_usd: consumed.model_cost_micro_usd,
            vision_actions: consumed.vision_actions,
            external_writes: consumed.external_writes,
        },
        AgentRunOutcome::Failed { reason, .. } => SubagentOutcome {
            completed: false,
            answer: None,
            failure: Some(reason),
            turns: child.turns(),
            duration_ms,
            actions: consumed.actions,
            model_cost_micro_usd: consumed.model_cost_micro_usd,
            vision_actions: consumed.vision_actions,
            external_writes: consumed.external_writes,
        },
    }
}
