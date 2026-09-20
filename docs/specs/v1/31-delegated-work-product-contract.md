# 31 — Delegated Work Product Contract v1

Status: Normative product contract; not an implementation claim.

Composes [02 — Task/Run](02-task-run-state-machine.md),
[04 — Policy](04-policy-approval-capabilities.md),
[05 — Execution Router](05-execution-router.md),
[11 — Evidence](11-audit-evidence-verification.md),
[13 — Automations](13-scheduler-background-triggers.md),
[16 — Economics](16-evals-observability-economics.md),
[19 — Control Plane](19-api-control-plane-sync.md),
[23 — UX/Handoff](23-user-experience-handoff.md),
[26 — Projects](26-project-workspace-folder-as-project.md),
[27 — Global & Project Automations](27-global-and-project-automations.md),
[28 — Skills/Plugins](28-project-skills-plugins.md),
[29 — Connections](29-project-connections-secrets.md), and
[30 — Document Workspace](30-document-workspace-preview-edit.md).

## 31.1 Product outcome

Lumi v1 is not complete merely because it exposes agent primitives. It MUST make
delegation of substantial digital work trustworthy enough to become routine.

The primary product loop is:

```text
Open Project
→ delegate meaningful outcome
→ agent works across authorized files/tools/services
→ Lumi verifies the result
→ user sees what changed and what is proven
→ user supplies judgment only where required
→ repeated work can become an Automation
```

The expansion loop is:

```text
Work
→ repeated Task
→ Automation
→ hardened Workflow Pack
→ bounded operational responsibility
```

Project remains context/workspace, Task remains the goal, Run remains an attempt,
Action remains a concrete effect, and policy remains the source of authority.
This spec adds no alternate orchestrator or permission source.

## 31.2 Winning product contract

A conforming release MUST be exceptionally good at all of the following, not
merely contain a screen or interface for them:

1. **Fast real delegation.** A user can open an existing Project and delegate a
   meaningful outcome without translating it into tool calls.
2. **Durable execution.** Work survives app/runtime restart and can be stopped,
   resumed, redirected, or recovered without blind replay of ambiguous effects.
3. **Inspectable change.** Lumi distinguishes its changes from pre-existing or
   concurrent human changes and exposes reviewable diffs/artifacts.
4. **Verified completion.** Required postconditions and evidence determine
   success; model self-report and executor-native success do not.
5. **Attention efficiency.** Routine verified work stays quiet. Approvals,
   decisions, blockers, incidents, and failed verification reach one coherent
   Review Queue with enough context to act.
6. **Safe repetition.** A useful repeated Task can become an Automation without
   copying transient approvals, credentials, browser sessions, or widened
   authority.
7. **Interoperable execution.** Model/provider and executor choice are replaceable
   behind capability contracts. Fallback never weakens privacy or authority.
8. **Measured value.** Lumi can attribute verified work, human review/rescue
   minutes, latency, runtime cost, and residual work to Project/Task/Automation.

A release that is impressive in a demo but routinely requires users to redo,
reconstruct, or manually verify Lumi's work does not satisfy this contract.

## 31.3 Delegation surface

The primary Work surface MUST center the desired outcome and operational state,
not chat history.

For an active Task the UI MUST make it possible to answer:

- what outcome is being pursued;
- what Project and ExecutionEnvironment own the work;
- what phase is active;
- what has changed;
- what verification has passed/failed;
- whether Lumi needs human judgment;
- what can be stopped, redirected, resumed, or reviewed.

Operational progress SHOULD use bounded phase summaries such as understanding,
editing, validating, correcting, waiting for approval, and ready for review.
The product MUST NOT require exposing private chain-of-thought to make progress
inspectable.

Task completion SHOULD surface outcome, changes, evidence, attention state,
elapsed time, and attributable cost in one reviewable view.

## 31.4 Parallel delegated work

Lumi MUST support more than one active Task without allowing concurrency to
silently corrupt shared state.

For Git Projects, independent write-heavy Tasks SHOULD use isolated task
workspaces/worktrees when practical. For non-Git Projects, the runtime MUST use
explicit write/resource admission and conflict detection.

Parallel work MUST preserve:

- one durable Task/Run identity per delegated outcome;
- Project and ExecutionEnvironment binding;
- ownership of task workspace/change set;
- cancellation and recovery independently per Task;
- shared-resource arbitration from Spec 27;
- detection of external/human modifications before publication;
- explicit conflict state instead of last-writer-wins;
- no claim that another Task's or the user's edits were produced by this Run.

Parallelism is a throughput mechanism, not authority. Starting another Task MUST
NOT duplicate a credential grant, approval, unattended lease, or mutable external
session unless policy explicitly permits the shared capability.

## 31.5 Review Queue and attention contract

Review Queue is the cross-product attention surface for completed or blocked work.
It is not a second Task store and not a notification inbox.

Routine outcomes:

- VERIFIED_ROUTINE;
- NO_ACTION;
- NO_ALERT.

These MUST remain discoverable in durable History but SHOULD NOT create attention
noise unless explicit review policy requires it.

Actionable outcomes:

- APPROVAL_REQUIRED;
- NEEDS_DECISION;
- BLOCKED;
- VERIFICATION_FAILED;
- INCIDENT.

An actionable Review Item MUST identify the source Task/Run/Automation, Project,
reason, evidence summary, required human action, and whether any material side
effect has already occurred.

Read/unread, acknowledge, approve, resolve, retry, and continue-interactively are
distinct commands. Reading or dismissing presentation state MUST NOT authorize an
Action, resolve an incident, or mark an unverified Task successful.

Equivalent repeated failures SHOULD be grouped or rate-limited and MAY auto-pause
the responsible Automation under Spec 13. Review volume growing linearly with
healthy automation volume is a product failure signal.

## 31.6 Work → Automation conversion

A completed or repeated Task MAY offer **Create automation from this**.

Conversion MUST create a disabled draft first. It MAY carry:

- Project binding;
- normalized goal/task source;
- selected reusable Skill/Workflow reference;
- required inputs;
- proposed schedule/event;
- output/review policy;
- validation/postcondition template;
- compatible model/executor capability requirements.

Conversion MUST NOT copy:

- consumed or one-time approvals;
- unattended authority leases;
- plaintext secrets;
- opaque live browser/native sessions;
- transient task workspace locks;
- stale model context;
- external side-effect idempotency keys;
- read/incident state.

Before Enable, the draft MUST pass the same preview, dependency readiness, policy,
connection, test-run, schedule, and authority checks required by Specs 13/27–29.
A successful interactive Task is evidence for a candidate Automation, not proof
that unattended execution is safe.

## 31.7 Remote supervision

Web/mobile/remote clients are supervision surfaces before they are replacement
execution environments.

A remote client SHOULD support:

- inspect active work and verified outcome;
- review diff/report/evidence;
- answer a bounded question;
- approve/reject when authorized;
- pause/stop;
- redirect/continue;
- resume/fork where the environment advertises support.

The owning ExecutionEnvironment retains filesystem state, authenticated local
sessions, secret references, OS permissions, and executor state per Spec 19.
Remote authentication MUST NOT imply local credential possession or action
authority. Commands are durable requests with acknowledgement separate from
execution success.

## 31.8 Execution strategy

For consequential external work the router SHOULD prefer:

```text
structured connector/API
→ browser semantic
→ native semantic
→ deterministic app adapter
→ vision/coordinate
```

Selection MUST consider observed reliability, evidence quality, latency, cost,
policy, privacy, and current environment capability. Lower-semantic fallback MUST
not occur merely because it is convenient for the model.

A route that cannot verify target/account identity or required postconditions
MUST be treated as unsupported for a certified consequential workflow.

## 31.9 Project memory and reusable learning

Project memory MUST remain source-attributed, bounded, invalidatable, and separate
from authority. Lumi MAY learn reusable workflow structure from successful work,
but promotion to Skill, Automation, Workflow Pack, or regression fixture requires
an explicit artifact/review step.

Production failures that are reproducible SHOULD become regression cases with:

- triggering conditions;
- expected safe behavior;
- verifier/oracle;
- fixed version when known.

Lumi MUST NOT silently mutate executable Skills/Workflows based only on model
self-reflection from one Run.

## 31.10 Value and attention metrics

In addition to Spec 16, product telemetry SHOULD compute:

```text
human_attention_minutes =
  expected_approval_minutes
  + review_minutes
  + unexpected_rescue_minutes
  + rework_minutes

verified_value_per_human_attention_minute =
  verified_displaced_labor_value / max(human_attention_minutes, epsilon)

review_rework_ratio =
  (review_minutes + rework_minutes) / max(displaced_manual_minutes, epsilon)
```

Customer-facing views SHOULD headline verified work, human attention, exceptions,
cycle time, and cost per verified success rather than tokens, messages, tool
calls, or agent-hours.

## 31.11 Market-proof acceptance slice

Before claiming the delegated-work contract is proven, one exact release build
MUST demonstrate on real internal Projects:

- at least 50 substantial Project Tasks across representative repository/document
  work, with failures retained in the denominator;
- zero Project-root escapes;
- zero loss of pre-existing user changes;
- zero crash-induced duplicate consequential effects;
- >=95% verified completion on the declared certified internal matrix;
- every reproducible escaped failure represented in the regression corpus;
- parallel Tasks that either isolate writes or surface deterministic conflicts;
- restart/reopen/resume through the shipped desktop path;
- evidence-first review from the actual persisted state, not fixtures;
- at least one useful completed Task converted into a disabled Automation draft.

Automation market proof additionally requires at least 100 representative
occurrences on the declared acceptance repository/workflow, durable history,
actionable-only Review Queue behavior, and zero unauthorized external writes.

These are product decision thresholds, not customer guarantees.

## 31.12 Stop and narrow conditions

The team SHOULD narrow scope before adding breadth when any of the following is
observed after representative use:

- unexpected rescue remains above 15% after approximately 100 runs;
- users routinely redo Lumi output;
- Review Queue attention grows approximately with healthy Automation volume;
- users cannot explain why Lumi marked work successful;
- permissions are repeatedly widened to make workflows function;
- runtime/support cost consumes most measured displaced labor value;
- a third independent deployment remains substantially bespoke;
- feature expansion outpaces retained meaningful use.

The response to these signals is diagnosis, narrower scope, stronger verification,
or a different wedge, not a larger feature catalog.

## 31.13 Explicit non-goals

This contract does not require:

- a full IDE replacement;
- raw chain-of-thought display;
- a public marketplace;
- arbitrary multi-agent organizations;
- full mobile editing/runtime parity;
- broad enterprise control-plane completeness;
- universal Office fidelity;
- dozens of model providers;
- autonomous high-consequence financial/legal/personnel commitments.

Those capabilities MUST NOT block proving the core delegation loop.

## 31.14 Required tests

V1 MUST include tests proving:

1. two parallel Tasks cannot silently overwrite one another or user edits;
2. Review Queue routine outcomes remain in History without attention noise;
3. read/dismiss does not approve or resolve;
4. Task-to-Automation conversion copies no credential/approval/lease/session authority;
5. remote client cannot substitute local filesystem/credential state;
6. executor/provider fallback preserves privacy and policy;
7. completion cannot be verified without required evidence;
8. crash after possible external effect reconciles before retry;
9. customer-facing economics use verified-success and human-attention denominators;
10. all product surfaces resolve to the same durable Task/Run/Automation identities.

## 31.15 Done

This spec is complete when the shipped desktop path makes the following boring:

```text
Open Project
→ delegate
→ leave Lumi working
→ return later
→ inspect what changed
→ see proof
→ spend judgment only where needed
→ safely automate the repeated case
```

Passing interface tests without real persisted state, real model-driven work, and
representative end-to-end evidence does not satisfy this contract.
