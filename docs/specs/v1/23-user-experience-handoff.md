# 23 — User Experience, Handoff & Control v1

Status: Normative product contract

## 23.1 Goal

Make delegation feel like working with a strong colleague, not babysitting a cursor.

For Work mode, the primary durable context is a Project as defined by spec 26.

## 23.2 Task creation

User MUST be able to express:

- goal;
- desired output;
- constraints;
- deadline optional;
- allowed systems/resources;
- approval preferences where permitted.

For a Project-bound task, task creation MUST identify the active `project_id` and owning ExecutionEnvironment.

System SHOULD show inferred plan at appropriate granularity.

The UI MUST NOT silently retarget a task because the user later navigates to another Project.

## 23.3 Progress

Task view SHOULD show:

- current phase;
- Project when applicable;
- completed milestones;
- active system/app;
- pending approvals;
- exceptions;
- artifacts;
- changed-files summary when applicable;
- validation status when applicable;
- time;
- cost/budget;
- evidence summary.

Do not stream noisy internal reasoning.

## 23.4 Approval UX

Approval MUST show business effect, not raw technical gesture.

Good:
- Send quote to customer@example.com for 12,500,000 VND.
- Push 3 local commits from project PrintUp to origin/feature/project-workspace.

Bad:
- Click button at (812, 477).
- Run git command.

## 23.5 Exception UX

Exception card SHOULD show:

- what blocked;
- why;
- safe choices;
- consequences;
- evidence;
- suggested next step.

For Project conflicts, the card SHOULD identify affected files/resources without implying unrelated pre-existing changes belong to Lumi.

## 23.6 Handoff

User MUST be able to:

- pause;
- take over;
- edit goal/constraints;
- approve/reject;
- supply missing information;
- resume;
- cancel.

Material goal change MAY require replanning/policy.

## 23.7 Takeover

When user takes over native/browser session:

- automation MUST pause conflicting input;
- state SHOULD remain observable;
- resume MUST re-observe state before continuing.

When the user edits a Project with another editor during takeover/pause, resume MUST revalidate relevant file/Git preconditions before applying queued mutations.

## 23.8 Completion

Final result SHOULD include:

- concise outcome;
- Project/change-set summary when applicable;
- artifact links;
- validations and their status;
- exceptions/unresolved items;
- evidence/provenance;
- actions taken;
- approvals used;
- relevant cost/time.

## 23.9 Trust language

UI MUST distinguish:

- planned;
- attempted;
- executed;
- verified;
- ambiguous.

Never collapse "attempted" into "done".

For Project work, "edited" MUST NOT imply "validated."

## 23.10 Background visibility

Background/unattended local execution SHOULD be visible through tray/status.

User MUST have local stop.

## 23.11 Privacy

Capture indicators SHOULD appear when screen/microphone capture active beyond ordinary selective evidence.

Project mode alone MUST NOT imply continuous screen capture or whole-project cloud upload.

## 23.12 Provider choice

Advanced user/admin MAY select allowed provider preference.

UI MUST communicate when policy forces local-only or specific provider restrictions.

Project-local/provider policy MUST survive model/provider switching.

## 23.13 Artifacts

Draft artifact and published/sent artifact MUST be visually distinct.

Project files modified in place SHOULD be distinguishable from separately generated artifacts.

## 23.14 Notifications

Notifications SHOULD be reserved for:

- approval;
- exception;
- completion;
- failure;
- material budget issue.

Avoid notification spam for routine steps.

## 23.15 Accessibility

Desktop app SHOULD meet accessible keyboard/navigation/contrast expectations appropriate to platform.

## 23.16 Tests

V1 UX tests SHOULD cover:

- approval clarity;
- pause/takeover/resume;
- ambiguous state;
- local stop;
- permission onboarding;
- background task visibility;
- artifact draft vs publish;
- failed verification shown honestly;
- Project identity visible for Project-bound tasks;
- agent change set distinct from pre-existing user changes.

## 23.17 Multi-surface supervision

Desktop, web, and future mobile clients are control surfaces over the same durable task/environment/Project contracts.

A remote client MUST NOT pretend to own:

- local filesystem;
- local credentials;
- browser/app sessions;
- native permissions;
- executor state;
- Project local roots.

Those belong to the ExecutionEnvironment.

Shared Project/task/approval/exception state SHOULD live behind common typed contracts rather than duplicated client logic.

## 23.18 Capability-aware UX

Clients MUST render actions based on the selected environment's advertised capabilities.

If a runtime does not support a feature:

- hide it;
- disable it with a clear reason;
- or route to an explicitly compatible fallback.

Do not display a control that will silently reinterpret into a weaker or riskier operation.

Project features such as clone, semantic index, Git remote write, or shell isolation MUST be capability-aware.

## 23.19 Remote supervision

Remote supervision SHOULD support:

- inspect Project/task progress;
- pause/cancel;
- steer;
- approve/reject;
- supply missing information;
- inspect artifacts/evidence;
- inspect approved change summaries;
- resume.

Remote transport authentication does not itself authorize every task/action operation.

Remote approval UI MUST show the same normalized business effect as local approval UI.

A remote client MUST refer to local Project resources through stable Project/root/resource identifiers, not through paths on the remote client's own filesystem.

## 23.20 Reconnect behavior

After client disconnect/reconnect:

- task truth comes from durable runtime state;
- client cache is not authoritative;
- current environment capabilities override stale cached capabilities;
- pending approval status is reloaded;
- Project identity/root health is revalidated for Project-bound work;
- a previously active native/browser surface is re-observed before takeover/resume;
- relevant Project files/Git state are revalidated before queued mutations resume.

## 23.21 Additional UX tests

V1 SHOULD additionally cover:

- same task supervised from desktop then web without state divergence;
- stale client capability after runtime downgrade;
- reconnect while WAITING_APPROVAL;
- remote cancel invalidates new action admission;
- remote client cannot reference its own local file path as environment-owned resource;
- local and remote approval render identical normalized business effect;
- remote Project supervision cannot widen local filesystem roots.

## 23.22 Project home

When a Project is opened, the product SHOULD provide a stable Project home rather than dropping the user into a transient chat.

Project home SHOULD make the following discoverable:

- Project name/root identity;
- current environment/device;
- active and recent tasks;
- files/search;
- changes;
- Git status/branch when applicable;
- validations;
- artifacts;
- approvals/exceptions;
- evidence;
- recent Project memory/context where appropriate.

The Project home is an operations surface over delegated work, not a requirement to implement a full IDE.

## 23.23 Open-folder flow

The minimum Open Folder flow is:

```text
Open Folder
  -> native folder picker
  -> canonicalize selected root
  -> create/reopen Project identity
  -> discover capabilities + project metadata
  -> show Project home
  -> user creates or resumes a Task
```

The UI SHOULD make the selected root and scope understandable.

It SHOULD NOT ask the user to approve every ordinary file read/write inside an already authorized Project root unless policy requires it.

It MUST ask when an action crosses the authorized Project boundary or another policy threshold.

## 23.24 Recent Projects

Open Recent MUST show durable Projects rather than raw path strings where possible.

Each entry SHOULD indicate:

- Project name;
- root availability;
- environment/device;
- last opened time;
- active/unresolved task indicator.

If the root is unavailable, selecting the Project MUST present a clear relink/recovery path rather than silently creating an empty folder.

## 23.25 Changes experience

For Project tasks that mutate files, the user SHOULD be able to inspect:

- files Lumi created;
- files Lumi modified;
- files Lumi moved/deleted;
- diff/patch summary;
- validation status;
- whether a change pre-existed Lumi;
- unresolved conflicts.

The product SHOULD optimize for "what changed and is it good?" rather than exposing raw low-level tool traces by default.

## 23.26 Git experience

When Git is present, the UI SHOULD expose enough information for safe delegation:

- branch;
- dirty/clean state;
- ahead/behind when available;
- Lumi-produced changes;
- pre-existing user changes;
- commit/push/PR state when those capabilities are enabled.

Push, force-push, remote branch deletion, and other external/destructive Git effects MUST render as business-level approvals where policy requires them.

## 23.27 External editor coexistence

Lumi MUST coexist with external editors/IDEs.

The UX SHOULD assume files can change outside Lumi.

When external changes conflict with a pending Lumi edit:

- pause conflicting mutation;
- show the conflict;
- preserve both sources when feasible;
- allow replan/rebase/reapply;
- do not silently overwrite the newer external change.

## 23.28 Project completion standard

For a Project task, completion should answer four questions clearly:

1. What did Lumi change?
2. What did Lumi validate?
3. What remains unresolved?
4. What, if anything, requires the user's next action?

A polished narrative without an inspectable change set and validation state is insufficient for substantial file/repository work.


## 23.29 Automations

Desktop SHOULD expose a first-class **Automations** surface.

The default list SHOULD answer:

- What will run?
- For which Project?
- When is the next run?
- Is it enabled, paused, or needs attention?
- What happened last time?
- Does anything need review?

Each Automation row SHOULD show:

- name;
- Project;
- human-readable schedule;
- next run;
- latest semantic outcome;
- state;
- review/attention indicator.

The Automation detail SHOULD expose:

- trigger and timezone;
- task source (prompt / Skill / Workflow);
- workspace mode;
- context mode;
- authority summary;
- model preference;
- overlap/catch-up/retry policy;
- delivery/review policy;
- recent runs;
- source revision/hash.

Primary controls:

- Run Now;
- Pause / Resume;
- Edit;
- Archive/Delete.

Users SHOULD see human schedule language by default:

```text
Every 4 hours
Daily at 09:00 · Asia/Ho_Chi_Minh
Every Monday at 10:00
First weekday monthly at 10:30
After launch: +1h, +4h, +24h, +48h, +72h
```

Raw cron/RRULE belongs behind an advanced disclosure.

## 23.30 Automation Review Queue

Automation results SHOULD not flood ordinary task/chat surfaces.

Review Queue SHOULD rank:

1. approval required;
2. incident / failure / blocked;
3. actionable recommendation;
4. completed informational result.

Healthy silent monitor outcomes such as `NO_ALERT` stay in History unless policy asks for review.

Each review item SHOULD show:

- Automation;
- semantic outcome;
- concise result;
- evidence/artifact links;
- Project;
- run time;
- required approval or next action;
- Continue Interactively.

Continuing interactively MUST preserve the exact run/Project/evidence provenance.

Opening a review item MUST NOT implicitly approve a pending action.

## 23.31 Automation creation

When creating from natural language, UI MUST show the structured definition before enabling:

```text
WHAT       prompts/daily-ads-ops.md
WHERE      Ads Agents · local project
WHEN       Daily at 09:00 · Asia/Ho_Chi_Minh
WORKSPACE  Project root
AUTHORITY  External read-only · Project report/worklog writes allowed
REVIEW     Every run
FAILURE    Retry twice · auto-pause after repeated failures
```

“Enable” authorizes only the displayed bounded definition.

The user SHOULD be encouraged to Run Test before enabling.
