//! The agent loop: plan → propose → gate → execute → observe.
//!
//! The loop holds the conversation (trusted system prompt + goal +
//! observations), plans via the [`Planner`], turns proposed tool calls
//! into proposals through the tools themselves, and pushes each through
//! the orchestrator gate. The verification environment is supplied by
//! the host (it knows how to observe the world independently).

use crate::planner::Planner;
use crate::tools::AgentTool;
use lumi_models::request::ModelMessage;
use lumi_orchestrator::Orchestrator;
use lumi_protocol::{ActionProposal, Budget, ConsumedBudget, RunId, TaskId, Timestamp};

use std::collections::HashMap;

/// Why the loop stopped.
#[derive(Debug, Clone)]
pub enum AgentRunOutcome {
    /// The planner produced a final answer after verified steps.
    Completed { answer: String, turns: u32 },
    /// A step needs a scoped human approval. Grant one via
    /// [`AgentLoop::continue_with_approval`] and resume.
    WaitingApproval {
        action: Box<ActionProposal>,
        action_digest: String,
        reason: String,
    },
    /// Terminal failure: policy denial, unverified step, budget/turn
    /// exhaustion, ambiguity, or planner error. The category is the
    /// canonical taxonomy entry so durable records and evals improve the
    /// correct layer.
    Failed {
        category: lumi_protocol::FailureCategory,
        reason: String,
    },
}

/// The work-mode agent loop bound to one orchestrator, workspace, task,
/// verification environment, and principal. Generic over the durable
/// state store: the desktop shell runs it against `JsonStateStore`, tests
/// against `InMemoryStateStore`.
pub struct AgentLoop<'a, S: lumi_state::StateStore> {
    pub orchestrator: &'a mut Orchestrator<S>,
    pub planner: &'a dyn Planner,
    pub tools: HashMap<String, Box<dyn AgentTool + 'a>>,
    pub workspace: &'a lumi_workspaces::Workspace,
    pub env: &'a dyn lumi_audit::VerificationEnvironment,
    pub principal: lumi_protocol::Principal,
    pub task_id: TaskId,
    pub run_id: RunId,
    pub budget: Budget,
    pub consumed: ConsumedBudget,
    pub max_turns: u32,
    messages: Vec<ModelMessage>,
    turns: u32,
    /// Observation payload of the last executed tool (fed back once).
    last_observation: Option<String>,
    /// The pending tool call awaiting an approval decision.
    pending: Option<(&'static str, Box<ActionProposal>)>,
    done: bool,
}

impl<'a, S: lumi_state::StateStore> AgentLoop<'a, S> {
    /// Creates a loop with the trusted system prompt and the user goal.
    /// Takes ownership of the tools (they live as long as the loop).
    #[must_use]
    #[allow(clippy::too_many_arguments)] // explicit wiring beats a config struct here
    pub fn new(
        orchestrator: &'a mut Orchestrator<S>,
        planner: &'a dyn Planner,
        tools: Vec<Box<dyn AgentTool + 'a>>,
        workspace: &'a lumi_workspaces::Workspace,
        env: &'a dyn lumi_audit::VerificationEnvironment,
        principal: lumi_protocol::Principal,
        task_id: TaskId,
        run_id: RunId,
        budget: Budget,
        goal: &str,
        resume_context: Option<&str>,
    ) -> Self {
        let mut tool_map = HashMap::new();
        let mut names = Vec::new();
        for tool in tools {
            names.push(tool.name());
            tool_map.insert(tool.name().to_owned(), tool);
        }
        let system =
            crate::planner::system_prompt(goal, &workspace.root().display().to_string(), &names);
        // Durable progress from a previous interrupted attempt is
        // RUNTIME-PROVIDED context (our own ledger, trusted provenance):
        // it tells the planner what already happened so a re-run
        // continues instead of redoing work.
        let user_message = match resume_context {
            Some(context) => format!("{goal}\n\n{context}"),
            None => goal.to_owned(),
        };
        Self {
            orchestrator,
            planner,
            tools: tool_map,
            workspace,
            env,
            principal,
            task_id,
            run_id,
            budget,
            consumed: ConsumedBudget::default(),
            max_turns: 16,
            messages: vec![
                ModelMessage::System { content: system },
                ModelMessage::User {
                    content: user_message,
                },
            ],
            turns: 0,
            last_observation: None,
            pending: None,
            done: false,
        }
    }

    /// Runs the loop until completion, a needed approval, a failure, or
    /// the turn/budget bound.
    #[allow(clippy::too_many_lines)]
    pub fn run(&mut self) -> AgentRunOutcome {
        if self.done {
            return AgentRunOutcome::Failed {
                category: lumi_protocol::FailureCategory::Crash,
                reason: "loop already finished".to_owned(),
            };
        }
        // Destructure for disjoint field borrows inside the loop.
        let AgentLoop {
            orchestrator,
            planner,
            tools,
            workspace,
            env,
            principal,
            task_id,
            run_id,
            budget,
            consumed,
            max_turns,
            messages,
            turns,
            last_observation,
            pending,
            done,
        } = self;

        loop {
            if *turns >= *max_turns {
                *done = true;
                return AgentRunOutcome::Failed {
                    category: lumi_protocol::FailureCategory::BudgetExceeded,
                    reason: format!("turn budget exhausted ({max_turns})"),
                };
            }
            if let Some(dimension) = budget.check(consumed, Timestamp::now()) {
                *done = true;
                return AgentRunOutcome::Failed {
                    category: lumi_protocol::FailureCategory::BudgetExceeded,
                    reason: format!("task budget exhausted: {dimension:?}"),
                };
            }
            *turns += 1;
            if let Some(observation) = last_observation.take() {
                messages.push(ModelMessage::ToolResult {
                    tool_call_id: "last".to_owned(),
                    content: observation,
                });
            }

            // 1. Plan: model proposes tool calls or a final answer.
            let response = match planner.plan(messages) {
                Ok(response) => response,
                Err(e) => {
                    *done = true;
                    return AgentRunOutcome::Failed {
                        category: e.category,
                        reason: format!("planner error: {e}"),
                    };
                }
            };

            // 2. Final answer (no tool calls).
            if response.tool_calls.is_empty() {
                *done = true;
                let answer = response.content.unwrap_or_default();
                messages.push(ModelMessage::Assistant {
                    content: answer.clone(),
                });
                return AgentRunOutcome::Completed {
                    answer,
                    turns: *turns,
                };
            }

            // 3. Execute proposed tool calls through the gate.
            let mut retry_after_observation = false;
            for call in &response.tool_calls {
                let Some(tool) = tools.get(call.name.as_str()) else {
                    *done = true;
                    return AgentRunOutcome::Failed {
                        category: lumi_protocol::FailureCategory::ModelFormat,
                        reason: format!("planner proposed unknown tool {:?}", call.name),
                    };
                };

                let ctx = crate::ToolContext {
                    workspace,
                    task_id,
                    principal,
                };
                let action = match tool.build_proposal(call.arguments.clone(), &ctx, run_id) {
                    Ok(action) => action,
                    Err(e) => {
                        // Model-format error: feed back once as an
                        // observation so the model can correct itself.
                        last_observation.replace(format!(
                            "TOOL_ERROR: invalid arguments for {}: {e}",
                            call.name
                        ));
                        retry_after_observation = true;
                        break;
                    }
                };

                let outcome = {
                    let ctx = crate::ToolContext {
                        workspace,
                        task_id,
                        principal,
                    };
                    let mut executor = |authorized: &ActionProposal| tool.execute(authorized, &ctx);
                    orchestrator.execute_step(&action, None, budget, consumed, *env, &mut executor)
                };
                match outcome {
                    lumi_orchestrator::StepOutcome::VerifiedSuccess {
                        verification,
                        observation,
                        ..
                    } => {
                        // The tool's observation payload (file content,
                        // shell output) is the result the planner sees.
                        let payload = observation.unwrap_or_else(|| {
                            format!("{} {:?}", action.operation, action.arguments)
                        });
                        last_observation
                            .replace(format!("OK (verification {verification:?}): {payload}"));
                    }
                    lumi_orchestrator::StepOutcome::ApprovalNeeded { reason, .. } => {
                        let action_digest = action.material_digest();
                        *pending = Some((tool.name(), Box::new(action.clone())));
                        return AgentRunOutcome::WaitingApproval {
                            action: Box::new(action),
                            action_digest,
                            reason,
                        };
                    }
                    lumi_orchestrator::StepOutcome::Denied { reason } => {
                        *done = true;
                        return AgentRunOutcome::Failed {
                            category: lumi_protocol::FailureCategory::PolicyDenyExpected,
                            reason: format!("policy denied {:?}: {reason}", call.name),
                        };
                    }
                    lumi_orchestrator::StepOutcome::Unverified { detail, .. } => {
                        *done = true;
                        return AgentRunOutcome::Failed {
                            category: lumi_protocol::FailureCategory::Postcondition,
                            reason: format!("step not verified: {detail}"),
                        };
                    }
                    lumi_orchestrator::StepOutcome::Retryable { error } => {
                        last_observation
                            .replace(format!("TOOL_ERROR (retryable): {}", error.message));
                        retry_after_observation = true;
                        break;
                    }
                    lumi_orchestrator::StepOutcome::Ambiguous { .. } => {
                        *done = true;
                        return AgentRunOutcome::Failed {
                            category: lumi_protocol::FailureCategory::AmbiguousState,
                            reason: format!(
                                "step {:?} ended AMBIGUOUS: verify external state before continuing",
                                call.name
                            ),
                        };
                    }
                    lumi_orchestrator::StepOutcome::Stopped { reason } => {
                        *done = true;
                        return AgentRunOutcome::Failed {
                            category: lumi_protocol::FailureCategory::UserCancel,
                            reason,
                        };
                    }
                }
            }
            let _ = retry_after_observation;
        }
    }

    /// Resumes after a human approval: validates/consumes it through the
    /// orchestrator, records the observation, and continues the loop.
    pub fn continue_with_approval(
        &mut self,
        approval_id: &lumi_protocol::ApprovalId,
    ) -> AgentRunOutcome {
        let Some((tool_name, action)) = self.pending.take() else {
            return AgentRunOutcome::Failed {
                category: lumi_protocol::FailureCategory::ApprovalInvalid,
                reason: "no pending action to approve".to_owned(),
            };
        };
        let Some(tool) = self.tools.get(tool_name) else {
            return AgentRunOutcome::Failed {
                category: lumi_protocol::FailureCategory::Crash,
                reason: "pending tool no longer registered".to_owned(),
            };
        };
        let ctx = crate::ToolContext {
            workspace: self.workspace,
            task_id: &self.task_id,
            principal: &self.principal,
        };
        let mut executor = |authorized: &ActionProposal| tool.execute(authorized, &ctx);
        let outcome = self.orchestrator.execute_step(
            &action,
            Some(approval_id),
            &self.budget,
            &mut self.consumed,
            self.env,
            &mut executor,
        );
        match outcome {
            lumi_orchestrator::StepOutcome::VerifiedSuccess { .. } => {
                self.last_observation = Some(format!(
                    "OK (approved): {} {:?}",
                    action.operation, action.arguments
                ));
                self.run()
            }
            lumi_orchestrator::StepOutcome::Ambiguous { .. } => {
                self.done = true;
                AgentRunOutcome::Failed {
                    category: lumi_protocol::FailureCategory::AmbiguousState,
                    reason: "approved step ended ambiguous; verify external state".to_owned(),
                }
            }
            other => {
                self.done = true;
                AgentRunOutcome::Failed {
                    category: lumi_protocol::FailureCategory::Postcondition,
                    reason: format!("approved step did not verify: {other:?}"),
                }
            }
        }
    }

    /// The last observation payload, for UX display.
    #[must_use]
    pub fn last_observation(&self) -> Option<&str> {
        self.last_observation.as_deref()
    }

    /// Mutable access to the orchestrator while the loop is paused — the
    /// host needs the approval ledger (issue a scoped human approval,
    /// query pending records) between `run()` and
    /// [`AgentLoop::continue_with_approval`] without tearing down the
    /// loop's conversation state.
    pub fn orchestrator_mut(&mut self) -> &mut Orchestrator<S> {
        self.orchestrator
    }

    /// Budget consumed so far (recorded onto the durable run).
    #[must_use]
    pub const fn consumed(&self) -> &ConsumedBudget {
        &self.consumed
    }

    /// Conversation turn count so far.
    #[must_use]
    pub const fn turns(&self) -> u32 {
        self.turns
    }
}
