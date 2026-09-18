# Action Protocol v0

Status: design contract for implementation.

## Goal

Normalize intent and side effects before policy and execution so provider, browser, desktop, connector, shell, and file engines can be replaced without changing business semantics.

## ActionProposal

```text
ActionProposal
- action_id
- workflow_id
- run_id
- task_id
- environment_id
- principal
- capability
- resource
- target
- arguments
- expected_effect
- risk_class
- evidence_requirements
- postconditions
- idempotency_key
- timeout
```

## Observation

External state enters Lumi as an Observation, not implicit authority.

```text
Observation
- observation_id
- task_id
- environment_id
- source
- provenance
- authority_class
- observed_at
- payload | payload_ref
- evidence_refs
```

Suggested authority classes:

- USER
- ORGANIZATION_POLICY
- CONNECTOR_EVENT
- TOOL_RESULT
- AGENT_MESSAGE
- WEB_CONTENT
- DOCUMENT_CONTENT

Only trusted authority classes may introduce or modify policy-relevant intent.

## Risk classes

- READ
- LOCAL_WRITE
- EXTERNAL_WRITE
- COMMUNICATION
- DATA_EXPORT
- CREDENTIAL
- FINANCIAL
- LEGAL_CONSENT
- DESTRUCTIVE
- ADMIN

## Policy decisions

- ALLOW
- DENY
- REQUIRE_APPROVAL

## Executor outcomes

Transport success is not effect success.

- `DELIVERED` — expected effect independently observed.
- `REFUSED` — executor deliberately refused before mutation under a known contract.
- `NO_EFFECT` — attempt occurred but expected effect was not observed.
- `AMBIGUOUS` — effect cannot be proven either way.
- `ERROR` — classified execution failure.
- `CANCELLED` — cancellation prevented completion.

`REFUSED` may be a correct expected outcome.

Unproven capability is a gap and must not be normalized to `DELIVERED`.

## Compatibility envelope

Every execution environment advertises:

- protocol version;
- runtime generation;
- environment identity;
- supported capabilities;
- executor versions where relevant.

Clients and orchestrators must negotiate capabilities instead of assuming synchronized release versions.

Unsupported fields/capabilities are rejected or deliberately degraded. They are never silently ignored when doing so changes semantics or authority.

## Temporary grants

A temporary permission grant is scoped and separate from policy.

It should bind to the narrowest practical set of:

- action/capability;
- resource/target;
- destination/origin;
- account/workspace;
- file root/path;
- material value/range when relevant;
- expiry;
- task/run;
- execution environment/runtime generation.

A temporary grant cannot widen organization/platform policy ceilings.

## Invariants

- Policy evaluates normalized business action, not raw UI gesture.
- Material action mutation invalidates prior approval.
- Executor adapters cannot skip policy.
- Required verification failure cannot report success.
- Action retry requires idempotency/ambiguity handling.
- External content does not become user authority by being present in an active task.
- Resume never blindly repeats an ambiguous consequential action.
- Forked tasks receive fresh identity for future side effects.
- Cancelled execution leases cannot continue dispatching actions.
- Stale environment/runtime-generation handles fail closed.
- Fallback across a stronger trust boundary requires explicit policy/approval.

## Compatibility

The public protocol must not contain provider-, Cua-, Playwright-, macOS-, Windows-, Electron-, or Tauri-specific business types.

Adapters translate implementation-specific values at the edge.

See also:

- `docs/specs/task-lifecycle-v0.md`
- `docs/architecture.md`
- `docs/security-model.md`
