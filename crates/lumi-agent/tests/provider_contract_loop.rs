//! Provider-wire planning loop: the REAL `ModelPlanner` (via the OpenAI
//! adapter and the capability-contract request/response path) drives the
//! full work-mode loop offline. The only fixture is the HTTP layer —
//! recorded provider responses are served by `FixtureTransport`, so the
//! delegation loop is provable end-to-end with no live provider.
//!
//! The scripted-planner tests (`fixture_scenarios.rs`, `work_mode.rs`)
//! cover loop semantics; this file proves the model path itself —
//! request normalization, tool-schema translation, response parsing —
//! is the same code a live provider would hit.

use lumi_agent::{AgentRunOutcome, ModelPlanner, ReadFileTool, WriteFileTool};
use lumi_audit::FixtureEnvironment;
use lumi_models::adapters::OpenAIDriver;
use lumi_models::instance::{AuthHeaders, ProviderInstance};
use lumi_models::request::ToolSpec;
use lumi_models::transport::{FixtureResponse, FixtureTransport};
use lumi_orchestrator::{ExecutorDescriptor, Orchestrator, OrchestratorConfig};
use lumi_policy::{CapabilityGrant, GrantSource};
use lumi_protocol::{Budget, Capability, ExecutionTier, ResourceType, RunId, TaskId};
use lumi_state::InMemoryStateStore;
use lumi_workspaces::{CleanupPolicy, Workspace};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const WIRE_FIXTURE: &str = include_str!("fixtures/provider/openai-write-then-answer.json");

fn depot(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let tid = format!("{:?}", std::thread::current().id());
    let dir = std::env::temp_dir().join(format!(
        "lumi-provider-loop-{tag}-{}-{tid}-{}",
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

#[derive(serde::Deserialize)]
struct WireFixture {
    #[allow(dead_code)]
    scenario: String,
    model: String,
    responses: Vec<WireResponse>,
}

#[derive(serde::Deserialize)]
struct WireResponse {
    status: u16,
    body: serde_json::Value,
}

#[test]
fn model_planner_drives_the_loop_through_the_openai_capability_contract_offline() {
    let guard = DepotGuard(depot("wire"));
    let fixture: WireFixture = serde_json::from_str(WIRE_FIXTURE).unwrap();

    // The fixture transport plays the provider: recorded chat.completion
    // bodies, one per model turn, no network.
    let transport = FixtureTransport::serving(
        fixture
            .responses
            .iter()
            .map(|r| FixtureResponse::Http {
                status: r.status,
                body: r.body.to_string(),
            })
            .collect(),
    );

    let instance = ProviderInstance::new(
        "inst-fixture-openai",
        "openai",
        "fixture-openai",
        "https://fixture.invalid",
        lumi_protocol::SecretRef("fixture-credential".to_owned()),
        "openai",
    );
    let auth: AuthHeaders = vec![("Authorization".to_owned(), "Bearer fixture".to_owned())];

    // The tool spec the planner offers the model — the same JSON schema
    // the real system prompt promises.
    let tools_spec = vec![ToolSpec {
        name: "write_file".to_owned(),
        description: "Create a new file inside the task workspace with the given text content."
            .to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "content": {"type": "string"}
            },
            "required": ["path", "content"]
        }),
    }];

    let workspace = Workspace::create(
        &guard.0.join("ws"),
        &TaskId::generate(),
        "u-owner",
        CleanupPolicy::Manual,
    )
    .unwrap();

    let tenant = lumi_protocol::TenantId::parse("tenant-wire").unwrap();
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
        executors: vec![ExecutorDescriptor::new(
            ExecutionTier::ConnectorApi,
            "workspace-tools",
            "1.0.0",
            [
                Capability::well_known(lumi_protocol::capabilities::FILES_READ),
                Capability::well_known(lumi_protocol::capabilities::FILES_CREATE),
            ],
        )],
        tier_policy: lumi_orchestrator::TierPolicy::default(),
    };
    let mut orchestrator = Orchestrator::new(config, InMemoryStateStore::new());

    let planner = ModelPlanner {
        driver: &OpenAIDriver::new(),
        instance: &instance,
        auth: &auth,
        transport: &transport,
        model: fixture.model.clone(),
        tools: tools_spec,
        system_prompt: "You are Lumi, executing a task on the user's machine.".to_owned(),
        timeout_ms: 5_000,
    };

    let env = FixtureEnvironment::new().with_workspace(workspace.root().to_path_buf());
    let mut loop_agent = lumi_agent::AgentLoop::new(
        &mut orchestrator,
        &planner,
        vec![Box::new(WriteFileTool), Box::new(ReadFileTool)],
        &workspace,
        &env,
        lumi_agent::workflow_principal("tenant-wire", "u-wire"),
        TaskId::generate(),
        RunId::generate(),
        Budget::default(),
        "Write out/provider-note.txt",
    );
    let outcome = loop_agent.run();

    // The full loop completed through the real adapter parse path.
    match outcome {
        AgentRunOutcome::Completed { answer, turns } => {
            assert!(
                answer.contains("provider-wire"),
                "final answer came from the fixture provider response: {answer}"
            );
            assert_eq!(turns, 2, "one tool-call turn, then the answer turn");
        }
        other => panic!("expected Completed, got {other:?}"),
    }

    // The file was really written with the bytes the provider proposed.
    let note = workspace.root().join("out/provider-note.txt");
    assert_eq!(
        std::fs::read_to_string(note).unwrap(),
        "written through the capability contract"
    );

    // The wire requests prove the capability-contract path: correct
    // endpoint, auth headers, tool schema, and conversation shape.
    let requests = transport.requests();
    assert_eq!(requests.len(), 2, "one provider call per model turn");
    assert_eq!(
        requests[0].url,
        "https://fixture.invalid/v1/chat/completions"
    );
    let headers = &requests[0].headers;
    assert!(
        headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer fixture"),
        "credential headers resolved at the boundary"
    );
    let first_body: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(first_body["model"], "gpt-4o-mini");
    assert!(
        first_body["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["function"]["name"] == "write_file"),
        "tool schema translated to the provider wire"
    );
    assert!(
        first_body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["role"] == "system"),
        "trusted system prompt present"
    );

    // The second request carries the tool result observation back to the
    // provider — the observe half of the loop over the real wire format.
    let second_body: serde_json::Value = serde_json::from_str(&requests[1].body).unwrap();
    let roles: Vec<&str> = second_body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["role"].as_str())
        .collect();
    assert!(
        roles.contains(&"tool"),
        "tool result fed back as observation: {roles:?}"
    );
}
