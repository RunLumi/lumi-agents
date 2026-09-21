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
| 13 | Scheduler/background triggers | implemented | `lumi-scheduler` + cron matcher; desktop wiring shipped: admitted firings materialize durable project-bound tasks (`automations.rs`, PR #119) |
| 14 | MCP/connectors/extension | partial | `lumi-connectors` (757 lines, 17 tests); real MCP bridge integration deferred |
| 15 | Desktop app/distribution | implemented | Tauri v2 shell; `release.yml` macOS universal + Windows x64 with checksums (`docs/release.md`) |
| 16 | Evals/observability/economics | implemented | `lumi-evals` (metrics, outcomes, economics, dogfood harness); live + fixture passes recorded |
| 17 | Security/privacy/secrets | implemented | `lumi-secrets`, policy ceilings, injection hygiene corpus, provider credential memory-only handling |
| 18 | Error taxonomy/retry/recovery | implemented | canonical `FailureCategory` end-to-end incl. desktop run envelopes |
| 19 | API control-plane/sync | absent | local-first product: no cloud control plane yet — intentional deferral (architecture: local authority) |
| 20 | Versioning/compat | implemented | `lumi-versioning`; policy-version compatibility in resume paths |
| 21 | Release certification | partial | `lumi-certification` + `docs/release.md`; **CycloneDX SBOMs for both Rust workspace and npm UI graph in release.yml**, SHA-256SUMS per release; code signing/notarization plumbed and secrets-gated (APPLE_* — activation pending org certificates) |
| 22 | V1 definition of done | meta | tracked via `docs/plan.md` status log |
| 23 | UX/handoff | implemented | `lumi-handoff` progress/exception/trust language drives the desktop views |
| 24 | Skills/subagents | **done (core)** | skills exist in packs; **`run_subagent` bounded delegation shipped (PR: spec24-subagents)**: narrowed capabilities, carved budget, turn deadline, no self-approval, honest failures; **§24.13 economics measured on every outcome (duration_ms, actions, model cost, vision/external-write counts)**; parallel fan-out economics evaluation deferred (needs live multi-provider runs) |
| 25 | Implementation order | meta | satisfied historically; see plan.md |
| 26 | Project workspace | implemented | Spec 26 backend + desktop Projects surface (PRs #54–#63) |
| 27 | Global/project automations | **partial → advancing** | PR #119: admitted schedule firings now materialize durable project-bound tasks with dedup/catch-up/quiet-hours policy via the scheduler gate. Remaining: desktop management surface (list/edit/enable), event triggers beyond cron, notification fan-out |
| 28 | Project skills/plugins | **partial → advancing** | PR #122: project-scoped admission layer shipped — `AdmissionRegistry` pins exact pack versions per project (atomic JSON, fail-closed), `admit_pack_for_project` enforces pin match, non-deprecated status, runtime floor, host OS at run start. Remaining: desktop management UI is conditional on an in-app pack catalog — pin management without in-app enforcement would be decorative (AGENTS.md §7); enforcement lives at pack-run admission (workflow mode) |
| 29 | Project connections/secrets | **done (core)** | `lumi-secrets` broker (OS keyring); desktop project Connections UI (add/disconnect/list, PR #121); **credential-presence verify without exposing values (`connections_verify`, PR: spec-29-depth)**; records file holds references only (e2e-enforced) |
| 30 | Document workspace preview/edit | **partial → advancing** | Markdown rendered/source/split (react-markdown+remark-gfm, HTML disabled, inert links); image preview via bounded base64 IPC; literal CSV/TSV grid; **CodeMirror 6 editing (history undo/redo, Mod-f find/replace) with per-language syntax highlighting for ~140 languages via @codemirror/language-data (rust/python/js-ts/json/yaml/sql/html/css/c/go/java from dedicated packages, the rest from bundled legacy modes; all lazy) and a read-only highlighted CodeView for code files (PR: spec30-code-display)**; **XLSX basic edit: literal cells editable, formula cells stay read-only last-saved-result, whole-workbook save traverses the gate via base64 binary save (PR #129)**; **DOCX basic text edit: body paragraphs editable (first 400, first-run formatting semantics disclosed), JSZip round-trip of word/document.xml through the same gated save (PR: spec30-docx-edit)**; **PDF.js lazy page rendering + per-page text search, worker bundled, library chunk lazy-loaded (§30.15)**; **preview breadth: all webview-decodable images (png/jpg/webp/gif/bmp/ico/avif/svg), video/audio with an honest decode-failure state, and zip archives as a read-only entry listing (extraction stays a gated write); binary routing unified — every self-fetching kind bypasses the text read (which refuses binaries), so DOCX/media/zip and even honest-unsupported binaries open their correct surface instead of an error toast (PR: spec30-preview-breadth)**; explicit unsupported-format states for DOCX/XLSX/PPTX and macro/encrypted files. **PPTX static preview shipped (§30.10 phase C, PR: spec30-pptx-preview)**: pptx-react-viewer read-only via the lazy `./viewer` subpath (slide rail, zoom, slide numbers); the MPL-2.0 `mtx-decompressor` dependency was explicitly approved 2026-09-21 and recorded in the desktop MPL notices. Macro-enabled (.pptm) and legacy (.ppt) stay unsupported. **staged-save binding shipped**: every UI file create/edit/delete is a durable USER Task/Run with a normalized ActionProposal, policy gate, checksum postconditions, and verified evidence (§30.5/§30.12). Remaining: PPTX basic-edit qualification (the §30.10 supported-operation profile + hostile-file corpus — a separate gate from preview), IME corpus tests |
| 31 | Delegated work contract | implemented | delegation loop wired end-to-end (PRs #90/#99/#100/#102/#103) |

Local-model live evidence (2026-09-20): `docs/evals/dogfood-run-live.json` —
qwen2.5:3b (Ollama, OpenAI-compatible, local) completed the Work-mode task
through the full gate, 3/3 verified runs after a clarified brief.
