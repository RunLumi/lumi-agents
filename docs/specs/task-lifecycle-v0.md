# Task Lifecycle v0

Status: design contract.

## Goal

Make long-running work controllable and resumable without pretending every state change is another chat message.

## Operations

### create

Creates a new task with:

- goal;
- execution environment;
- policy profile;
- provider route policy;
- budget;
- initial resources.

### resume

Reopens persisted task state without replaying side effects.

### fork

Creates a new task branch from a known checkpoint/history boundary.

Forked tasks receive new task/run IDs and independent future side effects.

### steer

Adds trusted user direction to an active task.

Steer is not the same as an external event.

### pause

Stops new action admission while preserving durable state.

### cancel

Stops work and invalidates future execution leases.

Cancellation cannot undo completed external effects.

### checkpoint

Persists recoverable state appropriate to the work surface.

### archive

Removes task from active views without deleting audit/evidence according to retention policy.

## External event

An external event is lower-authority input, not a user instruction.

Examples:

- webhook;
- email;
- Slack event;
- file changed;
- connector event.

It carries:

- source;
- provenance;
- observed_at;
- authority_class;
- payload/evidence reference.

## Invariants

- Resume never blindly replays an ambiguous side effect.
- Fork never reuses idempotency identity for future side effects.
- Steer obeys the same policy ceiling as the original task.
- Cancelled task leases cannot continue executing.
- External events cannot grant new authority.
