# 13 — Automations, Scheduler & Background Triggers v1

Status: Normative

## 13.1 Goal

Run recurring, one-shot, event-driven, and monitoring work safely against a Lumi Project.

An **Automation** is a durable instruction to create real Tasks/Runs later.

It is not a cron wrapper and it is not a new source of authority.

The core model is:

```text
Automation
  = Trigger
  + Project / ExecutionEnvironment
  + Prompt / Skill / Workflow reference
  + Context mode
  + Authority lease
  + Run policy
  + Delivery policy
```

When an Automation fires:

```text
trigger becomes due
→ scheduler evaluates admission
→ create durable Task/Run
→ resolve current Project + automation revision
→ intersect stored authority with current policy
→ execute through normal orchestrator/policy/verifier path
→ persist result/evidence/history
→ deliver only according to delivery policy
```

A scheduled run MUST use the same policy, approval, evidence, verification, cancellation, and Project boundaries as interactive work.

## 13.2 Product principles

V1 follows these principles:

1. **Test before scheduling.** A prompt/workflow SHOULD run manually before unattended activation.
2. **Fresh by default.** Each recurring run SHOULD start with fresh model context and explicit durable state.
3. **Project is durable context.** Local automations bind to a Project and its owning ExecutionEnvironment.
4. **Scheduler creates work; it does not authorize work.**
5. **No silent authority growth.** Stored authority is a ceiling; current policy may narrow it further.
6. **Every run is inspectable.** Success, no-action, blocked, skipped, failed, and cancelled runs appear in history.
7. **Silence is a delivery decision, not missing evidence.** A healthy monitor may produce no notification while still creating a successful run record.
8. **Repository prompts may define work, not permissions.**
9. **Background work is reviewable.** A completed run can be opened, inspected, and continued interactively.
10. **Failures must decay safely.** Repeated broken automations back off and eventually pause instead of burning resources forever.

## 13.3 Automation domain object

A persisted Automation MUST include at least:

- automation_id;
- tenant_id;
- owner/principal;
- display_name;
- enabled state;
- automation revision;
- Project binding when Project-scoped;
- ExecutionEnvironment binding or eligible-environment policy;
- trigger definition;
- task source;
- context mode;
- workspace mode;
- stored authority/lease reference;
- run policy;
- delivery policy;
- created_at;
- updated_at;
- next_run_at when time-based;
- last_run summary;
- failure/block counters.

An Automation MAY include:

- description;
- tags;
- model/provider preference;
- reasoning effort preference;
- quiet notification hours;
- activation/canary policy;
- end condition;
- start/end date;
- event follow-up offsets;
- output contract.

## 13.4 Automation states

Canonical lifecycle:

```text
DRAFT
  → ENABLED
  ↔ PAUSED
  → NEEDS_REVIEW
  → AUTO_PAUSED
  → ARCHIVED
```

Meaning:

- `DRAFT` — not eligible to fire.
- `ENABLED` — scheduler may admit runs.
- `PAUSED` — operator paused intentionally.
- `NEEDS_REVIEW` — material configuration/authority/environment change requires review before new runs.
- `AUTO_PAUSED` — runtime paused after repeated failures/blocks/schedule errors.
- `ARCHIVED` — retained for history, never fires.

Deleting an Automation MUST NOT delete its historical Tasks/Runs/evidence by default.

## 13.5 Task source

An Automation MUST resolve exactly one primary task source:

1. project-relative prompt file;
2. inline prompt;
3. Skill;
4. Workflow Pack;
5. Role/queue entrypoint where supported.

For Project-native work, prompt/skill references SHOULD be relative to the Project instead of copied into the Automation record.

Example:

```yaml
task:
  prompt_file: prompts/daily-ads-ops.md
```

The runtime MUST record the resolved source hash for every run.

For Git Projects it SHOULD also record:

- repository HEAD;
- branch/ref;
- dirty-state summary;
- prompt/skill file hashes;
- relevant config hashes where declared.

Changing prompt text does not itself grant broader capability.

A task source change that requests capabilities outside the stored automation authority MUST fail closed or move the Automation to `NEEDS_REVIEW`.

## 13.6 Skill resolution

A task source MAY explicitly reference one or more skills.

Skills remain instruction/capability declarations under spec 24 and MUST NOT grant authority.

The Project skill resolver MAY support approved compatibility locations such as:

- `.lumi/skills/`;
- `.codex/skills/`;
- organization-managed skill registries.

An Automation SHOULD explicitly name the required Skill when reliable unattended execution should not depend on model tool/skill discovery.

## 13.7 Trigger types

V1 MUST support:

### AT

One-shot timestamp.

Use for:

- reminder/follow-up;
- post-change check;
- delayed verification.

### INTERVAL

Fixed recurrence such as every 4 hours.

Use for:

- health checks;
- periodic polling where wall-clock alignment is unimportant.

### CRON

Cron schedule with explicit IANA timezone.

Use for:

- daily/weekly operational jobs;
- fixed wall-clock routines.

### RRULE

RFC 5545 recurrence rule with explicit timezone.

Use when cron becomes awkward, such as first weekday of a month.

### EVENT

Normalized event from:

- connector;
- webhook;
- another Lumi workflow/run;
- file/folder watcher;
- device-local event;
- explicit manual event.

Every event MUST carry a stable event identity/idempotency key.

V1 MAY support condition watchers as an optional gate on AT/INTERVAL/CRON/RRULE/EVENT.

## 13.8 Event follow-up plans

An event Automation MAY define follow-up offsets.

Example:

```yaml
trigger:
  kind: event
  event: ads.launch.completed
  followups: [1h, 4h, 24h, 48h, 72h]
```

The parent event produces one logical follow-up series.

Each child occurrence MUST have a deterministic deduplication key:

```text
<automation_id>:<event_id>:<offset>
```

Cancelling the follow-up series MUST cancel only pending occurrences, not erase completed history.

This primitive exists for real operational workflows such as:

- launch watch;
- post-change verification;
- deployment follow-up;
- customer onboarding checkpoints.

Do not implement these as ad-hoc sleep loops inside one long-running model turn.

## 13.9 Condition gate

A condition gate is a cheap, bounded, read-only preflight that decides whether to create the full Task.

It MUST return structured output:

```json
{
  "fire": true,
  "message": "optional event context",
  "state": {}
}
```

Rules:

- condition evaluation MUST be bounded by tool/time budget;
- it MUST NOT perform consequential external writes;
- its persisted state MUST be size-bounded;
- state persists only after successful evaluation;
- failure MUST be visible and MUST NOT masquerade as “condition false”;
- condition state is not model memory.

Use condition gates only when they materially avoid expensive/noisy full runs.

For ads health monitoring, a full 4h read-only run is acceptable; do not add a fragile gate merely to save a few model calls.

## 13.10 Timezone and DST

Every wall-clock schedule MUST persist an IANA timezone.

The scheduler MUST NOT infer user timezone at each future run.

DST behavior MUST be deterministic and documented.

For ambiguous/non-existent wall times the schedule implementation MUST choose and persist one explicit policy, for example:

- first valid occurrence;
- second valid occurrence;
- skip;
- shift forward.

A displayed schedule MUST include timezone.

## 13.11 Jitter and exact timing

Recurring schedules MAY specify a bounded jitter/stagger window to avoid thundering-herd starts.

Example:

```yaml
timing:
  jitter: 5m
```

Jitter MUST NOT violate a user-declared exact-time requirement.

The UI MUST distinguish:

- exact schedule;
- flexible schedule/window.

## 13.12 Project and workspace binding

A local Automation SHOULD bind to one durable Project from spec 26.

Workspace modes:

- `PROJECT_ROOT`
- `ISOLATED_WORKTREE`
- `SANDBOX_PROJECTION`

Default for source-code mutation SHOULD be `ISOLATED_WORKTREE` when Git is available.

Default for operational repositories that intentionally persist reports/worklogs MAY be `PROJECT_ROOT` when policy allows repo writes and overlap is controlled.

A scheduled run MUST record the concrete workspace identity it used.

It MUST NOT silently switch from isolated worktree to live Project root.

For non-Git Projects, scheduled work runs in the Project root or an explicitly configured sandbox projection.

## 13.13 ExecutionEnvironment availability

Automations that require local files, browser sessions, native apps, local models, or local credentials are bound to an eligible ExecutionEnvironment.

If unavailable:

- the run MAY wait;
- MAY be skipped according to run policy;
- MAY reroute only when current policy explicitly allows an equivalent environment;
- MUST NOT silently weaken locality/privacy constraints.

A local-only Automation MUST never fail over to cloud execution.

Local browser/session requirements SHOULD be displayed before enabling the Automation.

## 13.14 Context mode

V1 defines:

### FRESH

Default.

Each run starts a fresh model context.

Durable continuity comes from:

- Project files;
- artifacts;
- evidence;
- Automation state;
- workflow state;
- explicit Project memory.

Use for routine reports, monitors, and independently auditable runs.

### PERSISTENT_AUTOMATION

Runs deliberately share an automation-scoped persistent context.

Use only where prior run summaries materially improve the next run.

Persistent context MUST remain bounded and policy-controlled.

### RESUME_TASK

A future occurrence resumes one specific durable Task.

Use for “check this again later and continue” patterns.

The Automation MUST bind to the exact Task/Project/Environment and re-observe relevant state before new mutation.

Automation context MUST NOT accidentally inherit arbitrary chat routing, temporary elevation, stale approvals, or secrets.

## 13.15 Authority and unattended lease

Every Automation capable of using tools MUST have explicit stored authority.

The authority model is:

```text
effective run authority
  = automation stored ceiling
  ∩ current organization policy
  ∩ current user/device policy
  ∩ Project scope
  ∩ current runtime capabilities
```

The result may only become narrower over time without explicit reauthorization.

An unattended lease SHOULD bind:

- automation_id;
- principal;
- Project;
- device/environment;
- capability scope;
- external-write scope;
- model/data-egress scope;
- expiry or reauthorization policy;
- action/cost budget.

Revocation MUST stop new admission and halt active runs at the next policy gate.

Repository instructions, prompt files, Skills, models, web pages, and event payloads MUST NOT widen the lease.

## 13.16 Background approvals

Unattended execution has no implicit human present.

When a run reaches an action requiring approval:

```text
RUNNING
→ WAITING_APPROVAL
→ approval expires/rejected/approved
```

No material side effect may proceed before approval.

Approval MUST bind to the exact normalized action under spec 04.

The Automation itself remains enabled unless policy says otherwise.

A run waiting for approval MUST have a deadline/expiry path.

For a read-only scheduled Ads run, discovering an Ads-write recommendation is NOT approval to act; the correct result is a reviewable recommendation or `BLOCKED_APPROVAL_REQUIRED`.

## 13.17 Run admission

Before creating/executing a run, scheduler admission MUST evaluate:

1. Automation enabled state;
2. due/event trigger validity;
3. timezone/window;
4. start/end date;
5. Project/environment health;
6. current capability availability;
7. unattended lease validity;
8. deduplication;
9. overlap policy;
10. catch-up policy;
11. quiet/execution window where configured;
12. global/project/automation budgets.

A scheduler rejection MUST be recorded with a reason when it represents a real due occurrence.

## 13.18 Deduplication

Every event trigger SHOULD have an explicit idempotency key.

Time-based occurrences SHOULD derive a stable occurrence identity from:

```text
automation_id + intended_schedule_window
```

Duplicate delivery MUST NOT create duplicate consequential work.

Deduplication happens before Task creation when possible.

## 13.19 Overlap policy

Every recurring Automation MUST define an overlap policy.

Canonical values:

- `SKIP_IF_RUNNING` — default for monitors/ops;
- `QUEUE_ONE` — retain at most one pending occurrence;
- `CANCEL_AND_REPLACE` — cancel old run only when workflow declares replacement safe;
- `PARALLEL` — requires explicit policy and independent scopes.

Default MUST be `SKIP_IF_RUNNING`.

Never create an unbounded queue of missed recurring work.

Ads monitoring SHOULD use `SKIP_IF_RUNNING` because two agents operating the same authenticated Ads session provide little value and increase ambiguity.

## 13.20 Missed-run policy

Each schedule MUST define catch-up behavior:

- `SKIP`
- `RUN_ONCE`
- `RUN_EACH_MISSED`
- `REQUIRE_USER`

Default SHOULD be `RUN_ONCE` for ordinary recurring work.

A run policy MAY specify a maximum staleness:

```yaml
catch_up:
  policy: RUN_ONCE
  max_age: 2h
```

If a daily Ads report is six hours late but still decision-useful it may run once.

If a +1h launch-watch check is already +9h late, the stale occurrence SHOULD be skipped or folded into the next meaningful checkpoint rather than pretending it is still a +1h observation.

## 13.21 Retry and backoff

Retry policy MUST distinguish:

- transient infrastructure/provider error;
- deterministic invalid configuration;
- business `BLOCKED` outcome;
- approval wait;
- semantic no-action success.

Transient run failures MAY retry with bounded exponential backoff.

A retry MUST preserve run/occurrence identity and idempotency semantics.

Never blindly retry an ambiguous consequential side effect.

Recommended recurring backoff profile:

```text
1m → 5m → 15m → 60m
```

Exact values remain configurable.

A successful run resets the execution-failure streak.

## 13.22 Auto-pause

To prevent unattended failure loops, an Automation SHOULD auto-pause after repeated equivalent failures.

Recommended defaults:

- 5 consecutive execution failures; or
- 3 consecutive schedule/configuration failures.

A workflow MAY also define repeated-block handling, for example:

```yaml
auto_pause:
  same_block_code_after: 3
```

This is especially useful for conditions such as expired browser login: repeatedly launching a full Ads session every 4h while authentication is broken wastes cost and attention.

Auto-pause MUST create one actionable notification containing:

- automation;
- reason;
- latest run;
- suggested recovery;
- explicit Resume action.

## 13.23 Semantic outcome vs execution status

Automation execution status and business outcome are distinct.

Execution status:

- `SUCCEEDED`
- `BLOCKED`
- `FAILED`
- `CANCELLED`
- `SKIPPED`
- `AMBIGUOUS`

Business outcome is workflow-defined, for example:

- `NO_ALERT`
- `NO_ACTION`
- `WATCH`
- `INCIDENT`
- `READY_FOR_HUMAN_LAUNCH`
- `NOT_READY`
- `BLOCKED_LOGIN`
- `BLOCKED_APPROVAL_REQUIRED`

`NO_ALERT` and `NO_ACTION` are successful outcomes.

Do not infer success from notification delivery.

The runtime SHOULD receive semantic outcome structurally from the workflow/agent completion contract instead of parsing prose.

Compatibility adapters MAY recognize explicit terminal codes while migrating older prompt-based repositories.

## 13.24 Run history

Every admitted run MUST create durable history containing at least:

- run_id;
- automation_id + revision;
- intended occurrence;
- trigger/event identity;
- started/ended timestamps;
- Project/environment/workspace;
- task source + hashes;
- model/provider route;
- effective capability/lease reference;
- execution status;
- semantic outcome;
- cost/budget;
- approvals;
- evidence refs;
- artifact refs;
- changed-files summary when applicable;
- delivery result;
- failure/block reason;
- retry lineage.

Run history MUST survive application restart.

UI SHOULD support:

- recent runs;
- filter by outcome/status;
- exact run detail;
- retry/manual run;
- open result;
- continue interactively.

## 13.25 Review Queue

Automation completion and notification are separate concerns.

A run MAY:

- appear only in History;
- appear in Review Queue;
- notify user;
- require approval;
- escalate as failure/incident.

Typical policies:

### HISTORY_ONLY

Healthy routine runs remain inspectable but silent.

### REVIEW_ALWAYS

Every completed run appears in Review Queue.

### REVIEW_ACTIONABLE

Only actionable semantic outcomes appear.

### NOTIFY_ACTIONABLE

Notify only for configured outcomes such as `INCIDENT`, `WATCH`, `BLOCKED`, `FAILED`.

Review Queue items SHOULD show:

- Automation name;
- semantic outcome;
- one-sentence result;
- Project;
- run time;
- key evidence/artifacts;
- approval/block;
- Continue Interactively action.

Opening a review item MUST NOT imply approval of pending actions.

## 13.26 Delivery policy

Delivery policy MAY target:

- Lumi Review Queue;
- desktop notification;
- connector/channel;
- webhook;
- none.

External delivery is itself an external side effect and remains policy-bound.

Webhook targets MUST follow SSRF/network policy and SHOULD use explicit allowlists.

Delivery failure MUST NOT rewrite a successful task execution as a failed business workflow unless delivery is explicitly required for completion.

Record execution status and delivery status separately.

## 13.27 Notification quiet hours

Notification quiet hours are distinct from execution quiet hours.

A monitor MAY continue running overnight while suppressing non-critical notifications.

Critical incidents MAY override notification quiet hours when policy permits.

Do not skip safety monitoring merely because notification delivery is quiet.

## 13.28 Activation and canary

An Automation SHOULD support a simple maturity gate:

```text
DRAFT
→ manual test
→ CANARY
→ ACTIVE
```

Recommended controls:

- manual test required before enable;
- first N runs marked for review;
- read-only authority by default;
- explicit promotion to broader authority.

The Automation definition MAY contain:

```yaml
activation:
  require_manual_test: true
  review_first_runs: 5
```

This maps directly to operational repositories that require several reviewed manual runs before unattended scheduling.

## 13.29 Manual runs

A user MUST be able to run an Automation now without altering its enabled/paused schedule state.

Manual execution MUST still use:

- the Automation revision;
- Project binding;
- effective policy;
- run history;
- evidence;
- delivery rules unless explicitly overridden.

A “Run test” mode MAY force Review Queue delivery and prohibit external writes.

## 13.30 Edit and reauthorization

Editing these fields MUST require policy re-evaluation and MAY require reauthorization:

- Project/root;
- ExecutionEnvironment;
- workspace mode;
- capability scope;
- external-write scope;
- connector/account scope;
- model/data-egress class;
- webhook/delivery destination.

Editing schedule timing alone SHOULD NOT widen authority.

Every edit creates a new Automation revision.

Runs always record the revision that fired.

## 13.31 Cancellation

User/admin MUST be able to:

- pause Automation;
- cancel active run;
- disable future runs;
- revoke lease;
- emergency-stop local execution.

Pausing future fires does not automatically cancel an active run unless user selects that action.

Lease revocation MUST stop the active run at the next gate.

## 13.32 Natural-language creation

The product MAY let users say:

> Run `prompts/daily-ads-ops.md` every day at 09:00 Asia/Ho_Chi_Minh.

Lumi MAY draft the Automation but MUST present a reviewable summary before activation:

- what runs;
- Project;
- when;
- timezone;
- workspace;
- tools/capabilities;
- external-write authority;
- output/review policy;
- retry/auto-pause;
- next run.

Natural-language convenience MUST NOT obscure the stored structured definition.

## 13.33 Repo-native Automation manifest

A Project MAY declare shareable Automation definitions in a versioned manifest.

Canonical v1 location:

```text
.lumi/automations.yaml
```

The manifest is desired configuration, not direct authority.

Import flow:

```text
discover manifest
→ parse/validate
→ show diff vs installed Automations
→ operator imports/enables
→ policy creates bounded leases
```

Changing the manifest MUST NOT silently create broader live authority.

The runtime SHOULD retain:

- manifest path;
- file hash;
- imported revision;
- current drift status.

A Project MAY keep human guidance in another folder such as `schedules/`; only the canonical manifest is machine-discovered by default.

## 13.34 Manifest schema

Example:

```yaml
version: 1
timezone: Asia/Ho_Chi_Minh

automations:
  daily-ops:
    name: Daily Ads Ops
    enabled: true

    trigger:
      kind: cron
      expression: "0 9 * * *"
      timezone: Asia/Ho_Chi_Minh

    task:
      prompt_file: prompts/daily-ads-ops.md

    context: FRESH
    workspace: PROJECT_ROOT

    authority:
      profile: unattended-read-external-write-project

    overlap: SKIP_IF_RUNNING

    catch_up:
      policy: RUN_ONCE
      max_age: 6h

    limits:
      timeout: 60m
      max_retries: 2

    activation:
      require_manual_test: true
      review_first_runs: 5

    delivery:
      review: REVIEW_ALWAYS
      notify_on: [BLOCKED, FAILED]
```

Semantic capability names and authority profiles are resolved by policy, not invented by YAML.

## 13.35 Ads Agents acceptance fixture

`RunLumi/ads-agents` is the reference real-repository acceptance case for v1 Automations.

The repository separates:

```text
Skills   = how work is performed
Prompts  = current task entrypoints
Schedule = when it runs
Guardrails = authority
Session lifecycle = audit/closeout
```

Lumi MUST preserve this separation.

The Automation runner MUST NOT copy Ads methodology into the schedule definition.

Expected mapping:

| Automation | Task source | Trigger | Default delivery |
|---|---|---|---|
| Intraday health | `prompts/intraday-4h-health-check.md` | every 4h | silent on `NO_ALERT`, review/notify WATCH/INCIDENT/BLOCKED |
| Daily ops | `prompts/daily-ads-ops.md` | 09:00 Asia/Ho_Chi_Minh | review every run |
| Weekly review | `prompts/weekly-ads-review.md` | weekly | review every run |
| Monthly strategy | `prompts/monthly-strategy-review.md` | first weekday monthly | review every run |
| Pre-launch QA | `prompts/pre-launch-qa.md` | event/manual | review every run |
| Launch watch | `prompts/launch-watch.md` | launch event + offsets | notify only actionable |
| Post-change impact | `prompts/post-change-impact-review.md` | verified change + offsets | notify only actionable |
| Incident triage | `prompts/incident-triage.md` | incident event/manual | always review + notify |

Ads-specific requirements:

- scheduled/unattended runs MUST NOT gain Ads-write authority;
- repo report/worklog writes MAY be allowed;
- every top-level run MUST execute the repo's session lifecycle;
- `NO_ALERT` still persists session report/worklog and successful Automation history;
- missing browser login produces a blocked semantic outcome, not fake success;
- repeated identical login/auth blocks SHOULD auto-pause;
- 4h run MUST NOT become a budget/pause optimization loop;
- launch/post-change checkpoints MUST use event follow-up offsets rather than sleeping one long Task;
- authenticated Meta browser session remains bound to the owning ExecutionEnvironment;
- two overlapping Ads runs SHOULD NOT control the same browser/account session concurrently.

## 13.36 Required v1 UI

Desktop SHOULD provide an **Automations** surface with:

### List

- name;
- Project;
- enabled/paused/needs-attention state;
- schedule in human language;
- next run;
- last run;
- last semantic outcome;
- review/alert indicator.

### Detail

- trigger;
- prompt/Skill/Workflow;
- Project/workspace;
- authority summary;
- model preference;
- run limits;
- delivery policy;
- recent history;
- source revision/hash;
- Run Now;
- Pause/Resume;
- Edit;
- Delete/Archive.

### Review Queue

- actionable/completed Automation results;
- approvals;
- blocked runs;
- failed runs;
- Continue Interactively.

The UI SHOULD show “Every 4 hours”, “Daily at 09:00”, and “First weekday monthly” instead of raw cron/RRULE by default.

Advanced users MAY inspect/edit the raw recurrence representation.

## 13.37 Observability

Per Automation track:

- scheduled occurrences;
- admitted runs;
- skipped/deduplicated occurrences;
- success/no-action;
- blocked;
- failed;
- cancelled;
- average runtime;
- model/tool cost;
- retries;
- auto-pauses;
- human review/approval time;
- delivery success;
- useful/actionable outcome rate.

A high-frequency Automation that mostly produces noise is a product problem.

## 13.38 Security

V1 MUST protect against:

- schedule-created authority escalation;
- malicious prompt/Skill changes;
- untrusted event payload prompt injection;
- webhook spoofing;
- replayed webhook/event;
- path escape from Project;
- stale approval reuse;
- browser/session cross-account confusion;
- duplicate external side effects;
- retry after ambiguous effect;
- notification/webhook exfiltration;
- silent cloud failover;
- unattended shell/network widening.

Event payloads SHOULD carry references/minimized content rather than unnecessary sensitive bodies.

Webhook/event authentication MUST be explicit.

## 13.39 Required tests

V1 MUST test:

- one-shot AT run;
- interval recurrence;
- cron timezone;
- RRULE recurrence;
- DST edge;
- duplicate event;
- event follow-up offsets;
- disabled/paused automation;
- manual run while paused does not enable schedule;
- Project missing/unavailable;
- local-only device offline;
- workspace mode preserved across restart;
- prompt/source hash recorded;
- prompt change cannot widen authority;
- lease revoked before run;
- lease revoked mid-run;
- overlap SKIP_IF_RUNNING;
- overlap QUEUE_ONE;
- missed RUN_ONCE;
- stale missed run skipped by max_age;
- transient retry/backoff;
- ambiguous side effect is not blind-retried;
- repeated failure auto-pauses;
- repeated blocked-login auto-pause policy;
- WAITING_APPROVAL parks with no side effect;
- NO_ALERT recorded as successful and silent;
- delivery failure recorded separately from execution;
- Review Queue item opens exact run;
- continue-interactively preserves Project/run provenance;
- restart preserves Automations + next-run calculation + history;
- manifest import cannot silently broaden authority;
- Ads Agents fixture: 4h read-only run writes closeout artifacts and performs zero Ads writes.

## 13.40 V1 non-goals

V1 does not require:

- distributed cloud cron fleet;
- arbitrary user-authored scheduler code with unrestricted exec;
- sub-minute high-frequency trading/monitoring;
- unbounded event streams;
- public Automation marketplace;
- full visual DAG builder;
- self-modifying schedules without policy bounds;
- automatic expansion from read-only to write authority;
- perfect holiday calendars for every country;
- long-lived model turns sleeping between follow-up checkpoints.

## 13.41 Definition of Done

Automations v1 is ready when Lumi can demonstrate:

1. create/import an Automation in a Project;
2. manually test it;
3. enable it;
4. survive app/runtime restart without losing schedule/history;
5. run at the intended timezone;
6. create a real durable Task/Run;
7. execute in the configured Project/workspace;
8. enforce bounded unattended authority;
9. wait for approval rather than bypass it;
10. persist exact source/config/evidence provenance;
11. apply dedup/overlap/catch-up/retry safely;
12. put the result in History/Review Queue according to policy;
13. auto-pause a repeatedly broken job;
14. open the run and continue interactively;
15. run the `ads-agents` intraday + daily acceptance cases with mandatory report/worklog closeout and zero unauthorized Ads writes.

The product standard is:

> Schedule the work, not the risk.

A user should be able to trust that Lumi will return when it should, do only what it was authorized to do, leave evidence of every run, stay quiet when nothing matters, and surface the exact moments that need human judgment.
