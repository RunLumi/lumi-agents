# Model Provider Architecture

## Principle

Lumi workflows depend on capabilities, not provider names.

Business logic must not encode OpenAI-, Anthropic-, Gemini-, or any other vendor-specific message shapes.

## Capability contract

A provider adapter should expose metadata similar to:

```text
text
reasoning
vision
tool_use
structured_output
embeddings
native_computer_use
streaming
long_context
local_execution
data_regions
context_limit
parallel_tools
```

Capabilities are versioned and testable.

## Adapter waves

### Tier 1

- OpenAI
- Anthropic
- Gemini

### Tier 2

- Azure OpenAI
- AWS Bedrock
- OpenRouter
- xAI/Grok after contract validation
- generic OpenAI-compatible endpoints

### Local/on-prem

- Ollama
- vLLM
- validated enterprise OpenAI-compatible servers

## Routing

Provider selection order:

1. tenant privacy/data policy;
2. local/cloud constraint;
3. provider allowlist;
4. region/data residency;
5. required capabilities;
6. measured workflow success;
7. latency;
8. cost per verified success;
9. remaining task budget.

No provider fallback may silently weaken privacy.

## Roles

Different models may handle different roles:

- planner;
- extraction/classification;
- vision;
- code;
- verifier;
- embeddings;
- summarization.

Do not route everything to the largest model by default.

## Reliability registry

Track provider/model performance per workflow family:

- verified success;
- latency;
- retries;
- tool-format failures;
- human rescue;
- variable cost;
- vision failures;
- context overflows.

Routing should learn from measured performance.

## Budgets

Per task define:

- max model calls;
- max inference cost;
- max wall-clock time;
- max tool actions;
- max retries;
- max vision actions.

Budget exhaustion creates a controlled exception.

## Provider fallback

Fallback is allowed only when:

- required capability still exists;
- data policy remains satisfied;
- workflow allows fallback;
- material behavior differences are handled.

Local-only means local-only.

## Contract testing

Every adapter must pass a shared test suite covering:

- text;
- streaming;
- structured output;
- tools;
- tool errors;
- cancellation;
- timeout;
- image input where supported;
- usage/cost metadata;
- retryable vs terminal errors;
- context-limit behavior.

"OpenAI compatible" is not sufficient evidence of semantic compatibility.

## Provider-specific safety

Do not normalize away safety-relevant provider behavior.

Adapters may surface provider-specific confirmation/safety signals into Lumi policy as observations, but Lumi policy remains authoritative.


## Driver, instance, catalog, session

Do not model a provider as one global singleton.

Use four layers:

```text
ProviderDriver
  -> ProviderInstance
    -> ModelCatalog
      -> Session / Task route
```

### ProviderDriver

The implementation/protocol family, such as:

- OpenAI;
- Anthropic;
- Gemini;
- Bedrock;
- OpenAI-compatible.

### ProviderInstance

One concrete account/configuration boundary.

It owns or references:

- credentials;
- endpoint/region;
- organization/account;
- data policy;
- rate limits;
- provider-specific settings;
- cached model catalog.

Two accounts using the same driver must not share mutable authentication/session state accidentally.

### ModelCatalog

The capability description discovered for one provider instance.

Catalog data is evidence about availability, not durable authorization.

### Route snapshot

When a task/turn starts, persist the routing decision:

- provider driver;
- provider instance;
- model;
- router/policy version;
- capability match;
- data-egress classification;
- service tier when relevant.

A configuration reload must not silently change an active task's trust posture.

If continuing work becomes incompatible with updated managed requirements, stop or require an explicit reroute.

## External agent-harness adapters

A local coding/work harness such as Codex CLI, Claude Code, or OpenCode is **not** the same abstraction as a model provider.

Lumi may later support:

```text
AgentHarnessDriver
  -> local authenticated harness instance
  -> normalized Lumi task/events
```

This can be useful in Work mode for bring-your-existing-subscription adoption.

Rules:

- harness events are normalized at the adapter edge;
- harness approval state does not supersede Lumi policy;
- harness credentials remain environment-owned;
- Workflow mode should prefer direct provider/executor contracts when stronger control, cost accounting, or verification is required.

Do not make Lumi's core dependent on the lifecycle or CLI format of one external harness.

## Authority of external model events

Provider/tool/harness output is observation, not user authority.

External notifications should preserve provenance and authority class so the orchestrator can distinguish:

- trusted user steer;
- developer/system policy;
- connector event;
- provider-generated tool result;
- another agent's message;
- untrusted web/document content.

No provider event grants new permissions merely because it arrived inside an active task.
