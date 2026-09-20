# Automations research — 2026-09

Status: supporting research, non-normative  
Normative contract: `docs/specs/v1/13-scheduler-background-triggers.md`

## Why this exists

Lumi already had a safe scheduler core, but the product contract was too small for a
real Project-native automation experience. This note captures the useful patterns
reviewed before expanding Spec 13.

## Codex / ChatGPT scheduled work

Primary references:

- https://openai.com/index/introducing-the-codex-app/
- https://developers.openai.com/docs/automations
- https://developers.openai.com/docs/environments/git-worktrees

Useful patterns:

- Automations combine instructions with optional Skills and a schedule.
- Results land in a review surface instead of disappearing into background logs.
- Local scheduled work can bind to a local Project.
- Git Projects may run against the live checkout or an isolated worktree.
- A scheduled job can start fresh or deliberately return to an existing context.
- Teams are encouraged to test the prompt/workflow manually before scheduling.
- Worktrees isolate background mutations from the user's current local edits.

What Lumi adopts:

- Project-native scheduling;
- prompt/Skill references;
- worktree option;
- manual test before enable;
- Review Queue / continue-interactively handoff.

What Lumi does differently:

- explicit authority lease and current-policy intersection on every run;
- first-class semantic outcomes such as NO_ALERT / NO_ACTION;
- stronger separation between run success and delivery success;
- repo-native manifest with import/review rather than automatic authority.

## OpenClaw

Primary references:

- https://github.com/openclaw/openclaw/blob/main/docs/automation/cron-jobs.md
- https://github.com/openclaw/openclaw/blob/main/docs/automation/index.md
- https://github.com/openclaw/openclaw/blob/main/docs/automation/taskflow.md

Useful patterns:

- scheduler state and run history are durable;
- one-shot, interval, cron and event/condition-trigger styles;
- isolated and persistent execution contexts;
- bounded tool policy per job;
- manual run and run-history inspection;
- retries/backoff and auto-disable after repeated failures;
- dynamic cadence for monitors;
- delivery can be chat, webhook or silent;
- condition watchers persist tiny state and fire only on meaningful change;
- event-trigger scripts are treated as unattended execution and therefore high trust.

What Lumi adopts:

- durable Automation + separate Run history;
- explicit context mode;
- bounded retries/backoff;
- auto-pause;
- condition gates;
- delivery separate from execution;
- stable occurrence/idempotency identities.

What Lumi intentionally does not copy into v1:

- unrestricted headless operator shell jobs;
- long-lived stream-command triggers;
- broad gateway administration surface;
- sub-minute general-purpose cron;
- background scripts as a shortcut around normal Lumi policy/orchestrator gates.

## Ads Agents as acceptance case

`RunLumi/ads-agents` is unusually useful because it already separates:

```text
Skill = how
Prompt = what
Schedule = when
Guardrails = authority
Session lifecycle = evidence/closeout
```

This catches several scheduler design mistakes quickly:

- a 4h monitor must not become an optimization loop;
- NO_ALERT must be a successful silent run, not “nothing happened”;
- scheduled read-only Ads work may still write reports/worklogs to the Project;
- launch and post-change monitoring need offset follow-ups, not one sleeping task;
- browser login/device availability are real runtime dependencies;
- repeated login blocks should stop wasting runs;
- run overlap against one authenticated Ads session must be prevented;
- every run must leave auditable closeout artifacts.

The `.lumi/automations.yaml` manifest in that repository is the target
repo-native fixture for Spec 13.
