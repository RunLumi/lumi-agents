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
16. Spec 13 — scheduler/background
17. Spec 14 — MCP/connectors/extensions
18. Spec 15 — desktop app/device distribution with Project entry points
19. Spec 23 — user experience/handoff with Project home/changes UX

Spec 26 is intentionally placed immediately after filesystem/shell primitives.

Do not build "Open Folder" as a UI-only feature before Project identity, root boundaries, task binding, conflict handling, and change-set semantics exist.

## 25.4 Enterprise/product hardening

Then:

20. Spec 17 — full security/privacy hardening throughout implementation
21. Spec 19 — control plane/sync
22. Spec 20 — versioning/migrations
23. Spec 21 — release certification automation
24. Spec 24 — skills/subagents
25. Spec 22 — v1 final acceptance

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
   ├─→ 15 Desktop
   └─→ 23 UX/Handoff
13 Scheduler
14 Extensions
          ↓
17 Security hardening
19 Control Plane
20 Versioning
21 Certification
24 Skills/Subagents
          ↓
22 V1 Done
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

## 25.9 Rule

Do not start a new architectural layer because the roadmap lists it.

Start it when the previous layer's contract tests and vertical-slice evidence make it necessary.

For Project mode, optimize for the shortest path from **Open Folder** to **verified useful change**, not for IDE feature breadth.
