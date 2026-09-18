# ADR 0001: Execution hierarchy

- Status: Accepted
- Date: 2026-09-18

## Decision

Lumi chooses the least fragile execution surface that can complete an action:

1. connector/API;
2. browser DOM/accessibility through Playwright or equivalent structured browser control;
3. native semantic automation through macOS Accessibility or Windows UI Automation;
4. deterministic app-specific adapter;
5. screenshot/vision/coordinate control.

Vision is a fallback, not the architecture.

## Why

Business workflows need repeatability, auditability, low latency, and recovery. Semantic actions carry more intent than coordinates and are easier to test. APIs are even stronger when they preserve the customer's permissions and business semantics.

The model is valuable for planning, extraction, ambiguity, and recovery. It should not be asked to visually rediscover a deterministic button on every run.

## Consequences

- Every executor reports its tier.
- Workflow packs may require a minimum tier for sensitive actions.
- Any non-read-only vision action requires approval in the default policy.
- Evals report success and intervention rate by tier so we can see where fragility enters the system.
