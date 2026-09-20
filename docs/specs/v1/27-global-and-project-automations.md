# 27 — Global & Project Automations v1

Status: Normative product contract; not an implementation claim.

Extends [13 — Automations](13-scheduler-background-triggers.md),
[23 — UX](23-user-experience-handoff.md), and
[26 — Projects](26-project-workspace-folder-as-project.md).
Installation and connection readiness follow [28](28-project-skills-plugins.md)
and [29](29-project-connections-secrets.md).

## 27.1 Outcome and scope

A user can find, create, configure, pause, test, and review automations without
opening each project. Inside a project, the same operations remain available
with that project selected.

**Two views, one Automation identity, one scheduler.** A global menu is a
management view, not global execution authority or a second automation store.

The supplied Scheduled tasks screenshot informs the interaction: search, simple
rows, Active/Paused/Completed filters, Create, and unread results. Lumi retains
its own DESIGN.md/ICON.md language and adds project, environment, and attention
context. Do not copy personal task names/content from the screenshot into fixtures.

## 27.2 Navigation

Required entry points:

```text
Application sidebar
  Automations                  /automations

Project
  Automations                  /projects/:project_id/automations

Shared detail                  /automations/:automation_id
Exact occurrence/run           /automations/:automation_id/runs/:run_id
```

These are route contracts, not claims that endpoints already exist.
Project navigation filters the global collection; it does not clone records.
Detail deep links preserve the originating filter/scroll position on Back.
The application entry remains available with no project currently open.

V1 execution is project-bound: creating globally requires choosing exactly one
registered Project and its eligible ExecutionEnvironment. A future projectless
reminder mode is not implied. A reusable template is not an executable automation.

## 27.3 Canonical ownership and visibility

Every automation has stable tenant, project, environment binding, owner, and
revision. Names and paths are display metadata, never identifiers or authority.

The global list MUST query registered automations the principal may view in the
current tenant. It MUST NOT scan the whole disk, load every project into model
context, or aggregate across tenants implicitly. Project names, counts, search
matches, unread counts, and error messages obey the same authorization filter.

A local-only release shows registered local automations. Cross-device metadata
is optional under Spec 19; unavailable sources are disclosed as partial/stale
coverage, never represented as a complete organization-wide inventory.

View, create/manage, run, approve consequential actions, install extensions,
and connect/revoke credentials are distinct authorized operations. Visibility
in this menu grants none of the other operations.

## 27.4 List anatomy

Default: a calm operating list, not a card dashboard.

```text
Automations                                          [ Create v ]
Scheduled work across your projects.
[ Search automations... ]     [ All projects ] [ All environments ]
All   Active   Paused   Completed      [ Needs attention ]
                                           Mark all results as read

Daily Ads Ops                         Needs connection       • unread
Ads Agents · Daily at 09:00 · Asia/Ho_Chi_Minh
Next: 21 Sep, 09:00 · This Mac           Last: Blocked login   ...
```

Each row exposes name, project, human schedule/timezone, next intended run,
environment availability, latest execution/business outcome, and unread/attention
state. Secondary detail can be disclosed at compact widths. Event-driven rows
say `On launch` or `Waiting for event`, not an invented next timestamp.

Search matches authorized names/project names/descriptions. Filters and sort
persist in navigation state. Default order: actionable attention first, then
next due time, then stable ID. Runs/History remain a separate view.

Empty states distinguish no automations, no filtered matches, unavailable
project, and no permission. Unknown/stale results are not rendered as healthy.

## 27.5 Filter semantics

Automation lifecycle and individual run result are different dimensions.

| Filter | Meaning |
| --- | --- |
| All | Non-archived automations, including drafts and attention states |
| Active | Enabled automations with an open recurrence/event subscription |
| Paused | Operator-paused or auto-paused, with the reason visible |
| Completed | One-shot/bounded schedules exhausted, with no pending or active occurrences |
| Needs attention | Orthogonal filter for review, missing dependencies, failed closeout, approval, or actionable failure |

A recurring automation remains Active after a successful run. An enabled event
subscription with no next timestamp is not Completed. Compute exhaustion from
its persisted end condition and occurrence ledger, not from a green last-run badge.
A finished schedule may still have an unsuccessful last outcome; show both.
Archived entries are available through an explicit filter, not destroyed.

## 27.6 Read state is not approval

Read/unread is per principal, not shared globally across users. Store a read
cursor against an automation's result sequence or individual review items.

`Mark all results as read` affects only authorized matching results through a
captured sequence watermark. Results arriving afterwards stay unread. A filtered
view makes the affected scope explicit; paginate on the server, not only the
visible rows. This operation MUST NOT approve, dismiss, resolve an incident,
change schedule state, delete evidence, or cancel a run.

Approval/incident attention remains visible even when the item is read.
Healthy NO_ALERT runs remain in History without creating unread noise unless
canary/review policy requires it.

## 27.7 Create and configure

Create offers `New automation`, `From project prompt`, and `Import manifest`.
All use the same editor and admission rules.

Show the essentials first:

1. Project (preselected and visible when opened from a project).
2. Name and task source: inline instructions or project prompt/Skill/Workflow.
3. When: once, interval, daily/weekly/monthly, or supported event.
4. Result handling: review every run or actionable results only.

Advanced disclosure: environment/workspace, context, model route, budgets,
catch-up, retries, overlap, expiry, and notification quiet hours.
Defaults come from the selected project's reviewed policy, never the previously
opened project's credentials or temporary permissions.

Preview before enabling MUST show the normalized recurrence, timezone, next five
time occurrences where calculable, timing window, project/environment, selected
source and extension versions, connection readiness, effective read/write scope,
allowed output paths, and destination of results. Event previews show the event
source/required fields and relative checkpoints instead.

`Save draft`, `Run test`, and `Enable` have different effects. Test runs use the
same execution-policy ceiling; clicking a button does not turn a read-only
scheduled Ads task into an interactive Ads writer. Material edits invalidate
stale tests/consents as applicable. Missing connections/required sandbox support
block enablement with a concrete remedy; the draft remains saveable.

## 27.8 Shared detail and commands

Detail has Overview, Runs, and Configuration, using existing page primitives.
Show dependencies and exact revision/source drift alongside schedule and policy.

Required commands: Run now, Run test, Pause, Resume, Edit, Duplicate to project,
Archive. Cancellation is offered separately for active work. Destructive history
purge is not bundled with Archive.

All mutations are authenticated, revision-checked, idempotent commands against
the same record from either view. Concurrent edits return a conflict/diff; they
do not silently overwrite. Acknowledgment means durable acceptance, not successful
execution. Clients re-query canonical state after reconnect.

`Run now` while paused does not resume future runs. Resume revalidates dependencies,
current authority and catch-up; it does not replay every missed occurrence.
Duplicate creates a disabled draft with a new ID. It copies portable instructions
and schedule intent only—not leases, approvals, read history, event subscriptions,
connection bindings, browser sessions, or secrets. Changing project/environment
requires equivalent rebinding and review, not a silent retarget.

## 27.9 Manifest synchronization

`.lumi/automations.yaml` is shareable desired configuration. The runtime owns
installed IDs/revisions, activation, leases, occurrences, read state, and history.

Identify imported entries by `(tenant, registered project, manifest entry key)`.
Repeated import updates the same definition or reports drift, never duplicates
schedules. File edits create a proposed revision, not implicit activation.

A UI edit changes the installed definition. For manifest-backed entries show
`Local changes not written to project` with an explicit `Write to project` action.
Use safe compare-and-swap file edits and preserve unrelated work. An unparseable
manifest is an error, not an empty manifest authorizing deletion. Removing a
manifest entry proposes retirement; it does not erase history or credentials.

## 27.10 Shared-resource concurrency

Spec 13 overlap policy prevents repeated instances of ONE automation. It is
insufficient when daily, weekly, incident, and launch-watch jobs share a browser.

The runtime MUST additionally admit work against protected resources such as:

- authenticated browser profile/session or foreground desktop input;
- connection/account mutation scope;
- shared report destination where writes could collide.

Derive resource identity from registered environment/connection bindings, not an
arbitrary model-supplied lock name. Serialize conflicting control across projects,
automations, manual tasks, and supported local clients. A read-only Ads workflow
can still conflict through browser navigation/account switching.

Contended runs remain durably waiting, subject to deadline/staleness and bounded
queue rules. Do not silently drop a unique launch checkpoint/incident because an
unrelated report is running. Incident priority applies at safe admission/checkpoint
boundaries, not by killing an unreconciled operation or stealing a stale lock.
Leases/fencing prevent an old worker continuing after reassignment. Foreground
human takeover pauses conflicting automation and requires re-observation.

## 27.11 Environment and dependency truth

Local runs require the owning runtime and necessary sessions to be available.
Closing a window, quitting the runtime, sleep, screen lock, and network loss are
separate states. Display the actual support matrix; do not promise an asleep Mac
will execute browser work or silently relocate it to a cloud worker.

Before each run, resolve the project's pinned extension set and connection bindings
under Specs 28–29. Missing/revoked dependencies block the run. Reading a previous
success or pressing Resume never supplies credentials or reinstates old authority.

## 27.12 Acceptance and implementation order

Build through existing scheduler, project, state, handoff, and desktop boundaries:
authorized aggregate query -> shared CRUD/editor -> filtered project view -> run
history/read cursor -> manifest diff -> dependency readiness -> resource arbitration.
Do not introduce a second scheduler, global super-project, or mandatory cloud fleet.

Required tests:

- Create in Project A appears exactly once globally and in A, never in B.
- Global creation requires explicit project; project creation uses the same API.
- Pause/edit from either view immediately reflects in the other; duplicate request
  IDs and simultaneous edits cannot duplicate work or clobber revisions.
- Cross-tenant/project denial applies to list, search, counts, detail and commands.
- Successful recurring run stays Active; exhausted bounded schedule is Completed;
  event subscription remains Active; outcome failures stay visible.
- Mark-all-read changes no approval/incident state; another principal's read cursor
  and results arriving after the watermark are unaffected.
- Duplicate/relink/reimport cannot inherit another project's credentials or lease.
- Offline project/daemon, missed runs, stale caches and unsupported recurrence have
  honest UI states; unsupported mandatory fields fail instead of being ignored.
- Daily + weekly + incident jobs cannot concurrently drive one authenticated
  session; independent authorized sessions may run in parallel.
- Revision/source/connection changes are rechecked at the real execution boundary.
- Keyboard/focus, small-window layout, Vietnamese labels and denied/empty/error
  states work without collapsing to unlabeled icons.

Use generic Ads Ops fixtures: 4h health plus daily report, required report/worklog
closeout, zero Ads writes, silent NO_ALERT history, and an actionable missing-login
case. Live private-repository testing requires access; public CI uses sanitized
local fixtures and never imports private accounts, tokens, or customer data.
