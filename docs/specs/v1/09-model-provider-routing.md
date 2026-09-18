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
