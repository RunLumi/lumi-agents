//! Work-mode agent runner (roadmap Work mode, spec 12 §12.11).
//!
//! A bounded loop: the planner (a model via `lumi-models`) proposes tool
//! calls or a final answer; each proposed tool call is turned into a
//! normalized [`ActionProposal`] by the tool itself (models never compose
//! raw proposals) and pushed through the orchestrator gate
//! (policy → approval → journal → verification → audit). Tool results —
//! including failures and verifier outcomes — are fed back as
//! observations, which are DATA: the system prompt marks them untrusted.
//!
//! Approval-gated steps pause the loop (`WaitingApproval`); the host app
//! obtains a human approval and resumes. Repeated Work-mode successes are
//! pack candidates (§12.11).

pub mod loop_impl;
pub mod planner;
pub mod tools;

pub use loop_impl::{AgentLoop, AgentRunOutcome};
pub use planner::{ModelPlanner, Planner, ScriptedPlanner};
pub use tools::ToolContext;
pub use tools::{workflow_principal, ReadFileTool, RunShellTool, ToolError, WriteFileTool};
