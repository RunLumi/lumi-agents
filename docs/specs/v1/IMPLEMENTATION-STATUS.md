# Implementation status per spec (living matrix)

Updated 2026-09-20. Status vocabulary: **implemented** (real code path +
tests + product surface), **partial** (real code path exists; product
surface or breadth missing — the named gap is the follow-up), **absent**
(no implementation; intent only), **meta** (process document, no runtime).

| Spec | Area | Status | Notes / gap |
|---|---|---|---|
| 01 | Core domain model | implemented | `lumi-protocol` (ids, resources, actions, envelopes) |
| 02 | Task/Run state machine | implemented | `lumi-state` durable store + orchestrator transitions; startup recovery sweep (desktop) |
| 03 | Action/observation protocol | implemented | `action-protocol-v0.md` + gate pipeline in `lumi-orchestrator` |
| 04 | Policy/approval/capabilities | implemented | `lumi-policy` registry, approval ledger, digest-bound approvals; injection fixtures in `lumi-adversarial` |
| 05 | Execution router | implemented | tier policy + executor descriptors (`orch.rs`, `router.rs`) |
| 06 | Browser executor | partial | `lumi-browser` (915 lines, 19 tests); product wiring deferred until a workflow requires browser work (plan: conditional expansion) |
| 07 | Native desktop executor | partial | `lumi-native` (855 lines, 20 tests); conditional expansion as above |
| 08 | Files/shell/artifacts | implemented | `lumi-workspaces` + `lumi-project` files/shell/artifact store; gated writes with checksum guards |
| 09 | Model provider routing | implemented | `lumi-models`: 4 driver families under one contract suite; routing tests |
| 10 | Context/memory/retrieval | implemented | `lumi-memory` + project memory with provenance + invalidation |
| 11 | Audit/evidence/verification | implemented | `lumi-audit` hash chain + postcondition verifier; Evidence read model in desktop (PR #101) |
| 12 | Workflow packs | implemented | `lumi-packs` certification tests |
| 13 | Scheduler/background triggers | implemented (crate) | `lumi-scheduler`; desktop Automations UI wiring is the follow-up (P1 #73) |
| 14 | MCP/connectors/extension | partial | `lumi-connectors` (757 lines, 17 tests); real MCP bridge integration deferred |
| 15 | Desktop app/distribution | implemented | Tauri v2 shell; `release.yml` macOS universal + Windows x64 with checksums (`docs/release.md`) |
| 16 | Evals/observability/economics | implemented | `lumi-evals` (metrics, outcomes, economics, dogfood harness); live + fixture passes recorded |
| 17 | Security/privacy/secrets | implemented | `lumi-secrets`, policy ceilings, injection hygiene corpus, provider credential memory-only handling |
| 18 | Error taxonomy/retry/recovery | implemented | canonical `FailureCategory` end-to-end incl. desktop run envelopes |
| 19 | API control-plane/sync | absent | local-first product: no cloud control plane yet — intentional deferral (architecture: local authority) |
| 20 | Versioning/compat | implemented | `lumi-versioning`; policy-version compatibility in resume paths |
| 21 | Release certification | partial | `lumi-certification` + `docs/release.md`; signing/notarization + SBOM deferred (tracked) |
| 22 | V1 definition of done | meta | tracked via `docs/plan.md` status log |
| 23 | UX/handoff | implemented | `lumi-handoff` progress/exception/trust language drives the desktop views |
| 24 | Skills/subagents | partial | skills exist in packs; subagent orchestration deferred |
| 25 | Implementation order | meta | satisfied historically; see plan.md |
| 26 | Project workspace | implemented | Spec 26 backend + desktop Projects surface (PRs #54–#63) |
| 27 | Global/project automations | partial | scheduler crate ready; durable occurrence → Task wiring is the P1 follow-up (#73) |
| 28 | Project skills/plugins | partial | pack snapshots exist; project-scoped admission UI deferred |
| 29 | Project connections/secrets | partial | `lumi-secrets` broker; per-project connection requirements UI deferred |
| 30 | Document workspace preview/edit | **partial → advancing** | Markdown rendered/source/split (react-markdown+remark-gfm, HTML disabled, inert links); image preview via bounded base64 IPC; literal CSV/TSV grid; **CodeMirror 6 editing (history undo/redo, Mod-f find/replace, markdown highlighting) — PR #slice2**; **PDF.js lazy page rendering + per-page text search, worker bundled, library chunk lazy-loaded (§30.15)**; explicit unsupported-format states for DOCX/XLSX/PPTX and macro/encrypted files. Remaining: PDF thumbnails, DOCX/XLSX/PPTX qualified adapters (phase B/C), staged-save orchestrator binding (§30.12) |
| 31 | Delegated work contract | implemented | delegation loop wired end-to-end (PRs #90/#99/#100/#102/#103) |

Local-model live evidence (2026-09-20): `docs/evals/dogfood-run-live.json` —
qwen2.5:3b (Ollama, OpenAI-compatible, local) completed the Work-mode task
through the full gate, 3/3 verified runs after a clarified brief.
