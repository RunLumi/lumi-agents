# Action Protocol v0

Status: design contract for implementation.

## Goal

Normalize side effects before policy and execution.

## Core types

```text
ActionProposal
- action_id
- workflow_id
- run_id
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

## Decisions

- ALLOW
- DENY
- REQUIRE_APPROVAL

## Invariants

- Policy evaluates normalized business action, not raw UI gesture.
- Material action mutation invalidates prior approval.
- Executor adapters cannot skip policy.
- Required verification failure cannot report success.
- Action retry requires idempotency/ambiguity handling.

## Compatibility

The public protocol must not contain provider-, Cua-, Playwright-, macOS-, or Windows-specific types.
