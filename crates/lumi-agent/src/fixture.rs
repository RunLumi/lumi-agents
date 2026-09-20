//! Fixture-file-driven planning scenarios (offline, no live provider).
//!
//! A planning fixture is a JSON file describing the model turns of one
//! scenario — tool calls and/or a final answer. [`FixturePlanner`]
//! replays them deterministically through the same [`Planner`] contract
//! the real provider planner implements, so the full
//! plan → propose → gate → execute → observe loop is exercisable from
//! data: unit tests, the dogfood harness, and eval runs all consume the
//! same scenario files. Live-provider behavior is exercised separately
//! through the adapter contract suite and the provider-wire fixture test
//! (`tests/provider_contract_loop.rs`).

use crate::planner::Planner;
use lumi_models::error::ModelError;
use lumi_models::request::{ModelMessage, ToolSpec};
use lumi_models::response::{FinishReason, ModelResponse, ToolCall, Usage};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// One planning scenario: the ordered model turns Lumi should observe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningFixture {
    /// Stable scenario id, e.g. `fix-readme-typo`.
    pub scenario: String,
    /// What the scenario represents (evidence/report material).
    #[serde(default)]
    pub description: String,
    /// The user goal the host should submit with this scenario.
    pub goal: String,
    /// Ordered model turns. The last turn is usually a final answer.
    pub turns: Vec<FixtureTurn>,
    /// Workspace-relative paths the scenario claims to produce. The
    /// dogfood harness checks these INDEPENDENTLY of whatever the loop
    /// reported — a fixture (or a live model) claiming work it did not
    /// do fails the evaluation.
    #[serde(default)]
    pub expected_files: Vec<String>,
}

/// One replayed model turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FixtureTurn {
    /// The model proposes tool calls (normalized proposals are still
    /// built by the tools themselves — fixtures carry arguments only).
    ToolCalls { calls: Vec<FixtureCall> },
    /// The model produces a final answer (no tool calls).
    Answer { content: String },
}

/// One proposed tool call inside a [`FixtureTurn::ToolCalls`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Parses a fixture from JSON text.
///
/// # Errors
/// [`ModelError::format`] when the JSON is malformed or a turn is
/// structurally invalid.
pub fn parse_fixture(json: &str) -> Result<PlanningFixture, ModelError> {
    serde_json::from_str(json)
        .map_err(|e| ModelError::format(format!("invalid planning fixture: {e}")))
}

/// Loads a fixture from a file.
///
/// # Errors
/// IO failure or [`parse_fixture`] errors.
pub fn load_fixture(path: &Path) -> Result<PlanningFixture, ModelError> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| ModelError::format(format!("read planning fixture: {e}")))?;
    parse_fixture(&json)
}

/// A deterministic planner that replays one [`PlanningFixture`].
///
/// Like [`crate::ScriptedPlanner`], it records every conversation it is
/// shown so tests can assert what the model observed.
#[derive(Debug)]
pub struct FixturePlanner {
    fixture: PlanningFixture,
    system_prompt: String,
    tools: Vec<ToolSpec>,
    next: Mutex<usize>,
    received: Mutex<Vec<Vec<ModelMessage>>>,
}

impl FixturePlanner {
    /// Replays `fixture` with an empty system prompt.
    #[must_use]
    pub fn new(fixture: PlanningFixture) -> Self {
        Self::with_system(fixture, String::new())
    }

    /// Replays `fixture` with the given trusted system prompt.
    #[must_use]
    pub fn with_system(fixture: PlanningFixture, system_prompt: String) -> Self {
        Self {
            fixture,
            system_prompt,
            tools: Vec::new(),
            next: Mutex::new(0),
            received: Mutex::new(Vec::new()),
        }
    }

    /// The goal declared by the fixture (the host submits it as the task).
    #[must_use]
    pub fn goal(&self) -> &str {
        &self.fixture.goal
    }

    /// Scenario id of the loaded fixture.
    #[must_use]
    pub fn scenario(&self) -> &str {
        &self.fixture.scenario
    }

    /// Conversation snapshots recorded at each `plan()` call.
    #[must_use]
    pub fn received(&self) -> Vec<Vec<ModelMessage>> {
        self.received.lock().map(|r| r.clone()).unwrap_or_default()
    }

    fn turn_response(turn: &FixtureTurn, index: usize) -> ModelResponse {
        let base = ModelResponse {
            request_id: format!("fixture-{index}"),
            content: None,
            tool_calls: Vec::new(),
            structured: None,
            usage: Usage {
                input_tokens: 0,
                output_tokens: 0,
            },
            finish_reason: FinishReason::Stop,
            provider_request_id: None,
            latency_ms: 0,
            safety_signal: None,
        };
        match turn {
            FixtureTurn::ToolCalls { calls } => ModelResponse {
                tool_calls: calls
                    .iter()
                    .enumerate()
                    .map(|(i, call)| ToolCall {
                        id: format!("fixture-call-{index}-{i}"),
                        name: call.name.clone(),
                        arguments: call.arguments.clone(),
                    })
                    .collect(),
                finish_reason: FinishReason::ToolCall,
                ..base
            },
            FixtureTurn::Answer { content } => ModelResponse {
                content: Some(content.clone()),
                ..base
            },
        }
    }
}

impl Planner for FixturePlanner {
    fn plan(&self, messages: &[ModelMessage]) -> Result<ModelResponse, ModelError> {
        if let Ok(mut received) = self.received.lock() {
            received.push(messages.to_vec());
        }
        let mut next = self
            .next
            .lock()
            .map_err(|_| ModelError::format("poisoned fixture planner"))?;
        let index = *next;
        let Some(turn) = self.fixture.turns.get(index) else {
            return Err(ModelError::new(
                lumi_protocol::FailureCategory::ModelReasoning,
                format!(
                    "fixture {} exhausted after {} turns",
                    self.fixture.scenario, index
                ),
                false,
            ));
        };
        *next = index + 1;
        Ok(Self::turn_response(turn, index))
    }

    fn tool_specs(&self) -> Vec<ToolSpec> {
        self.tools.clone()
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn provider_family(&self) -> &str {
        "fixture"
    }
}
