# 13 — Scheduler, Background Work & Triggers v1

Status: Normative

## 13.1 Goal

Run bounded unattended work safely from schedules and events.

## 13.2 Trigger types

V1 MAY support:

- cron/schedule;
- webhook;
- connector event;
- file/folder event;
- inbox/queue event;
- device-local event;
- manual enqueue.

## 13.3 Trigger normalization

Trigger MUST normalize into a task creation request containing:

- tenant;
- principal;
- workflow/task template;
- trigger payload;
- timestamp;
- deduplication key;
- policy context.

## 13.4 Deduplication

Event triggers SHOULD support deduplication key/window.

Duplicate trigger MUST NOT create duplicate consequential work.

## 13.5 Unattended lease

Managed background execution MUST require a revocable lease or equivalent policy authorization.

Lease SHOULD include:

- device;
- workflow;
- capability scope;
- expiry;
- budget.

## 13.6 Background limits

Every background task MUST have:

- deadline;
- model/action budget;
- retry budget;
- cancellation path;
- audit.

## 13.7 Visibility

Employee-facing app MUST show active unattended work when it uses local device resources or user sessions.

## 13.8 Quiet hours

Organization/user policy MAY define quiet hours or no-interruption periods.

## 13.9 Device availability

If device unavailable:

- task MAY wait;
- MAY route to eligible device/cloud worker if policy permits;
- MUST NOT silently weaken locality/privacy constraints.

## 13.10 Events with sensitive payloads

Trigger payload SHOULD contain references, not unnecessary full sensitive content.

## 13.11 Schedule timezone

Schedule MUST persist timezone and DST semantics explicitly.

## 13.12 Missed schedules

Workflow must define catch-up policy:

- SKIP
- RUN_ONCE
- RUN_EACH_MISSED
- REQUIRE_USER

Default SHOULD avoid burst duplicate side effects.

## 13.13 Approval in background

If approval required:

- task enters WAITING_APPROVAL;
- no material action proceeds;
- approval expiry enforced.

## 13.14 Tests

V1 MUST test:

- duplicate webhook;
- missed schedule;
- device offline;
- lease revoked mid-run;
- budget exhausted;
- approval delayed;
- local-only task cannot cloud failover.
