# Lumi Agents V1 Readiness Report

Date: 2026-09-19 (updated)
Baseline: main @ PRs #17–#34
Verdict: **NO-GO for V1 release** — large portions of the platform are implemented and certified at the fixture level; the gaps below are honest and specific.

---

## 1. Requirement matrix (spec 22 → implementation → evidence)

Legend: ✅ implemented + tested · 🟡 partially implemented (honest gap noted) · ❌ not started.

### 22.2 Required platform capabilities

| MUST | Implementation | Test/Eval | Status |
|---|---|---|---|
| Work mode task runner | `lumi-agent` (plan→propose→gate→execute→observe loop; approval pause/resume; turn+budget bounds) | `crates/lumi-agent/tests/work_mode.rs` (5 scenarios incl. approval gating, unknown-tool, runaway bound) | ✅ (fixture planners; live-model E2E pending credentials) |
| Workflow mode runner | `lumi-packs` prepare_run + `lumi-workflows` certification harness | `crates/lumi-packs/tests/pack_certification.rs` (8) + `crates/lumi-workflows/tests/three_packs.rs` (5) | ✅ |
| Durable task/run state | `lumi-state` StateStore (InMemory + JSON atomic snapshot) | `crates/lumi-state/src/store.rs` tests incl. crash/resume | ✅ |
| Checkpoint/resume | Checkpoint + PreActionCheckpoint + `resume()` (§2.8 gates: device/policy/runtime compat, approval re-validation, ambiguity-first) | spec 2.16 test set | ✅ |
| Cancellation | CancelToken in orchestrator gate (checked first) + ShellSandbox poll-kill + BrowserWorkerHandle::kill | vertical-slice + workspaces tests | ✅ |
| Local policy | `lumi-policy` deny-by-default registry, hard gates, pre-auth narrowing | 24 policy tests incl. all §4.15 scenarios | ✅ |
| Scoped approvals | ApprovalLedger bound to material digest + tenant + expiry + single-use | §4.15 tests | ✅ |
| Audit/evidence | `lumi-audit` hash-chained ledger + JSONL store + EvidenceStore (redaction/retention/screenshot gating) | §11.14 test set; tamper detection on load | ✅ |
| Postcondition verification | PostconditionVerifier (7 check kinds); AMBIGUOUS ≠ PASSED | §11.14 test set | ✅ |
| Connector/API execution contract | Orchestrator executor contract + http-connector descriptors | fixture executors | 🟡 contract proven; real connector (ERP/CRM) adapters absent |
| Playwright browser executor | `lumi-browser` + `workers/playwright` | 17 hermetic protocol/executor tests; real-browser E2E documented for local runs only | 🟡 worker + contract done; browser-download E2E not in CI |
| macOS/Windows native executor | `lumi-native` DesktopDriver contract, session/generation/permission gates, effect-oracle outcomes, certification-matrix format | 20 native certification tests on both platform fixtures | 🟡 contract + fixtures done; **Cua wire client not connected** (awaits secrets-broker wiring; adapter returns stable pre-mutation error) |
| Controlled files | `lumi-workspaces` (path re-anchoring, traversal/symlink refusal, reversible delete, trash+restore) | §8.14 fixtures: traversal/symlink/overwrite/delete-restore | ✅ |
| Sandboxed/bounded shell | ShellSandbox: env allowlist (no inheritance), timeout kill, cancellation, output caps, exit-code fidelity, isolation honesty (refuses SANDBOXED demands) | §8.14 shell fixtures incl. env-leak probe | 🟡 HOST_BOUNDED only; SANDBOXED/CONTAINERIZED refused explicitly (by design) |
| Artifact generation/validation | ArtifactStore: provenance, checksum, built-in+custom validators, lifecycle | §8.14 artifact fixtures incl. publication separation | ✅ (DOCX/XLSX deep validation beyond magic bytes = future) |
| Provider-neutral model router | `lumi-models` routing order + RouteSnapshot resume re-validation | routing contract tests | ✅ |
| OpenAI + Anthropic + Gemini + local path | 4 drivers; OpenAI-compatible as distinct driver | shared contract suite (13 scenarios × 4 drivers, hermetic) | ✅ contracts; live-network calls untested (no credentials) |
| Context/memory separation | `lumi-memory` (WorkingContext / MemoryStore / RetrievalIndex / compaction; workflow state = lumi-state; evidence = lumi-audit) | §10.14 test set | ✅ |
| Background schedule/event support | `lumi-scheduler` (TriggerRequest normalization, DeduplicationLedger, UnattendedLease registry with mid-run recheck, Scheduler with quiet hours / catch-up policies / device availability) | `crates/lumi-scheduler/tests/scheduler_certification.rs` (6): duplicate webhook dropped pre-task, missed-window policies, device offline, lease revoked mid-run, budget exhaustion, approval delay parks task, local-only cannot cloud fail over, kill switch | ✅ |
| Extension/connector manifest | `lumi-connectors` (ConnectorManifest, ExtensionRegistry with capability-growth review, OrgPolicyCeiling, ScopedSecretBackend) | `crates/lumi-connectors/tests/connector_certification.rs` (7): policy-shape side effects, unrelated secret refused, growth requires review, network target denied, ceiling blocks project ext, crash isolation, cross-tenant | ✅ |
| Employee desktop shell | — | — | ❌ (issue #10) |
| Signed update path | — | — | ❌ (issue #11) |
| Device registration/revocation | Device trust states + policy denial (revoked ⇒ deny) | policy tests | 🟡 in-process model only; no fleet/control-plane revocation |
| Evals/observability/economics | `lumi-evals` MetricStore/SLO gates/alerts/economics | §16.15 proofs | ✅ |

### 22.3 Required safety

| MUST | Evidence | Status |
|---|---|---|
| Zero known policy bypass | Policy engine deny-by-default; hard gates non-narrowable; §4.15 suite | ✅ in-process (fleet/remote surfaces absent) |
| Action mutation invalidates approval | Digest mismatch test | ✅ |
| Cross-tenant isolation | Policy grants tenant-scoped; audit/evidence/memory/index all tenant-scoped + tested | ✅ in-process |
| Local-only provider policy enforced | Routing eligibility filter; fallback drawn only from filtered set; tested | ✅ |
| Prompt-injection suite | `lumi-adversarial` durable corpus: injection in tool results inert, policy-override arguments inert, cross-tenant refused, secret redaction, approval forgery, hostile planner output, revoked device/auth principal | ✅ |
| Secret isolation | SecretValue zeroize/redact/serialize-proof; broker audit without values; shell env-leak probe | ✅ |
| Crash/double-submit recovery | SideEffectJournal + resume ReverifySideEffect plan; no-re-execution after ConfirmedApplied | ✅ |
| Local emergency stop | CancelToken end-to-end (orchestrator, shell, browser kill) | 🟡 runtime-level; no desktop-app UI switch |
| Device/lease revocation | Instance/device rejection at admission | 🟡 in-process |
| Signed external builds | — | ❌ |

### 22.4 Required workflows

Three packs exist (invoice-reconciliation, quote-followup, expense-report-audit incl. a NATIVE_SEMANTIC step) and pass the harness:

- 100 runs/pack fixture corpus: **100% verified completion, 0 unauthorized side effects, 0 rescues, Alpha SLO passed**
- exception paths, approval gating, compatibility matrices, economic baselines declared

**Gap:** ≥100 runs against REAL systems with measured human baselines (the actual §22.4 gate) — requires live credentials + customer process measurement. Fixture ≠ production evidence.

### 22.5–22.10 gates

- **22.5 Work mode gate**: browser research / files / shell / artifacts / long-running coverage exists at component level; one connector/API + resumable task demonstrated via fixtures. 🟡 live-system demonstration pending.
- **22.6 Provider neutrality**: same workflow via ≥2 providers — contract-proven hermetically; live runs pending. Local/OpenAI-compatible path passes core contract tests. 🟡
- **22.7 Recovery gate**: restart, provider failure, approval delay, ambiguous side effect all covered by tests. Network interruption/browser crash covered by failure-category mapping + fixtures. ✅ at fixture level.
- **22.8 Distribution gate**: ❌ not started (signing/notarization/updater).
- **22.9 Documentation gate**: AGENTS.md, roadmap, architecture, specs, security model current. Provider docs current. Workflow-pack docs: pack.json format documented via packs/ + spec 12; user-facing pack authoring guide missing. 🟡
- **22.10 Economics gate**: runtime records cost per action/run (micro-USD), packs declare baselines, `lumi-evals` derives net value + payback. Manual-baseline measurement is declared-not-measured until pilots. 🟡

---

## 2. What is provably true today

- 12 crates, 3 packs, 1 worker; ~2,700 lines of spec-normative test code across 45 green suites on macOS/Ubuntu/Windows; clippy `-D warnings`; cargo-deny clean.
- The invariant chain (models propose → policy authorizes → executors act → verifiers determine success → audit records) is enforced at ONE choke point (`Orchestrator::execute_step`) and every subsystem test drives through it.
- Ambiguity safety: consequential actions with unverifiable outcomes become AMBIGUOUS, journal-block resume, and require external-state verification before any retry — proven by crash/ambiguity fixtures.
- Approval integrity: digest-bound, tenant-bound, expiring, single-use, human-only issuance — with mutation/expiry/cross-tenant/replay tests.
- Path/secrets hygiene: traversal + symlink + absolute re-anchoring refused/normalized; host env never inherited by shell children; secret values zeroized, redacted in Debug/Display/serde, absent from audit.

## 3. Remaining distance to V1 (ordered)

1. **Spec 15 + issue #10 desktop app** (Tauri shell, approvals/exceptions UX, emergency stop UI, privacy indicators) — the largest remaining surface.
2. **Spec 21 + issue #11 release certification** (signing, notarization, updater/rollback, SBOM) — requires Apple Developer / Windows code-signing credentials (STOP: external credentials needed).
3. **Spec 19/20 control-plane + migrations** — in-process revocation exists; fleet surface remains.
4. **Spec 23 UX contracts + spec 24 skills/subagents (only if measured value)**.
5. **Live-system workflow canaries** — blocked on credentials + customer baselines (STOP: external input needed).
6. **Readiness report refresh** after each of the above.

## 4. Go/No-Go

**NO-GO.** The trust substrate (policy/approval/audit/verification/recovery), the execution surfaces' contracts (browser/native/connector/model), and the measurement pipeline (evals/economics) are implemented and certified hermetically. What separates today's state from V1 is (a) the remaining Phase B/C/D surfaces — scheduler, extensions, desktop app, distribution, — and (b) evidence from real systems with real people in the loop, which no amount of in-repo code can substitute for.
