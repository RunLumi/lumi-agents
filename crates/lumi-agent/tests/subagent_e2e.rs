//! Subagent delegation certification (Spec 24 §24.9–24.11): narrowed
//! capabilities, carved budget, honest failures. The gate stays the
//! orchestrator — the child never bypasses policy.

use lumi_agent::{run_subagent, ReadFileTool, ScriptedPlanner, SubagentSpec, WriteFileTool};
use lumi_audit::FixtureEnvironment;
use lumi_models::response::{FinishReason, ModelResponse, ToolCall, Usage};
use lumi_orchestrator::{Orchestrator, OrchestratorConfig};
use lumi_policy::{CapabilityGrant, GrantSource};
use lumi_protocol::{Budget, Capability, ExecutionTier, ResourceType, RunId, TaskId};
use lumi_state::InMemoryStateStore;
use lumi_workspaces::{CleanupPolicy, Workspace};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn depot(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "lumi-subagent-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst),
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct DepotGuard(PathBuf);

impl Drop for DepotGuard {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn tool_call(name: &str, args: serde_json::Value) -> ToolCall {
    ToolCall {
        id: format!("call-{name}"),
        name: name.to_owned(),
        arguments: args,
    }
}

fn calls(calls: Vec<ToolCall>) -> ModelResponse {
    ModelResponse {
        request_id: "sub".to_owned(),
        content: None,
        tool_calls: calls,
        structured: None,
        usage: Usage::default(),
        finish_reason: FinishReason::ToolCall,
        provider_request_id: None,
        latency_ms: 0,
        safety_signal: None,
    }
}

fn answer(text: &str) -> ModelResponse {
    ModelResponse {
        request_id: "sub".to_owned(),
        content: Some(text.to_owned()),
        tool_calls: vec![],
        structured: None,
        usage: Usage::default(),
        finish_reason: FinishReason::Stop,
        provider_request_id: None,
        latency_ms: 0,
        safety_signal: None,
    }
}

fn tool_calls_response(calls: Vec<ToolCall>) -> ModelResponse {
    ModelResponse {
        request_id: "sub".to_owned(),
        content: None,
        tool_calls: calls,
        structured: None,
        usage: Usage::default(),
        finish_reason: FinishReason::ToolCall,
        provider_request_id: None,
        latency_ms: 0,
        safety_signal: None,
    }
}

fn setup(tag: &str) -> (Workspace, Orchestrator<InMemoryStateStore>, PathBuf) {
    let dir = depot(tag);
    let ws = Workspace::create(
        &dir.join("ws"),
        &TaskId::generate(),
        "u-sub",
        CleanupPolicy::Manual,
    )
    .unwrap();
    let tenant = lumi_protocol::TenantId::parse("tenant-sub").unwrap();
    let grant = |id: &str, capability: &'static str| CapabilityGrant {
        grant_id: id.to_owned(),
        tenant_id: tenant.clone(),
        principal_id: None,
        capability: Capability::well_known(capability),
        resource_scope: lumi_policy::ResourceScope::all_of([ResourceType::well_known(
            ResourceType::FILE,
        )]),
        target_prefixes: BTreeSet::new(),
        expires_at: None,
        source: GrantSource::OrganizationPolicy,
    };
    let config = OrchestratorConfig {
        registry: lumi_policy::CapabilityRegistry::default()
            .grant(grant("g-read", lumi_protocol::capabilities::FILES_READ))
            .grant(grant("g-create", lumi_protocol::capabilities::FILES_CREATE)),
        device_state: lumi_policy::DeviceExecutionState::Trusted,
        pre_authorizations: vec![],
        retry: lumi_state::RetryPolicy::default(),
        policy_version: "1.0.0".to_owned(),
        executors: vec![lumi_orchestrator::ExecutorDescriptor::new(
            ExecutionTier::ConnectorApi,
            "sub-tools",
            "1.0.0",
            [
                Capability::well_known(lumi_protocol::capabilities::FILES_READ),
                Capability::well_known(lumi_protocol::capabilities::FILES_CREATE),
            ],
        )],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
    };
    let orchestrator = Orchestrator::new(config, InMemoryStateStore::new());
    (ws, orchestrator, dir)
}

#[test]
fn subagent_completes_within_narrowed_toolset_and_budget() {
    let _guard = DepotGuard(depot("ok"));
    let (ws, mut orchestrator, _) = setup("ok");

    std::fs::write(ws.root().join("input.txt"), "hello sub").unwrap();
    let spec = SubagentSpec {
        goal: "Read input.txt and write a copy".to_owned(),
        allowed_capabilities: BTreeSet::from([
            Capability::well_known(lumi_protocol::capabilities::FILES_READ),
            Capability::well_known(lumi_protocol::capabilities::FILES_CREATE),
        ]),
        budget: Budget {
            max_actions: Some(3),
            ..Budget::default()
        },
        max_turns: 8,
        output_hint: Some("Return the copied content.".to_owned()),
    };

    std::fs::write(ws.root().join("input.txt"), "hello sub").unwrap();
    let planner = ScriptedPlanner::new(
        "",
        vec![
            tool_calls_response(vec![tool_call(
                "read_file",
                serde_json::json!({ "path": "input.txt" }),
            )]),
            tool_calls_response(vec![tool_call(
                "write_file",
                serde_json::json!({ "path": "copy.txt", "content": "hello sub" }),
            )]),
            answer("Copied: hello sub"),
        ],
    );

    let outcome = run_subagent(
        &mut orchestrator,
        &planner,
        vec![Box::new(ReadFileTool), Box::new(WriteFileTool)],
        &spec,
        &ws,
        &FixtureEnvironment::new().with_workspace(ws.root().to_path_buf()),
        &lumi_agent::workflow_principal("tenant-sub", "u-sub"),
        &TaskId::generate(),
        &RunId::generate(),
    );

    assert!(outcome.completed, "{outcome:?}");
    assert_eq!(outcome.answer.as_deref(), Some("Copied: hello sub"));
    assert_eq!(
        std::fs::read_to_string(ws.root().join("copy.txt")).unwrap(),
        "hello sub"
    );

    // §24.13 economics are measured from the loop's consumption
    // ledger: exactly the two executed actions, no vision, no
    // external writes, honest wall-clock duration.
    assert_eq!(outcome.actions, 2, "{outcome:?}");
    assert_eq!(outcome.vision_actions, 0);
    assert_eq!(outcome.external_writes, 0);
    assert_eq!(outcome.model_cost_micro_usd, 0);
    assert_eq!(outcome.turns, 3);
}

#[test]
fn subagent_budget_carve_out_stops_excess_actions() {
    let _guard = DepotGuard(depot("budget"));
    let (ws, mut orchestrator, _) = setup("budget");

    // Parent budget allows 10; the subagent gets a carve-out of 1.
    let spec = SubagentSpec {
        goal: "Write two files".to_owned(),
        allowed_capabilities: BTreeSet::from([Capability::well_known(
            lumi_protocol::capabilities::FILES_CREATE,
        )]),
        budget: Budget {
            max_actions: Some(1),
            ..Budget::default()
        },
        max_turns: 8,
        output_hint: None,
    };

    let planner = ScriptedPlanner::new(
        "",
        vec![
            calls(vec![tool_call(
                "write_file",
                serde_json::json!({"path": "a.txt", "content": "A"}),
            )]),
            calls(vec![tool_call(
                "write_file",
                serde_json::json!({"path": "b.txt", "content": "B"}),
            )]),
            answer("both done"),
        ],
    );

    let principal = lumi_agent::workflow_principal("tenant-sub", "u-sub");
    let outcome = run_subagent(
        &mut orchestrator,
        &planner,
        vec![Box::new(WriteFileTool)],
        &spec,
        &ws,
        &FixtureEnvironment::new().with_workspace(ws.root().to_path_buf()),
        &principal,
        &TaskId::generate(),
        &RunId::generate(),
    );

    assert!(!outcome.completed, "budget carve-out must stop the child");

    // Economics reflect the carve-out: exactly one action landed
    // before the budget stopped the child (§24.13 failure accounting).
    assert_eq!(outcome.actions, 1, "{outcome:?}");
    assert!(outcome.failure.unwrap().contains("budget"));
    assert!(ws.root().join("a.txt").is_file());
    assert!(!ws.root().join("b.txt").exists());
}

#[test]
fn subagent_tool_narrowing_refuses_ungranted_proposals() {
    let _guard = DepotGuard(depot("narrow"));
    let (ws, mut orchestrator, _) = setup("narrow");

    // Registry only grants READ; the child spec allows only read. The
    // scripted planner proposes WRITE — the tool is not registered, so
    // the child fails honestly instead of executing anything.
    let spec = SubagentSpec {
        goal: "Read only".to_owned(),
        allowed_capabilities: BTreeSet::from([Capability::well_known(
            lumi_protocol::capabilities::FILES_READ,
        )]),
        budget: Budget::default(),
        max_turns: 4,
        output_hint: None,
    };

    let planner = ScriptedPlanner::new(
        "",
        vec![calls(vec![tool_call(
            "write_file",
            serde_json::json!({"path": "evil.txt", "content": "x"}),
        )])],
    );

    let outcome = run_subagent(
        &mut orchestrator,
        &planner,
        // Only ReadFileTool is registered — the write tool is absent.
        vec![Box::new(ReadFileTool)],
        &spec,
        &ws,
        &FixtureEnvironment::new().with_workspace(ws.root().to_path_buf()),
        &lumi_agent::workflow_principal("tenant-sub", "u-sub"),
        &TaskId::generate(),
        &RunId::generate(),
    );

    assert!(!outcome.completed, "{outcome:?}");
    assert!(!ws.root().join("evil.txt").exists());
}
