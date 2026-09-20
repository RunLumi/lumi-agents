# 25 — V1 Implementation Order & Dependency Graph

Status: Normative sequencing guidance

## 25.1 Goal

Prevent teams/agents from building high-level product surfaces before the trust/execution contracts underneath are stable.

## 25.2 Critical path

Recommended implementation order:

1. Spec 01 — core domain model
2. Spec 03 — action/observation protocol
3. Spec 04 — policy/approval/capabilities
4. Spec 11 — audit/evidence/verification
5. Spec 18 — errors/retry/recovery
6. Spec 02 — durable task/run state
7. Spec 05 — execution router
8. Spec 06 — browser executor
9. Spec 07 — native desktop executor
10. Spec 09 — provider routing
11. Spec 16 — eval/observability/economics
12. Spec 12 — workflow packs

This sequence forms the first production vertical slice.

## 25.3 Work-mode and product layer

After the trust/execution vertical slice is reliable:

13. Spec 08 — files/shell/artifacts
14. Spec 26 — Project workspace / Folder-as-Project
15. Spec 10 — context/memory with Project scoping
16. Spec 13 — Project-native Automations / scheduler / run history
17. Spec 14 — MCP/connectors/extensions, with Spec 24 Skill contracts
18. Spec 29 — protected project connections / broker scoping / safe initialization
19. Spec 28 — project Skill/Plugin install, activation and dependency snapshots
20. Spec 15 — desktop app/device distribution with Project entry points
21. Specs 23 and 27 — shared global/project Automations management and review UX
22. Spec 30 — Document Workspace: isolated preview, bounded edits and safe save

Spec 26 is intentionally placed immediately after filesystem/shell primitives.
Specs 28–29 reuse Spec 14/17/24 boundaries; they are not a second extension runtime
or secret store. Spec 27 uses Spec 13 identities and admission, not another scheduler.
Spec 30 reuses project/file/artifact services and can start after those boundaries
are ready; it does not require a public plugin marketplace or Office server.

Do not build "Open Folder" as a UI-only feature before Project identity, root boundaries, task binding, conflict handling, and change-set semantics exist.

## 25.4 Enterprise/product hardening

Then:

23. Spec 17 — full security/privacy hardening throughout implementation
24. Spec 19 — control plane/sync
25. Spec 20 — versioning/migrations
26. Spec 21 — release certification automation
27. Spec 24 — remaining bounded subagent behavior
28. Spec 22 — v1 final acceptance, including 22.8 Automations, 22.14 project extensions/connections and 22.15 Document Workspace

Spec 17 security is cross-cutting and MUST be applied continuously; its position here describes broader product hardening, not permission to defer security basics.

## 25.5 Dependency diagram

```text
01 Domain
  ↓
03 Action/Observation
  ↓
04 Policy/Approval
  ↓
11 Audit/Verification
  ↓
18 Retry/Recovery
  ↓
02 Durable State
  ↓
05 Router
  ├─→ 06 Browser
  ├─→ 07 Native
  └─→ 09 Models
          ↓
16 Evals/Economics
          ↓
12 Workflow Packs
          ↓
08 Files/Shell/Artifacts
          ↓
26 Project Workspace
   ├─→ 10 Context/Memory
   ├─→ 13 Scheduler/Automations
   ├─→ 30 Document Workspace (also 08/17/23/29 boundaries)
   └─→ 14 Extensions + 24 Skills + 17 Secret/policy boundaries
                         ↓
                 29 Project Connections
                         ↓
                 28 Project Skills/Plugins
                         ↓
       15 Desktop + 23 Handoff + 27 Global/Project Automations
                         ↓
17 Security hardening
19 Control Plane
20 Versioning
21 Certification
24 Remaining Subagents
          ↓
22 V1 Done (including 22.8, 22.14 and 22.15)
```

## 25.6 First seven-day slice

Day 1–2:
- finalize Rust types for 01/03;
- serialization fixtures;
- canonical risk/error enums.

Day 2–3:
- policy evaluator 04;
- approval digest;
- fail-closed tests.

Day 3–4:
- audit event + verifier 11;
- false-success test.

Day 4–5:
- browser worker skeleton 06;
- deterministic fixture site.

Day 5–6:
- Cua adapter skeleton 07;
- deterministic macOS/Windows fixture.

Day 6–7:
- provider contract 09;
- one Tier-1 adapter;
- one end-to-end workflow measured through 16.

This historical slice remains useful as the dependency rationale even after those layers are implemented.

## 25.7 Current Project-mode implementation slice

Once specs 01–23 foundations exist, implement spec 26 through one end-to-end Work-mode Project slice rather than many disconnected abstractions.

Recommended order:

1. durable `Project` type + store;
2. Open Folder / Open Recent backend contract;
3. canonical authorized-root policy binding;
4. Project-bound Task/Run identity;
5. safe file search/read/create/edit/move/delete using existing workspace primitives;
6. stale-write and external-change detection;
7. Lumi change-set tracking distinct from pre-existing changes;
8. Git read/status/diff;
9. bounded local Git branch/commit capability;
10. Project-local shell cwd + validation command execution;
11. Project discovery + instruction provenance;
12. Project-scoped retrieval/memory;
13. desktop Project home + Files/Changes/Tasks/Artifacts/Evidence surfaces;
14. clone repository;
15. remote supervision/resource identifiers;
16. capability negotiation and downgrade behavior.

Do not start with semantic indexing, language servers, or multi-agent coding.

First prove that a user can safely open an important real repository, ask for a bounded multi-file change, inspect the diff, run validation, stop/restart Lumi, and resume without losing scope or overwriting unrelated work.

## 25.8 Project-mode acceptance slice

Before calling Folder-as-Project implemented, demonstrate on deterministic fixtures and at least one real internal repository:

- open existing dirty repository;
- detect pre-existing edits;
- ask Lumi for a multi-file task;
- change only authorized files;
- run project validation;
- present Lumi-only change set;
- preserve unrelated user edits;
- restart the desktop/runtime;
- reopen Project;
- resume Task;
- make no filesystem access outside authorized roots;
- refuse malicious repository instructions that attempt to widen authority.

## 25.9 Automations implementation slice

After durable Project/Task/Run state is real, implement Automations as a thin durable
task-creation layer rather than a second orchestrator.

Recommended order:

1. Automation domain record + revision;
2. persisted definitions + next-run calculation;
3. AT / INTERVAL / CRON / RRULE;
4. timezone/DST tests;
5. Project + ExecutionEnvironment binding;
6. task source resolution + source hashes;
7. unattended authority lease/current-policy intersection;
8. durable AutomationRun history;
9. dedup + per-automation overlap + shared browser/resource arbitration (Spec 27);
10. missed-run policy;
11. retry/backoff + auto-pause;
12. semantic outcome contract;
13. History + Review Queue;
14. manual Run Now / test mode;
15. EVENT + follow-up offsets;
16. repo-native `.lumi/automations.yaml` import/diff;
17. global and project-filtered list/editor over shared records, per-user read state (Spec 27);
18. dependency snapshot and connection readiness checks (Specs 28–29);
19. Ads Agents intraday/daily fixture;
20. launch-watch/post-change event-offset fixture.

Do not begin with a visual DAG editor or arbitrary scheduler scripts.

The acceptance spine is:

```text
Import Ads Agents automation
→ Run Test
→ Enable
→ appears once globally and in its Project
→ restart Lumi
→ due occurrence creates durable Task/Run
→ approved dependency snapshot and project connection ready
→ correct Project/browser environment with exclusive conflicting control
→ externally read-only authority
→ report/worklog persisted
→ NO_ALERT stays silent but visible in History
→ actionable result enters Review Queue
```

## 25.10 Rule

Do not start a new architectural layer because the roadmap lists it.

Start it when the previous layer's contract tests and vertical-slice evidence make it necessary.

For Project mode, optimize for the shortest path from **Open Folder** to **verified useful change**, not for IDE feature breadth.

## 25.11 Project extension and connection implementation slice

Track the bounded slice in #76; integrate its readiness checks with Automations #73.

1. Reuse project/state/policy/secret contracts; add registered-project and authenticated
   caller/account/destination authorization at the broker boundary.
2. Add safe initialization and effective Git/tracked-state checks, including local
   `.lumi/project.json` exclusion and explicit foreign-marker recovery under Spec 29.
3. Prove metadata-only native/compatibility Skill discovery and unambiguous naming.
4. Install one versioned local/first-party bundle with separate enable/connect states,
   exact content locks and immutable active-run snapshots.
5. Add one enforceably sandboxed or brokered integration; refuse unsupported isolation.
6. Wire project Connections/Skills/Plugins UX, update/disable/revoke dependencies,
   and cross-project isolation into the shared automation editor/admission path.
7. Run Specs 28–29 adversarial fixtures and supported-platform canaries; record
   evidence for Spec 22.14 separately from existing component tests.

No public marketplace or custom credential vault is prerequisite. Existing Keychain
plumbing and a configuration file do not pass the new isolation/activation gates.

## 25.12 Document Workspace implementation slice

Track [#82](https://github.com/RunLumi/lumi-agents/issues/82) and
[Spec 30](30-document-workspace-preview-edit.md). Library choices and their
qualification limits are in the [dated research](../../research/document-workspace-oss-2026-09-20.md).

1. Reconcile the actual frontend first: the reviewed Tauri shell is vanilla JS.
   Add a minimal React/TypeScript/Vite document entry without an unrelated shell rewrite.
2. Implement authorized document sessions, bounded binary transfer and unprivileged
   rendering; prove custom IPC/network/secret denial before loading hostile Office HTML.
3. Add host-mediated staged save, source-version conflicts, recovery and exact
   artifact provenance. Keep these independent of any particular renderer library.
4. Deliver text/Markdown, CSV, raster images and PDF with lazy adapters and accessible controls.
5. Add best-effort DOCX preview and bounded XLSX data/basic-edit profile; preserve
   source-cell identity and mark formula caches stale rather than invent recalculation.
6. Time-box native DOCX/PPTX edit candidate tests: exact/transitive licenses,
   offline behavior, no-op/one-edit preservation corpus, native WebViews and resource cost.
7. Integrate only qualified operations; keep unsupported originals safe and offer
   explicit external editing or labeled simplified copies. Do not replace a failed
   qualification with a general homemade Office engine.
8. Complete Files/Artifacts/Review Queue entrypoints, undo/draft recovery, Vietnamese
   IME, native accessibility and measured budgets under Spec 22.15.

No macro engine, realtime collaboration, full spreadsheet calculation, mandatory
LibreOffice bundle or remote Office server blocks the first useful slice. A
preview-only intermediate release must not be called all-format basic editing.


## 25.13 Strategy convergence slice

[Spec 31](31-delegated-work-product-contract.md) is a cross-cutting acceptance
spine, not a new architectural layer. Implement it by closing the shortest path
through existing components.

### Now: prove trustworthy delegation

1. Close durable-intent/replay and executor/artifact/approval defects.
2. Make Folder-as-Project work through the shipped desktop path on a dirty real repository.
3. Wire real model-driven Task planning/execution to persisted Task/Run state.
4. Make Lumi-only Changes, validation and required evidence visible from real state.
5. Support at least two concurrent Tasks with isolated workspaces or deterministic conflicts.
6. Make Review Queue derive from persisted outcomes and separate routine from actionable attention.
7. Complete the Project-native Automation vertical slice and Task → disabled Automation draft conversion.
8. Integrate Project Connections/Skills/Plugins at actual admission/execution boundaries.
9. Record verified success, review/rescue/rework minutes and runtime cost.
10. Turn every reproducible escaped failure into a regression case.

Do not put this slice behind a marketplace, broad enterprise control plane,
multi-agent organization layer, full mobile runtime, full IDE features, or broad
native-computer-use work.

### Next: make delegation retain and spread

Only after the Now gate is credible:

1. remote web/mobile supervision over environment-owned work;
2. GitHub plus the smallest high-value team/business connectors;
3. team review routing and shared/private extensions;
4. private/cloud runner where a real workflow requires it;
5. MCP compatibility and an ACP interoperability experiment;
6. qualified Document Workspace expansion;
7. basic team policy/budget/admin surfaces;
8. paid design-partner workflow evidence.

### Later: scale proven value

Full enterprise governance, curated/private marketplace, broader Role Pack
catalog, multi-project agents and richer mobile execution are conditional on
retained use, safe extension execution, repeatable paid value and deployment
reuse. Public marketplace and arbitrary agent organizations remain deferred.

### Evidence thresholds

Before broadening beyond Now, target:

- >=50 substantial real Project Tasks on one release build;
- >=95% verified completion on the certified internal matrix;
- <5% unexpected rescue on the hardened routine slice;
- 0 unauthorized side effects, root escapes, lost user edits or crash duplicates;
- >=100 representative Automation occurrences;
- 100% of reproducible escaped failures captured as regressions.

If approximately 100 representative runs still show >15% unexpected rescue,
material ambiguity, linear attention growth, or poor fully loaded economics,
narrow the workflow or product surface before adding features.
