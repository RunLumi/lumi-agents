# 02 — Task & Run State Machine v1

Status: Normative

## 2.1 Goal

Define durable lifecycle semantics for long-running, interruptible, resumable work.

## 2.2 Task states

Canonical task states:

- CREATED
- QUEUED
- RUNNING
- WAITING_APPROVAL
- WAITING_USER
- WAITING_EXTERNAL
- PAUSED
- COMPLETED
- FAILED
- AMBIGUOUS
- CANCELLED

## 2.3 Run states

A Run MAY use finer states:

- INITIALIZING
- PLANNING
- EXECUTING
- VERIFYING
- CHECKPOINTING
- RECOVERING

Run state always rolls up to a task-visible state.

## 2.4 Allowed transitions

Typical transitions:

CREATED -> QUEUED -> RUNNING

RUNNING -> WAITING_APPROVAL  
RUNNING -> WAITING_USER  
RUNNING -> WAITING_EXTERNAL  
RUNNING -> PAUSED  
RUNNING -> COMPLETED  
RUNNING -> FAILED  
RUNNING -> AMBIGUOUS  
RUNNING -> CANCELLED

WAITING_* -> RUNNING | CANCELLED | FAILED

PAUSED -> RUNNING | CANCELLED

AMBIGUOUS -> WAITING_USER | FAILED | COMPLETED only after explicit verification.

COMPLETED, FAILED, CANCELLED are terminal for a run.

## 2.5 Durable checkpoints

A checkpoint MUST include:

- run_id;
- step position;
- completed step IDs;
- pending actions;
- last verified side effect;
- retry counters;
- consumed budgets;
- relevant model/provider context refs;
- workflow state refs;
- timestamp.

Checkpoints MUST NOT include plaintext secrets.

## 2.6 Side-effect boundary

Before consequential execution, runtime SHOULD persist a pre-action checkpoint containing:

- normalized action;
- idempotency metadata;
- approval reference if required;
- verifier plan.

After execution, runtime MUST persist the result before advancing.

## 2.7 Ambiguous state

If the runtime cannot determine whether a side effect succeeded, it MUST enter AMBIGUOUS or an equivalent blocked state.

It MUST NOT blindly retry.

Examples:

- network timeout after form submission;
- application crash after save click;
- provider disconnect after external API request.

Recovery MUST inspect external state/postconditions before deciding to retry.

Failure to persist pre-action intent or the side-effect journal MUST prevent
executor invocation. All orchestrator constructors must load existing recovery
state; an unreadable journal cannot become an empty fresh run. Persistence
failure after dispatch prevents a success report and stops further admission
until storage and external state are independently reconciled.

A direct repeated execution call must obey the same journal guard as resume.
Changing a run/action identifier does not prove a new business effect is safe.
Old records with unknown required metadata must fail closed, rather than be
silently upgraded to permissive state.

## 2.8 Resume

Resume MUST:

1. reload compatible task/run state;
2. validate policy version compatibility;
3. validate workflow version/migration;
4. validate device capability availability;
5. re-check pending approvals and expiry;
6. verify ambiguous side effects;
7. continue from a safe checkpoint.

## 2.9 Cancellation

Cancellation MUST be cooperative and SHOULD be prompt.

Cancellation MUST:

- stop new model/tool work;
- interrupt cancellable executor work;
- not corrupt persisted state;
- record cancellation in audit;
- not roll back already-completed real-world side effects unless workflow explicitly defines compensating actions.

## 2.10 Pause

Pause differs from cancel:

- state is retained;
- no new side effects occur;
- user MAY resume later.

Long pauses MAY require reauthorization depending on policy.

## 2.11 Retry classes

Retries MUST distinguish:

- TRANSIENT
- RATE_LIMIT
- AUTH_REFRESHABLE
- STALE_STATE
- NON_RETRYABLE
- AMBIGUOUS_SIDE_EFFECT

Only retryable classes MAY auto-retry.

## 2.12 Retry budgets

Every retry loop MUST have:

- max attempts;
- backoff;
- deadline;
- budget accounting.

Unlimited retry loops are forbidden.

## 2.13 External waiting

WAITING_EXTERNAL SHOULD be used for:

- asynchronous export jobs;
- long-running remote processing;
- file availability;
- delayed webhook/event.

Polling MUST have a bounded interval and deadline.

## 2.14 Approval waiting

WAITING_APPROVAL MUST expose:

- human-readable reason;
- exact normalized action;
- risk;
- evidence context;
- expiry.

If action material changes, prior approval MUST be invalidated.

## 2.15 Completion

Task may be marked COMPLETED only when:

- required steps are complete;
- required postconditions pass;
- required artifacts validate;
- no unresolved consequential ambiguity remains.

## 2.16 Recovery tests

V1 MUST include tests for:

- crash before side effect;
- crash after side effect but before response persistence;
- expired approval;
- provider failure during planning;
- worker failure during browser/native action;
- restart from checkpoint;
- cancellation;
- policy version change.


## 2.17 Task lifecycle operations

The runtime MUST model these operations explicitly rather than encoding them as ordinary chat messages:

- CREATE;
- RESUME;
- FORK;
- STEER;
- PAUSE;
- CANCEL;
- CHECKPOINT;
- ARCHIVE.

### Fork

Fork creates a new task/run lineage from a known checkpoint/history boundary.

Future side effects MUST use new task/run/action/idempotency identity.

A fork MUST NOT reuse a parent action ID to repeat a real-world side effect.

### Steer

Steer adds trusted user direction to an active task.

Steer MUST obey the same platform/organization policy ceiling as the original task.

External connector/webhook/email events are observations/triggers, not equivalent to user steer.

### Archive

Archive removes work from active views without violating audit/evidence retention policy.

## 2.18 Provider route snapshot

Each run SHOULD persist the provider/model routing snapshot needed to explain and safely resume work:

- provider driver;
- provider instance/account;
- model;
- router/policy version;
- privacy/data-egress classification;
- capabilities relied upon.

Resume MUST NOT silently reroute outside the original or newly authorized privacy/policy envelope.

## 2.19 Environment compatibility on resume

Resume MUST validate:

- environment identity;
- runtime generation;
- required runtime capabilities;
- executor compatibility.

Stale runtime/session handles from a previous generation MUST fail closed.
