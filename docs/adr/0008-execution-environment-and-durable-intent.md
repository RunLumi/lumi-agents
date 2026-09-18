# ADR 0008: Execution environments, capability negotiation, and durable intent

- Status: Accepted
- Date: 2026-09-18

## Decision

Treat the machine/runtime that owns local work as a first-class `ExecutionEnvironment`.

Environment-owned state includes local files, app/browser sessions, local secret references, native permissions, locally running provider/harness processes, executor capabilities, and runtime generation.

Remote clients supervise an environment through typed protocol. They do not substitute their own machine state or credentials.

Clients and runtimes negotiate capabilities explicitly because desktop, web, mobile, control plane, and local runtimes can upgrade independently.

For consequential actions, Lumi persists durable normalized intent and idempotency state before dispatch, then records execution result and verifies postconditions.

## Action outcomes

Executor outcomes are:

- DELIVERED
- REFUSED
- NO_EFFECT
- AMBIGUOUS
- ERROR
- CANCELLED

Unsupported/unproven is never implicit success.

## Why

This combines four evidence-backed lessons:

- T3 Code keeps the environment as owner of machine/workspace state.
- Codex exposes durable task/thread lifecycle and compatibility-aware protocol behavior.
- Cua Driver treats exact refusal and empirical capability matrices as first-class.
- Lumi requires safe crash recovery for business side effects.

## Consequences

- Task state records environment identity/runtime generation.
- Stale handles fail closed after restart/rebind.
- Unsupported capabilities reject or deliberately degrade.
- Acknowledging durable intent is not reporting completed work.
- Side-effect retry requires post-crash ambiguity checks.
- Normative v1 task/run, action/observation, compatibility, and control-plane specs carry these requirements.


## Normative specs

This ADR is implemented by:

- `docs/specs/v1/01-core-domain-model.md`;
- `docs/specs/v1/02-task-run-state-machine.md`;
- `docs/specs/v1/03-action-observation-protocol.md`;
- `docs/specs/v1/19-api-control-plane-sync.md`;
- `docs/specs/v1/20-versioning-compatibility-migrations.md`.
