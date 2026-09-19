# Implementation truth map

Baseline: remote `main` `96fa46c4dba7aa70e86830232e2ba29e371b8d84`, reviewed 2026-09-19. Scope: root guidance, roadmap/readiness/plan, architecture/security, v1 specs and ADRs, all packs/crates, desktop/worker, CI, both open issues and recent merged PRs. Subsequent PRs must update affected rows with new evidence.

“Production-relevant” means useful implementation, not production certification. Several rows have more than one category.

| Area and primary source | Classification | Evidence and limit |
|---|---|---|
| `lumi-protocol`, `lumi-policy` | implemented and production-relevant | Canonical proposals/digests, risk/capability checks, scoped expiring approvals. Sensitivity is omitted from material digest; environment/generation binding and trusted device context need integration. |
| `lumi-state` | implemented and production-relevant; partially implemented | JSON snapshots, journal/checkpoint/resume helpers. Host integration must handle storage failures and migrations. |
| `lumi-orchestrator` | partially implemented | Policy/router/approval/verifier pipeline. Baseline discards persistence failures and ignores failed journal restore; direct replay needs admission protection. #43. |
| `lumi-audit` | implemented and production-relevant; not wired end-to-end | Hash-chained log, evidence and verifier. Orchestrator audit/evidence remain in memory unless persisted by a host; effect truth needs an independent environment. |
| `lumi-models` | implemented but fixture-only for provider evidence | Four adapter families, transport and routing. No live workflow proof or measured cross-provider economics. |
| `lumi-browser`, `workers/playwright` | implemented but not wired end-to-end | Semantic worker/IPC; root tests use a fake worker. Rust flattens operation fields while the real JS worker reads request.params, a live-path mismatch. Authenticated execution not evidenced. |
| `lumi-native` | implemented but fixture-only; partially implemented | DesktopDriver/session/oracle contract and platform fixtures. Cua adapter deliberately refuses before mutation; wire execution missing. |
| `lumi-workspaces` | implemented and production-relevant | Scoped file controls and bounded host shell. ArtifactStore path inputs need validation; declared shell network policy is not isolation. Sandbox/container demands are refused. |
| `lumi-secrets` | implemented and production-relevant | Broker and OS Keychain; isolated macOS test passes with host access. Pilot credentials not evidenced. |
| `lumi-memory` | implemented but not wired end-to-end | Context, memory policy and retrieval primitives; no production environment integration established. |
| `lumi-agent` | implemented but fixture-only for Work-mode proof | Bounded planning/tool loop; no live ownership of a business queue. |
| `lumi-scheduler` | implemented but not wired end-to-end | Normalization/dedup/quiet-hours/lease helpers and gated fixtures; no durable continuously operating queue service. |
| `lumi-connectors` | implemented but not wired end-to-end | Manifest/registry/scoped-secret admission. Real ERP/CRM/Meta/HR/legal adapters absent. |
| `lumi-packs` | partially implemented | Validation/action materialization. Preconditions/explicit pack approvals need admission enforcement; compatibility and exception dispatch are not a full runtime. |
| `packs/*`, `lumi-workflows` | implemented but fixture-only | Three examples. Harness pre-seeds required records/files; `evaluate_pack` has no real executor injection. |
| `lumi-evals` | partially implemented | Run metrics/SLO/economics. No role/deployment/live provenance or sustained human-time ledger. Negative net value is clamped; fixture implementation-cost conversion is 100x too large. |
| `lumi-handoff` | implemented but fixture-only; not wired end-to-end | Progress, approval, exception, trust and control-plane DTOs. Serialization is not authenticated RPC or policy distribution. |
| `lumi-desktop`, `apps/desktop` | partially implemented | Tauri returns sample work/evidence/permissions; frontend approval commands are unregistered; stop changes only a boolean. |
| `lumi-versioning` | implemented but not wired end-to-end | Version/capability/migration helpers; JSON state store does not invoke MigrationChain. |
| `lumi-certification` | implemented but fixture-only | Caller booleans produce a report; no actual signature/notarization/update/rollback/SBOM verification. |
| `lumi-adversarial` | implemented but fixture-only | Useful injection/policy/secret regressions; integrated/live boundaries still need proof. |
| `lumi-runtime` | implemented but fixture-only | CLI dry-run smoke path; lower-level dispatch is separate from the full orchestrator. |
| `.github/workflows/ci.yml` | implemented and production-relevant | Rust matrix/dependency policy; no Tauri, real browser/native canary or release artifact certification. |
| Role Pack, measured scorecard, incident intake | missing | #41/#42 track minimal composition and useful measurements. |
| Real-system canaries | blocked by external credentials; blocked by customer/system access | No target tenant, scope, owner or representative work supplied. #40. |
| Signed customer builds | partially implemented; blocked by external credentials | Packaging/evidence preparation can proceed; signing requires provisioned authority. #11. |
| Sustained capacity/reuse proof | blocked by customer/system access | No four production weeks or three independent deployments in supplied evidence. |
| Marketplace, provider catalog, agent organization, horizontal polish | unnecessary for the current market proof | Defer until a narrow role earns repeatable live economics. |

## Test interpretation

Baseline: 392 passing local tests (391 sandboxed plus one OS Keychain test with host permission). Many directly call helpers or pre-seed verification state, so the former claim that every subsystem test drives through one choke point was too strong.

Before customer canaries, inject failures at the actual entry boundary, verify target/account identity and business postconditions, and retain all failed/ambiguous runs in the denominator. Never seed a success oracle and describe it as an observed external effect.

## Reviewed safety delta after the baseline

The safety PR adds fail-closed writes/restores, generation/lock protection,
durable retry admission, original-action/verifier binding, result identity
checks, scoped cross-run idempotency and classified failure audit evidence.
The action digest is versioned to v2; legacy ambiguous records are not silently
rewritten. Unsupported pack preconditions now refuse preparation, and malformed
client-key metadata is rejected at protocol/policy admission.

The baseline rows above remain an audit record. This delta does not establish
live connectors, authenticated sessions, customer baselines, signed distribution,
role capacity or repeatability. Those gates remain NO-GO.
