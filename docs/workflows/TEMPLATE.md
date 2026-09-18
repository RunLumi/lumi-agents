# Workflow pack template

## Business contract

- Workflow name:
- User:
- Economic owner:
- Human approver:
- Trigger:
- Verified business outcome:
- Current manual minutes/run:
- Expected runs/month:
- Maximum acceptable failure/side-effect:

## Systems and permissions

- Apps/domains:
- Connector/API scopes:
- Browser scopes:
- Native apps:
- Filesystem paths:
- Secrets required:
- Explicitly forbidden targets:

## Execution plan

For each step record:

- preferred execution tier;
- deterministic selector/action;
- precondition;
- postcondition;
- idempotency/retry behavior;
- risk class;
- approval requirement;
- evidence captured;
- exception route.

## Evals

- fixture/staging setup:
- task count:
- success threshold:
- intervention threshold:
- stop/change condition:
- known failure modes:

## Durable artifacts

Every customer deployment must leave behind a versioned workflow definition, policy manifest, fixture/eval pack, failure taxonomy, runbook, and measured ROI baseline.
