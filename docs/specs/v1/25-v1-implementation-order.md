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

## 25.3 Next layer

After the vertical slice is reliable:

13. Spec 08 — files/shell/artifacts
14. Spec 10 — context/memory
15. Spec 13 — scheduler/background
16. Spec 14 — MCP/connectors/extensions
17. Spec 15 — desktop app/device distribution
18. Spec 23 — user experience/handoff

## 25.4 Enterprise/product hardening

Then:

19. Spec 17 — full security/privacy hardening throughout implementation
20. Spec 19 — control plane/sync
21. Spec 20 — versioning/migrations
22. Spec 21 — release certification automation
23. Spec 24 — skills/subagents
24. Spec 22 — v1 final acceptance

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
10 Context/Memory
13 Scheduler
14 Extensions
15 Desktop
23 UX/Handoff
          ↓
19 Control Plane
20 Versioning
21 Certification
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

## 25.7 Rule

Do not start a new architectural layer because the roadmap lists it.

Start it when the previous layer's contract tests and vertical-slice evidence make it necessary.
