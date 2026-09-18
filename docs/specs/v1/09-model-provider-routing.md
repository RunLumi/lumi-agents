# 09 — Model Provider & Routing v1

Status: Normative  
Supersedes: model-capabilities-v0.md

## 9.1 Goal

Make model providers replaceable while routing based on capability, privacy, reliability, latency, and cost.

## 9.2 Capability contract

Provider adapter MUST declare support for:

- text;
- reasoning;
- vision;
- tool_use;
- structured_output;
- embeddings;
- native_computer_use;
- streaming;
- long_context;
- local_execution;
- data_regions;
- max_context_tokens;
- parallel_tool_calls;
- cancellation;
- usage_reporting.

Unsupported capability MUST be explicit.

## 9.3 Provider adapter families

V1 target adapters:

Tier 1:
- OpenAI
- Anthropic
- Gemini

Tier 2:
- Azure OpenAI
- AWS Bedrock
- OpenRouter
- xAI/Grok when validated
- generic OpenAI-compatible

Local:
- Ollama
- vLLM / compatible enterprise endpoints

## 9.4 Provider abstraction rule

Business/workflow logic MUST NOT branch on provider name.

Provider-specific code belongs behind adapter interfaces.

## 9.5 Model request

Normalized request SHOULD include:

- role/task type;
- messages/context refs;
- tool schema;
- structured output schema;
- media refs;
- required capabilities;
- latency class;
- budget;
- privacy/data-egress constraints;
- cancellation token.

## 9.6 Model response

Normalize:

- text/content;
- tool calls;
- structured output;
- usage;
- finish reason;
- safety/provider signals;
- latency;
- provider request ID;
- retryability;
- error category.

## 9.7 Routing order

Router MUST enforce:

1. tenant data policy;
2. local/cloud constraint;
3. provider allowlist;
4. region/data residency;
5. capability match;
6. workflow/provider compatibility;
7. measured reliability;
8. latency budget;
9. cost per verified success;
10. remaining task budget.

Privacy constraints outrank cost.

## 9.8 Fallback

Fallback MUST NOT:

- violate local-only requirement;
- change allowed data region;
- bypass provider allowlist;
- remove required capability.

Fallback SHOULD be recorded in audit/telemetry.

## 9.9 Model roles

Router MAY distinguish roles:

- planner;
- extractor;
- classifier;
- verifier;
- code model;
- vision grounder;
- summarizer;
- embedding model.

Using different models per role is allowed.

## 9.10 Reliability registry

Track per model/provider/workflow family:

- request success;
- structured-output validity;
- tool-call validity;
- verified task success;
- retries;
- latency;
- cost;
- context overflow;
- human rescue.

## 9.11 Budgets

Each task SHOULD define:

- max model spend;
- max model calls;
- max tokens where meaningful;
- max wall time;
- role-specific caps.

Budget exhaustion MUST be controlled.

## 9.12 Context limits

Adapter MUST surface context-limit errors distinctly.

Runtime MAY compact/summarize context but MUST preserve:

- trusted instructions;
- policy-relevant facts;
- unresolved approvals;
- side-effect history needed for recovery.

## 9.13 OpenAI-compatible endpoints

"Compatible" MUST pass Lumi contract tests.

Do not assume identical:

- tool semantics;
- structured output;
- streaming;
- cancellation;
- usage metadata.

## 9.14 Provider safety signals

Provider-specific safety/refusal signals MAY be surfaced.

They do not override Lumi policy.

## 9.15 Tests

Shared adapter suite MUST cover:

- text;
- tools;
- malformed tool output;
- structured output;
- streaming;
- cancellation;
- timeouts;
- rate limits;
- context overflow;
- usage metadata;
- images where supported;
- local-only routing;
- fallback constraints.


## 9.16 Driver, instance, catalog, route snapshot

Provider architecture MUST distinguish:

```text
ProviderDriver
  -> ProviderInstance
    -> ModelCatalog
      -> Task/Turn RouteSnapshot
```

### ProviderDriver

Represents one protocol/implementation family.

Examples:

- OpenAI;
- Anthropic;
- Gemini;
- Bedrock;
- OpenAI-compatible.

### ProviderInstance

Represents one concrete account/configuration boundary.

It owns or references:

- credentials;
- endpoint;
- account/organization;
- region;
- data policy;
- rate-limit/account state;
- provider-specific settings;
- cached model catalog.

Two instances using the same driver MUST NOT accidentally share mutable authentication, session, or catalog state.

### ModelCatalog

Catalog describes currently discovered models/capabilities for one instance.

Catalog state does not itself grant authorization.

### RouteSnapshot

Each task/turn SHOULD persist:

- provider driver;
- provider instance ID;
- model;
- router version;
- applicable policy version;
- capability match;
- privacy/data-egress classification;
- region/service tier where relevant.

Resume SHOULD reuse or explicitly re-evaluate this snapshot.

Configuration reload MUST NOT silently widen an active task's privacy, provider, region, or authority envelope.

## 9.17 Provider instance lifecycle

Provider instance health/setup checks MUST avoid unintended side effects such as:

- launching login flows;
- starting provider hooks/MCP servers;
- changing credential stores;
- creating durable sessions.

Setup/authentication is an explicit operation.

Sign-out/revocation MUST prevent admission of new sessions before clearing mutable credential/account state.

## 9.18 External agent harnesses

External local agent harnesses such as Codex CLI, Claude Code, or OpenCode are not ModelProvider adapters.

If supported, they MUST use a separate abstraction such as:

```text
AgentHarnessDriver
  -> HarnessInstance
  -> Normalized Lumi task/events
```

This may support bring-your-existing-subscription Work mode.

Rules:

- harness credentials remain execution-environment owned;
- harness provider/tool events are lower-trust observations;
- harness approval state does not replace Lumi policy;
- workflow schemas do not depend on harness CLI/event shapes;
- Workflow mode MAY prefer direct model/executor contracts where stronger control, accounting, or verification is required.

## 9.19 Additional tests

V1 MUST additionally test:

- two accounts using same ProviderDriver remain isolated;
- provider setup probe does not create a login/session side effect;
- RouteSnapshot persists across resume;
- policy/config tightening does not silently reroute active task outside its envelope;
- external harness approval cannot bypass Lumi policy;
- provider instance revocation blocks new task admission.
