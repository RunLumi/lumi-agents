# Execution plan: trustworthy delegated digital work

Updated 2026-09-20. Strategic direction comes from [STRATEGY.md](../STRATEGY.md).
Implementation truth remains [implementation-truth.md](implementation-truth.md)
and release evidence remains [v1-readiness.md](v1-readiness.md). Normative
behavior lives in [specs/v1](specs/v1/00-v1-index.md), especially
[Spec 31](specs/v1/31-delegated-work-product-contract.md).

A strategy document is not evidence that a feature exists. This plan converts
the strategy into the shortest falsifiable execution path.

## Product contract

The near-term product must make this loop boring:

```text
Open real Project
→ delegate meaningful outcome
→ Lumi works in the authorized environment
→ inspect what changed
→ see independent proof
→ spend judgment only where needed
→ safely automate the repeated case
```

The expansion path is:

```text
Work → repeated Task → Automation → hardened Workflow Pack
→ bounded operational responsibility
```

The invariant remains:

```text
Models propose → policy authorizes → executors act
→ verifiers determine outcome → evidence records → economics measures value
```

Project is context/workspace, not authority. Scheduling is not authority. A
remote client is not the owner of local credentials. A successful model message
is not proof of successful work.

## The next 90 days: evidence, not feature completion

The highest-value uncertainty is whether Lumi can displace meaningful work while
reducing, rather than relocating, human attention. The next phase therefore
optimizes for real persisted end-to-end evidence.

| Order | Work | Exit condition | Tracking |
|---|---|---|---|
| P0 | Repair durable-intent/replay defects | Failed storage never dispatches; uncertain/applied effects never replay blindly | #43 |
| P0 | Close executor, artifact, approval and evidence boundary gaps | Real worker protocol verified; artifact escapes refused; sensitivity/device state/required evidence enforced | #44 |
| P0 | Reconcile implementation truth | Every readiness claim names actual proof layer and remaining gap; fixture/UI claims are not promoted to runtime truth | readiness |
| P0 | Real model-driven Folder-as-Project loop | Dirty real repo → meaningful multi-file Task → validation → Lumi-only changes/evidence → restart/reopen/resume; zero root escape or user-edit loss | #50 |
| P0 | Parallel delegated Tasks | At least two concurrent Tasks isolate writes/workspaces or surface deterministic conflicts; shared external resources serialize safely | Spec 31 / #50 |
| P0 | Evidence-first Review Queue | Persisted routine outcomes stay in History; only approvals/decisions/blockers/incidents/verification failures demand attention; read state grants no authority | Specs 23/27/31 |
| P0 | Project-native Automations | Durable Project-bound schedule/event → real Task/Run; lease/dedup/retry/history; Ads acceptance slice with zero unauthorized Ads writes | #73 |
| P0 | Task → Automation conversion | A useful completed Task creates a disabled draft; no approvals, credentials, leases, sessions or stale authority copied; test before Enable | #73 / Spec 31 |
| P0 | Project extensions + Connections | Pinned Skill/Plugin snapshots and protected project connection requirements are enforced at admission/execution, not just displayed | #76 |
| P0 | Verified-work economics | Persist verified completion, review/rescue/rework minutes, runtime cost and cost per verified success by Project/Task/Automation | Specs 16/31 |
| P0 | Release-path dogfood | The exact signed/canary candidate uses real persisted desktop state, not sample Work/Evidence/Permissions data | #10/#11 |
| P0 | Market evidence | 10–30 tightly relevant users/design partners; observe real work, baseline time/review burden and willingness to repeat/pay | founder-led |
| P1 | Remote supervision | Web/mobile can inspect, approve/reject, answer, stop, redirect and resume environment-owned work without acquiring local credentials | Spec 19/31 |
| P1 | Smallest real connector/browser bridge | One permitted real-system action/read produces independently verified result on the correct account/target | #40 |
| P1 | Qualified Document Workspace slice | Only formats/operations passing Spec 30 preservation/security gates ship; external editor remains escape hatch | #82 |
| P1 | Interoperability experiment | MCP works on a real useful integration; ACP feasibility is tested without reimplementing provider runtimes | Spec 14 |
| P1 | Canary distribution | Artifact-bound signing/update/rollback, emergency stop and onboarding are proven on actual builds | #11 |

Native/CUA expansion is conditional on a real workflow requiring it. Do not put
API pilots behind broad native automation, a cloud control plane, or Office-suite
breadth.

## Market-proof gates

Do not graduate from the first phase because the checklist looks complete. Use
the following decision thresholds on one exact release build:

| Metric | Threshold |
|---|---:|
| Substantial real internal Project Tasks | >=50 |
| Verified completion on declared certified internal matrix | >=95% |
| Unexpected rescue on hardened routine slice | <5% |
| Project-root escapes | 0 |
| Pre-existing-user-change loss | 0 |
| Unauthorized side effects | 0 |
| Crash-induced duplicate consequential effects | 0 |
| Representative Automation occurrences | >=100 |
| Reproducible escaped failures added to regression suite | 100% |
| Review/rework time | measured separately; target <20% of displaced manual time before claiming strong displacement |

These are decision thresholds, not customer guarantees. Failed and ambiguous
runs remain in the denominator.

## Attention is a first-class resource

A product that runs 10x more agent jobs but creates 10x more review work has not
won. Track separately:

```text
expected approval minutes
review minutes
unexpected rescue minutes
rework minutes
displaced baseline manual minutes
```

Headline product metrics:

```text
verified work completed
human minutes avoided
human attention minutes
exceptions
cycle time
cost per verified success
unauthorized effects
automation outcomes
```

Tokens, messages, tool calls and agent-hours are diagnostics, not proof of value.

## Work-mode acceptance spine

The shortest path to product truth is:

```text
Open important dirty Project
→ model plans a bounded Task
→ Task gets its own durable Run/workspace
→ files/shell/Git/browser/connector act only within capability
→ validation and postconditions run
→ Lumi-only Changes + Evidence are persisted
→ routine success is quiet / actionable failure enters Review Queue
→ stop or restart
→ reopen and resume correctly
→ optionally create a disabled Automation draft
```

Dogfood substantial work, not toy prompts. Every escaped reproducible failure
becomes a regression fixture.

Parallel work is required for table-stakes delegation, but it must be boring:
Git worktrees or equivalent isolation where practical, explicit write/resource
admission elsewhere, and no last-writer-wins corruption.

## Automation acceptance spine

Automations are a durable Task-creation layer, not a second orchestrator.

```text
Create/import Project Automation
→ preview schedule/source/dependencies/authority
→ Run Test
→ Enable
→ restart Lumi
→ due occurrence creates durable Task/Run
→ current policy + bounded lease + pinned dependencies admitted
→ execute
→ verify semantic outcome
→ routine result stays quiet in History
→ actionable result enters Review Queue
→ repeated equivalent failure auto-pauses
```

Use `RunLumi/ads-agents` as the acceptance repository for intraday/daily cases
where appropriate. Scheduled Ads work remains externally read-only until a
separately authorized write case is intentionally proven.

## Human authority and recovery

An approval command issues approval for one pending normalized action and trusted
principal. The executor gate validates and consumes it immediately before
dispatch. UI read/dismiss/acknowledge state never consumes approval and never
pretends execution occurred.

Read/report/draft work is the preferred early operational boundary. Payments,
transfers, ad-spend changes, legal commitments and consequential personnel
decisions remain outside initial autonomous scope.

A crash never authorizes retry. An ambiguous external effect is re-observed and
reconciled before any retry. Foreground human takeover pauses conflicting
automation and forces re-observation before resuming.

## Product-market phase, only after market-proof gates

Once the core loop is credible, invest in the retention and team surfaces that
make trustworthy delegation spread:

1. polished Project/Task/Changes/Evidence/Review Queue UX backed by real state;
2. remote web/mobile supervision;
3. Skills/Plugins + protected Connections;
4. MCP and a bounded ACP interoperability path;
5. GitHub plus 2–3 connectors selected from observed workflows;
6. private/cloud runner only where local availability blocks real work;
7. team sharing, review routing and basic policy/budget controls;
8. qualified Document Workspace expansion;
9. paid design-partner pilots with measured baseline and residual work.

Commercial decision thresholds from STRATEGY.md should be treated as hypotheses
to test, not forecasts: qualified-user activation, week-four retention, second
meaningful Task, Automation adoption/retention, paid conversion, review/rework
burden, runtime economics and support burden.

## Operational-wedge phase

Do not build a broad Role Pack catalog. Choose one bounded responsibility only
after actual access, a willing supervisor, baseline measurements and a buying
reason exist.

Finance Ops, Ads Ops, IT Ops and other candidates are hypotheses. Existing packs
are implementation leverage, not demand evidence.

For the chosen responsibility:

```text
observe real queue
→ baseline human work
→ use Work mode
→ identify repeated sequence
→ Automation
→ harden Workflow Pack
→ run >=100 representative units
→ measure rescue/economics
→ compose minimal Role Pack
→ sustain live operation
→ test independent deployment reuse
```

Keep the existing mature-role targets unless evidence justifies changing them:
approximately >=99% verified completion on the certified routine matrix, zero
unauthorized effects, >=80% baseline routine human minutes removed, >=80%
eligible routine units without unexpected rescue, cost <=30% of displaced loaded
labor value after support, and at least four measured production weeks.

## Explicitly defer

Until the market-proof gates are passed, do not prioritize:

- full IDE replacement or language-server breadth;
- public marketplace;
- dozens of providers;
- arbitrary multi-agent organizations;
- broad multi-project autonomous agents;
- full mobile editing/runtime parity;
- full enterprise control-plane completeness;
- generic assistant/chat polish;
- broad native platform automation without a workflow requirement;
- universal Office fidelity or a homegrown Office engine;
- large Role Pack catalog;
- deep fleet/MDM features beyond what the next real deployment requires.

Provider neutrality, capability contracts, local-only routing, revocation and
safe extension boundaries remain architectural requirements even when their
broad product surfaces are deferred.

## Stop / narrow conditions

Narrow, redesign, or stop expanding when:

- after approximately 100 representative runs, unexpected rescue remains >15%;
- users routinely redo Lumi output;
- users demo Lumi but return to another tool for meaningful work;
- Review Queue attention grows roughly with healthy Automation volume;
- users cannot explain why a Task is considered successful;
- permissions are repeatedly widened just to make workflows function;
- runtime/support cost consumes most measured labor value;
- a third independent deployment still requires substantial bespoke code;
- feature expansion outpaces retained meaningful use.

The cheapest response is usually a narrower workflow, stronger verifier,
better deterministic integration, or different wedge, not another feature layer.

## Weekly operating cadence

Every week review reality, not code volume:

1. real meaningful Tasks attempted and verified;
2. escaped failures converted to regression fixtures;
3. attention minutes and unexpected rescue;
4. Automation outcomes and Review Queue noise;
5. one customer/design-partner observation or deployment lesson;
6. current implementation-truth/readiness delta;
7. blockers requiring external access, credentials, signing authority or elapsed time.

Every blocker records the smallest external input needed and useful independent
work that can continue without pretending the blocker is solved.

## Delivery discipline

Use small reviewed PRs, exact-commit CI and failure regressions. Refresh readiness
when behavior changes. Documentation must drive an admission rule, evaluation,
pilot or decision.

The decisive question is not whether Lumi has the longest feature list. It is
whether a rational user can delegate consequential bounded work, return later,
understand exactly what happened and why it counts as successful, and choose to
let Lumi safely do the repeated case again.
