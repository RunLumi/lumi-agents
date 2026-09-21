//! The agent loop: plan -> propose -> gate -> execute -> observe.
//!
//! The host owns system instructions, authority and verification. Model calls
//! and their bounded observations stay paired by their original provider IDs.

use crate::planner::Planner;
use crate::tools::AgentTool;
use lumi_models::request::ModelMessage;
use lumi_models::response::FinishReason;
use lumi_orchestrator::{Orchestrator, StepOutcome};
use lumi_protocol::{
    ActionProposal, Budget, ConsumedBudget, FailureCategory, RunId, TaskId, Timestamp,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum AgentRunOutcome {
    Completed {
        answer: String,
        turns: u32,
    },
    WaitingApproval {
        action: Box<ActionProposal>,
        action_digest: String,
        reason: String,
    },
    Failed {
        category: FailureCategory,
        reason: String,
    },
}

/// One host-bound work loop. The conversation remains alive across a human
/// approval; resuming does not ask the model to reconstruct the approved action.
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
    last_observation: Option<String>,
    pending: Option<(&'static str, String, Box<ActionProposal>)>,
    done: bool,
}

impl<'a, S: lumi_state::StateStore> AgentLoop<'a, S> {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
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
        let system = if planner.system_prompt().trim().is_empty() {
            crate::planner::system_prompt(goal, &workspace.root().display().to_string(), &names)
        } else {
            planner.system_prompt().to_owned()
        };
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

    /// Runs until a final answer, approval, refusal or a bounded failure.
    #[allow(clippy::too_many_lines)]
    pub fn run(&mut self) -> AgentRunOutcome {
        if self.done {
            return AgentRunOutcome::Failed {
                category: FailureCategory::Crash,
                reason: "loop already finished".into(),
            };
        }
        if let Some((_, _, action)) = &self.pending {
            return AgentRunOutcome::WaitingApproval {
                action: action.clone(),
                action_digest: action.material_digest(),
                reason: "the exact pending action still requires a human decision".into(),
            };
        }
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
            if orchestrator.cancel.is_cancelled() {
                *done = true;
                return AgentRunOutcome::Failed {
                    category: FailureCategory::UserCancel,
                    reason: "task stopped before planning".into(),
                };
            }
            if *turns >= *max_turns {
                *done = true;
                return AgentRunOutcome::Failed {
                    category: FailureCategory::BudgetExceeded,
                    reason: format!("turn budget exhausted ({max_turns})"),
                };
            }
            if let Some(dimension) = budget.check(consumed, Timestamp::now()) {
                *done = true;
                return AgentRunOutcome::Failed {
                    category: FailureCategory::BudgetExceeded,
                    reason: format!("task budget exhausted: {dimension:?}"),
                };
            }
            *turns += 1;
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
            if orchestrator.cancel.is_cancelled() {
                *done = true;
                return AgentRunOutcome::Failed {
                    category: FailureCategory::UserCancel,
                    reason: "task stopped during planning".into(),
                };
            }
            if let Some(dimension) = budget.check(consumed, Timestamp::now()) {
                *done = true;
                return AgentRunOutcome::Failed {
                    category: FailureCategory::BudgetExceeded,
                    reason: format!("task budget exhausted: {dimension:?}"),
                };
            }
            if matches!(
                response.finish_reason,
                FinishReason::Refusal | FinishReason::Error | FinishReason::Length
            ) {
                *done = true;
                return AgentRunOutcome::Failed {
                    category: FailureCategory::ModelFormat,
                    reason: format!(
                        "model response is not complete: {:?}",
                        response.finish_reason
                    ),
                };
            }
            if response.tool_calls.is_empty() {
                *done = true;
                let answer = response.content.unwrap_or_default();
                if response.finish_reason != FinishReason::Stop || answer.trim().is_empty() {
                    return AgentRunOutcome::Failed {
                        category: FailureCategory::ModelFormat,
                        reason: "model returned neither a complete answer nor a tool call".into(),
                    };
                }
                messages.push(ModelMessage::Assistant {
                    content: answer.clone(),
                });
                return AgentRunOutcome::Completed {
                    answer,
                    turns: *turns,
                };
            }
            // Each executed call is represented immediately with its own result.
            // On an approval pause, later unexecuted proposals are not represented
            // as delivered calls; the model can plan them against refreshed state.
            for call in &response.tool_calls {
                if call.id.trim().is_empty() {
                    *done = true;
                    return AgentRunOutcome::Failed {
                        category: FailureCategory::ModelFormat,
                        reason: "tool call is missing its correlation ID".into(),
                    };
                }
                let Some(tool) = tools.get(call.name.as_str()) else {
                    *done = true;
                    return AgentRunOutcome::Failed {
                        category: FailureCategory::ModelFormat,
                        reason: format!("planner proposed unknown tool {:?}", call.name),
                    };
                };
                messages.push(ModelMessage::AssistantToolCall { call: call.clone() });
                let ctx = crate::ToolContext {
                    workspace,
                    task_id,
                    principal,
                };
                let action = match tool.build_proposal(call.arguments.clone(), &ctx, run_id) {
                    Ok(action) => action,
                    Err(e) => {
                        let content =
                            format!("TOOL_ERROR: invalid arguments for {}: {e}", call.name);
                        *last_observation = Some(content.clone());
                        messages.push(ModelMessage::ToolResult {
                            tool_call_id: call.id.clone(),
                            content,
                        });
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
                    StepOutcome::VerifiedSuccess {
                        verification,
                        observation,
                        ..
                    } => {
                        let payload = observation
                            .unwrap_or_else(|| format!("{} completed", action.operation));
                        let content = format!("OK (verification {verification:?}): {payload}");
                        *last_observation = Some(content.clone());
                        messages.push(ModelMessage::ToolResult {
                            tool_call_id: call.id.clone(),
                            content,
                        });
                    }
                    StepOutcome::ApprovalNeeded { reason, .. } => {
                        let action_digest = action.material_digest();
                        *pending = Some((tool.name(), call.id.clone(), Box::new(action.clone())));
                        return AgentRunOutcome::WaitingApproval {
                            action: Box::new(action),
                            action_digest,
                            reason,
                        };
                    }
                    StepOutcome::Denied { reason } => {
                        *done = true;
                        return AgentRunOutcome::Failed {
                            category: FailureCategory::PolicyDenyExpected,
                            reason: format!("policy denied {:?}: {reason}", call.name),
                        };
                    }
                    StepOutcome::Unverified { detail, .. } => {
                        *done = true;
                        return AgentRunOutcome::Failed {
                            category: FailureCategory::Postcondition,
                            reason: format!("step not verified: {detail}"),
                        };
                    }
                    StepOutcome::Retryable { error } => {
                        let content = format!("TOOL_ERROR (retryable): {}", error.message);
                        *last_observation = Some(content.clone());
                        messages.push(ModelMessage::ToolResult {
                            tool_call_id: call.id.clone(),
                            content,
                        });
                        break;
                    }
                    StepOutcome::Ambiguous { .. } => {
                        *done = true;
                        return AgentRunOutcome::Failed { category: FailureCategory::AmbiguousState, reason: format!("step {:?} ended AMBIGUOUS: verify external state before continuing", call.name) };
                    }
                    StepOutcome::Stopped { reason } => {
                        *done = true;
                        return AgentRunOutcome::Failed {
                            category: FailureCategory::UserCancel,
                            reason,
                        };
                    }
                }
            }
        }
    }

    pub fn continue_with_approval(
        &mut self,
        approval_id: &lumi_protocol::ApprovalId,
    ) -> AgentRunOutcome {
        let Some((tool_name, call_id, action)) = self.pending.take() else {
            return AgentRunOutcome::Failed {
                category: FailureCategory::ApprovalInvalid,
                reason: "no pending action to approve".into(),
            };
        };
        let Some(tool) = self.tools.get(tool_name) else {
            self.done = true;
            return AgentRunOutcome::Failed {
                category: FailureCategory::Crash,
                reason: "pending tool no longer registered".into(),
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
            StepOutcome::VerifiedSuccess {
                verification,
                observation,
                ..
            } => {
                let payload =
                    observation.unwrap_or_else(|| format!("{} completed", action.operation));
                let content = format!("OK (approved; verification {verification:?}): {payload}");
                self.last_observation = Some(content.clone());
                self.messages.push(ModelMessage::ToolResult {
                    tool_call_id: call_id,
                    content,
                });
                self.run()
            }
            StepOutcome::Ambiguous { .. } => {
                self.done = true;
                AgentRunOutcome::Failed {
                    category: FailureCategory::AmbiguousState,
                    reason: "approved step ended ambiguous; verify external state".into(),
                }
            }
            StepOutcome::Stopped { reason } => {
                self.done = true;
                AgentRunOutcome::Failed {
                    category: FailureCategory::UserCancel,
                    reason,
                }
            }
            other => {
                self.done = true;
                AgentRunOutcome::Failed {
                    category: FailureCategory::Postcondition,
                    reason: format!("approved step did not verify: {other:?}"),
                }
            }
        }
    }

    #[must_use]
    pub fn last_observation(&self) -> Option<&str> {
        self.last_observation.as_deref()
    }

    pub fn orchestrator_mut(&mut self) -> &mut Orchestrator<S> {
        self.orchestrator
    }

    #[must_use]
    pub const fn consumed(&self) -> &ConsumedBudget {
        &self.consumed
    }

    #[must_use]
    pub const fn turns(&self) -> u32 {
        self.turns
    }
}
