# ADR 0005: Provider-neutral model architecture

- Status: Accepted
- Date: 2026-09-18

## Decision

Lumi core contracts are provider-neutral and capability-oriented.

Business workflows may not depend on provider request/response types.

Provider adapters translate between Lumi capabilities and vendor APIs.

## Initial adapter order

Tier 1:

- OpenAI
- Anthropic
- Gemini

Tier 2:

- Azure OpenAI
- AWS Bedrock
- OpenRouter
- xAI/Grok after contract validation
- generic OpenAI-compatible endpoints

Local/on-prem:

- Ollama
- vLLM
- validated compatible enterprise servers

## Routing

Provider selection considers, in order:

1. tenant data policy;
2. local/cloud requirement;
3. provider allowlist;
4. region/data residency;
5. required capabilities;
6. measured workflow reliability;
7. latency;
8. cost per verified success;
9. task budget.

No fallback may silently weaken privacy policy.

## Consequence

Provider-specific safety/confirmation semantics may be surfaced as observations, but Lumi local policy remains authoritative.
