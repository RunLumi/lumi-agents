# Model Capability Contract v0

Status: design contract for implementation.

## Capability metadata

```text
ModelCapabilities
- text
- reasoning
- vision
- tool_use
- structured_output
- embeddings
- native_computer_use
- streaming
- long_context
- local_execution
- allowed_data_regions
- max_context_tokens
- parallel_tool_calls
```

## Adapter behavior

Each provider adapter normalizes:

- input messages/context;
- tool definitions;
- streaming;
- structured output;
- image input;
- cancellation;
- usage metadata;
- retryable errors;
- terminal errors;
- context-limit errors.

## Routing output

The router returns:

- selected provider;
- model;
- reason;
- capability match;
- privacy-policy match;
- estimated cost;
- latency class;
- fallback set.

## Invariants

- No business logic branches on provider name.
- No local-only task may silently route to cloud.
- "OpenAI compatible" requires contract tests.
- Provider-specific safety signals may inform Lumi policy but do not replace it.
