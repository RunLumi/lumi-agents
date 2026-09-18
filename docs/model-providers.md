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
