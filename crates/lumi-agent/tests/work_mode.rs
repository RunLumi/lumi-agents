//! Work-mode runner certification: plan → propose → gate → execute →
//! observe, with scripted planners (hermetic, no network).

use lumi_agent::{
    AgentLoop, AgentRunOutcome, ReadFileTool, RunShellTool, ScriptedPlanner, WriteFileTool,
};
use lumi_audit::FixtureEnvironment;
use lumi_models::request::ModelMessage;
use lumi_models::response::ToolCall;
use lumi_models::response::{FinishReason, ModelResponse, Usage};
use lumi_models::ModelError;
use lumi_orchestrator::{Orchestrator, OrchestratorConfig};
use lumi_policy::{CapabilityGrant, GrantSource, ResourceScope};
use lumi_protocol::{Budget, Capability, ExecutionTier, ResourceType};

use lumi_state::InMemoryStateStore;
use lumi_workspaces::{CleanupPolicy, Workspace};
use std::collections::BTreeSet;

fn planner_principal() -> lumi_protocol::Principal {
    lumi_agent::tools::workflow_principal("t-acme", "u-workflow")
}

fn setup(goal: &str, turns: Vec<ModelResponse>) -> (PathBufGuard, AgentRunTestSetup) {
    static DEPOT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let tid = format!("{:?}", std::thread::current().id());
    let depot = std::env::temp_dir().join(format!(
        "lumi-agent-{}-{tid}-{}",
        std::process::id(),
        DEPOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
    ));
    std::fs::create_dir_all(&depot).unwrap();
    let ws = Workspace::create(
        &depot,
        &lumi_protocol::TaskId::parse("task-agent").unwrap(),
        "u-owner",
        CleanupPolicy::Manual,
    )
    .unwrap();
    let registry = lumi_policy::CapabilityRegistry::default()
        .grant(CapabilityGrant {
            grant_id: "g-files".to_owned(),
            tenant_id: planner_principal().tenant_id,
            principal_id: None,
            capability: Capability::well_known(lumi_protocol::capabilities::FILES_CREATE),
            resource_scope: ResourceScope::all_of([ResourceType::well_known(ResourceType::FILE)]),
            target_prefixes: BTreeSet::new(),
            expires_at: None,
            source: GrantSource::OrganizationPolicy,
        })
        .grant(CapabilityGrant {
            grant_id: "g-files-read".to_owned(),
            tenant_id: planner_principal().tenant_id,
            principal_id: None,
            capability: Capability::well_known(lumi_protocol::capabilities::FILES_READ),
            resource_scope: ResourceScope::all_of([ResourceType::well_known(ResourceType::FILE)]),
            target_prefixes: BTreeSet::new(),
            expires_at: None,
            source: GrantSource::OrganizationPolicy,
        })
        .grant(CapabilityGrant {
            grant_id: "g-shell".to_owned(),
            tenant_id: planner_principal().tenant_id,
            principal_id: None,
            capability: Capability::well_known(lumi_protocol::capabilities::SHELL_EXECUTE),
            resource_scope: ResourceScope::all_of([ResourceType::well_known(ResourceType::FILE)]),
            target_prefixes: BTreeSet::new(),
            expires_at: None,
            source: GrantSource::OrganizationPolicy,
        });
    let config = OrchestratorConfig {
        registry,
        device_state: lumi_policy::DeviceExecutionState::Trusted,
        pre_authorizations: vec![],
        retry: lumi_state::RetryPolicy::default(),
        policy_version: "1.0.0".to_owned(),
        executors: vec![lumi_orchestrator::ExecutorDescriptor::new(
            ExecutionTier::ConnectorApi,
            "workspace-tools",
            "1.0.0",
            [
                Capability::well_known(lumi_protocol::capabilities::FILES_CREATE),
                Capability::well_known(lumi_protocol::capabilities::FILES_READ),
                Capability::well_known(lumi_protocol::capabilities::SHELL_EXECUTE),
            ],
        )],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
    };
    let store = InMemoryStateStore::new();
    let orchestrator = Orchestrator::new(config, store);
    let planner = ScriptedPlanner::new("", turns);
    let _ = goal;
    let env = FixtureEnvironment::new();
    (
        PathBufGuard(depot),
        AgentRunTestSetup {
            orchestrator,
            planner,
            workspace: ws,
            env,
        },
    )
}

struct PathBufGuard(PathBuf);

impl Drop for PathBufGuard {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

use std::path::PathBuf;

struct AgentRunTestSetup {
    orchestrator: Orchestrator<InMemoryStateStore>,
    planner: ScriptedPlanner,
    workspace: Workspace,
    env: FixtureEnvironment,
}

fn tool_call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall {
        id: format!("call-{name}"),
        name: name.to_owned(),
        arguments,
    }
}

fn response_with_calls(calls: Vec<ToolCall>) -> ModelResponse {
    ModelResponse {
        request_id: "req".to_owned(),
        content: None,
        tool_calls: calls,
        structured: None,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        finish_reason: FinishReason::ToolCall,
        provider_request_id: None,
        latency_ms: 0,
        safety_signal: None,
    }
}

fn response_answer(text: &str) -> ModelResponse {
    ModelResponse {
        request_id: "req".to_owned(),
        content: Some(text.to_owned()),
        tool_calls: vec![],
        structured: None,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        finish_reason: FinishReason::Stop,
        provider_request_id: None,
        latency_ms: 0,
        safety_signal: None,
    }
}

#[test]
fn work_mode_completes_write_then_answer() {
    let (_guard, setup) = setup(
        "Save a note",
        vec![
            response_with_calls(vec![tool_call(
                "write_file",
                serde_json::json!({"path": "notes/todo.txt", "content": "reconcile Q3"}),
            )]),
            response_answer("Saved the note."),
        ],
    );
    let mut orchestrator = setup.orchestrator;
    let env = setup.env;
    let planner = setup.planner;
    let ws = setup.workspace;
    {
        let mut agent = AgentLoop::new(
            &mut orchestrator,
            &planner,
            vec![Box::new(WriteFileTool), Box::new(ReadFileTool)],
            &ws,
            &env,
            planner_principal(),
            lumi_protocol::TaskId::parse("task-agent").unwrap(),
            lumi_protocol::RunId::parse("run-agent").unwrap(),
            Budget::default(),
            "Save a note",
            None,
        );
        let outcome = agent.run();
        match outcome {
            AgentRunOutcome::Completed { answer, turns } => {
                assert_eq!(answer, "Saved the note.");
                assert_eq!(turns, 2);
            }
            other => panic!("expected Completed, got {other:?}"),
        }
    }
    // The file genuinely exists in the workspace.
    assert_eq!(
        std::fs::read_to_string(ws.root().join("notes/todo.txt")).unwrap(),
        "reconcile Q3"
    );
    // Audit chain intact.
    assert!(matches!(
        orchestrator.audit.verify_chain(),
        lumi_audit::ChainVerification::Intact { .. }
    ));
}

#[test]
fn read_file_observation_feeds_back_to_planner() {
    let (depot, ws) = {
        static DEPOT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let tid = format!("{:?}", std::thread::current().id());
        let depot = std::env::temp_dir().join(format!(
            "lumi-agent-read-{}-{tid}-{}",
            std::process::id(),
            DEPOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        ));
        std::fs::create_dir_all(&depot).unwrap();
        let ws = Workspace::create(
            &depot,
            &lumi_protocol::TaskId::parse("task-agent-read").unwrap(),
            "u-owner",
            CleanupPolicy::Manual,
        )
        .unwrap();
        std::fs::write(ws.root().join("input.txt"), "invoice total: 4200").unwrap();
        (depot, ws)
    };
    // The planner asserts it RECEIVED the file content as a ToolResult.
    let received = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    struct AssertingPlanner {
        received: std::sync::Arc<std::sync::Mutex<String>>,
        turns: std::sync::Mutex<std::collections::VecDeque<ModelResponse>>,
    }
    impl lumi_agent::Planner for AssertingPlanner {
        fn plan(&self, messages: &[ModelMessage]) -> Result<ModelResponse, ModelError> {
            // Record the last tool result.
            for message in messages {
                if let ModelMessage::ToolResult { content, .. } = message {
                    *self.received.lock().unwrap() = content.clone();
                }
            }
            let mut turns = self.turns.lock().unwrap();
            if turns.len() == 1 {
                // Second turn: verify we saw the content, then answer.
                assert!(
                    self.received.lock().unwrap().contains("4200"),
                    "planner must observe the file content, saw {:?}",
                    self.received.lock().unwrap()
                );
            }
            turns
                .pop_front()
                .ok_or_else(|| lumi_models::ModelError::format("exhausted"))
        }
        fn tool_specs(&self) -> Vec<lumi_models::request::ToolSpec> {
            vec![]
        }
        fn system_prompt(&self) -> String {
            String::new()
        }
    }
    let planner = AssertingPlanner {
        received: std::sync::Arc::clone(&received),
        turns: std::sync::Mutex::new(
            vec![
                response_with_calls(vec![tool_call(
                    "read_file",
                    serde_json::json!({"path": "input.txt"}),
                )]),
                response_answer("done"),
            ]
            .into_iter()
            .collect(),
        ),
    };
    let registry = lumi_policy::CapabilityRegistry::default().grant(CapabilityGrant {
        grant_id: "g-read".to_owned(),
        tenant_id: planner_principal().tenant_id,
        principal_id: None,
        capability: Capability::well_known(lumi_protocol::capabilities::FILES_READ),
        resource_scope: ResourceScope::all_of([ResourceType::well_known(ResourceType::FILE)]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: GrantSource::OrganizationPolicy,
    });
    let config = OrchestratorConfig {
        registry,
        device_state: lumi_policy::DeviceExecutionState::Trusted,
        pre_authorizations: vec![],
        retry: lumi_state::RetryPolicy::default(),
        policy_version: "1.0.0".to_owned(),
        executors: vec![lumi_orchestrator::ExecutorDescriptor::new(
            ExecutionTier::ConnectorApi,
            "workspace-tools",
            "1.0.0",
            [Capability::well_known(
                lumi_protocol::capabilities::FILES_READ,
            )],
        )],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
    };
    let store = InMemoryStateStore::new();
    let mut orchestrator = Orchestrator::new(config, store);
    let env = FixtureEnvironment::new().with_record(
        lumi_protocol::ResourceRef {
            resource_type: ResourceType::well_known(ResourceType::FILE),
            id: "workspaces/task-agent-read".to_owned(),
            sensitivity: None,
        },
        serde_json::json!({"exists": true}),
    );
    let mut agent = AgentLoop::new(
        &mut orchestrator,
        &planner,
        vec![Box::new(ReadFileTool)],
        &ws,
        &env,
        planner_principal(),
        lumi_protocol::TaskId::parse("task-agent-read").unwrap(),
        lumi_protocol::RunId::parse("run-agent-read").unwrap(),
        Budget::default(),
        "Read the input",
        None,
    );
    let outcome = agent.run();
    assert!(matches!(outcome, AgentRunOutcome::Completed { .. }));
    assert!(received.lock().unwrap().contains("4200"));
    std::fs::remove_dir_all(&depot).ok();
}

#[test]
fn shell_tool_runs_inside_workspace() {
    let (_guard, setup) = setup(
        "list files",
        vec![
            response_with_calls(vec![tool_call(
                "run_shell",
                serde_json::json!({"command": "node", "args": ["-e", "console.log('listed')"]}),
            )]),
            response_answer("listed."),
        ],
    );
    let mut orchestrator = setup.orchestrator;
    let env = setup.env;
    let planner = setup.planner;
    let ws = setup.workspace;
    let mut agent = AgentLoop::new(
        &mut orchestrator,
        &planner,
        vec![Box::new(RunShellTool)],
        &ws,
        &env,
        planner_principal(),
        lumi_protocol::TaskId::parse("task-agent").unwrap(),
        lumi_protocol::RunId::parse("run-agent").unwrap(),
        Budget::default(),
        "list files",
        None,
    );
    let outcome = agent.run();
    assert!(
        matches!(outcome, AgentRunOutcome::Completed { .. }),
        "{outcome:?}"
    );
    // The shell output reached the planner as a tool-result observation.
    let received = planner.received();
    let second_turn = received.last().unwrap();
    let saw_output = second_turn.iter().any(|m| match m {
        ModelMessage::ToolResult { content, .. } => content.contains("listed"),
        _ => false,
    });
    assert!(saw_output, "planner must observe the shell output");
}

#[test]
fn unknown_tool_from_planner_fails_the_run() {
    let (_guard, setup) = setup(
        "x",
        vec![response_with_calls(vec![tool_call(
            "delete_everything",
            serde_json::json!({}),
        )])],
    );
    let mut orchestrator = setup.orchestrator;
    let env = setup.env;
    let planner = setup.planner;
    let ws = setup.workspace;
    let mut agent = AgentLoop::new(
        &mut orchestrator,
        &planner,
        vec![Box::new(WriteFileTool)],
        &ws,
        &env,
        planner_principal(),
        lumi_protocol::TaskId::parse("task-agent").unwrap(),
        lumi_protocol::RunId::parse("run-agent").unwrap(),
        Budget::default(),
        "x",
        None,
    );
    let outcome = agent.run();
    match outcome {
        AgentRunOutcome::Failed { reason, .. } => {
            assert!(reason.contains("unknown tool"), "{reason}");
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn max_turns_bound_stops_runaway_loops() {
    // A planner that always proposes the same write (never converges).
    let endless = std::iter::repeat_with(|| {
        response_with_calls(vec![tool_call(
            "write_file",
            serde_json::json!({"path": "out/loop.txt", "content": "again"}),
        )])
    })
    .take(20)
    .collect();
    let (_guard, setup) = setup("loop", endless);
    let mut orchestrator = setup.orchestrator;
    let env = setup.env;
    let planner = setup.planner;
    let ws = setup.workspace;
    let mut agent = AgentLoop::new(
        &mut orchestrator,
        &planner,
        vec![Box::new(WriteFileTool)],
        &ws,
        &env,
        planner_principal(),
        lumi_protocol::TaskId::parse("task-agent").unwrap(),
        lumi_protocol::RunId::parse("run-agent").unwrap(),
        Budget::default(),
        "loop",
        None,
    );
    // Each write needs a distinct path or it collides with overwrite
    // refusal; either way the loop must stop at max_turns.
    let outcome = agent.run();
    match outcome {
        AgentRunOutcome::Failed { reason, .. } => {
            assert!(
                reason.contains("budget")
                    || reason.contains("overwrite not authorized")
                    || reason.contains("terminal"),
                "{reason}"
            );
        }
        AgentRunOutcome::Completed { .. } => panic!("runaway loop must not complete"),
        other => panic!("unexpected {other:?}"),
    }
    assert!(agent.turns() <= 17, "turn bound respected");
}
