# V1 Completion Plan

This plan takes Lumi Agents from its current state (23 crates, 392 tests, NO-GO) to
spec-22 GO. It is written for the team — human or agent — that will execute the
remaining work.

**Baseline**: main @ PRs #17–#38 merged · 23 crates · 392 tests · 60 suites ·
0 clippy warnings · specs 01–14, 16–20, 23 implemented · issues #10, #11 open.

**Readiness report**: `docs/v1-readiness.md` (updated after each phase below).

---

## Phase 1 — Desktop shell (spec 15 + issue #10)

The backend API (`lumi-desktop`) and Tauri scaffold (`apps/desktop`) are done.
This phase builds the frontend UI and wires it to live orchestrator state.

### 1.1 Tauri IPC → orchestrator wiring

Replace the stub commands in `apps/desktop/src-tauri/src/lib.rs` with a real
`AppState` that holds an `Orchestrator`, `ApprovalLedger`, `SideEffectJournal`,
and `CancelToken`. Each IPC command delegates to the existing crate APIs.

| Task | Acceptance |
|---|---|
| Hold `Orchestrator<InMemoryStateStore>` in Tauri managed state | `execute_step` callable from IPC |
| `get_pending_approvals` reads from `ApprovalLedger` | Returns real approval cards with digests |
| `approve_action` calls `ledger.validate_and_consume` | Single-use approvals consumed; digest mismatch fails |
| `emergency_stop` calls `CancelToken.cancel()` | All subsequent `execute_step` calls return `Stopped` |
| `get_evidence` reads from `lumi-audit` ledger + evidence store | Trust-labeled entries (§23.9) |

Effort: ~2 days. Tests: Tauri command tests using the existing
`InMemoryStateStore` + fixture executors.

### 1.2 Frontend views

Build the HTML/CSS/JS views against the real IPC data.

| View | Spec | Acceptance |
|---|---|---|
| Dashboard | §23.3 | Shows phase, milestones, cost, budget bar |
| Approvals | §23.4 | Business effect, not gesture; approve/reject wired to ledger |
| Exceptions | §23.5 | What blocked, safe choices, evidence refs, next step |
| Evidence | §23.9 | Trust labels distinct; attempted ≠ done |
| Kill switch | §23.10 | One click stops everything; state reflects immediately |

Effort: ~3 days. Tests: Playwright browser tests against the Tauri dev server
(optional — manual testing acceptable for alpha).

### 1.3 Packaging

| Task | Acceptance |
|---|---|
| `cargo tauri build` produces installable app | `cargo tauri build` exits 0 on macOS |
| App launches and shows the dashboard | Manual smoke test |
| Icon + branding | Placeholder replaced with real icon |

Effort: ~1 day.

---

## Phase 2 — Secrets broker → Cua wire client (spec 07 completion)

Connect the `lumi-native` DesktopDriver to the actual Cua Driver binary.

### 2.1 CuaDriverAdapter wire protocol

| Task | Acceptance |
|---|---|
| Implement `DesktopDriver::execute` in `CuaDriverAdapter` | Calls the Cua Driver process via its stdio/HTTP protocol |
| Resolve auth via `SecretBroker` at the call boundary | Credentials never stored in the adapter |
| Map Cua output to `NativeOutcome` with effect oracle | OS success ≠ DELIVERED; oracle must observe |
| Integration test with a scripted Cua process | Hermetic; same pattern as `lumi-browser` fake worker |

Effort: ~3 days. Blocked on: Cua Driver binary availability (not credentials).

---

## Phase 3 — Spec 19 control-plane + spec 20 migrations (fill remaining gaps)

The `lumi-handoff` and `lumi-versioning` crates have the contract types. This
phase adds the runtime enforcement.

### 3.1 Control-plane policy distribution

| Task | Acceptance |
|---|---|
| Load signed org policy from file/endpoint | Invalid signature → reject, keep current policy |
| Org ceiling enforced at connector admission | Denied origin/license blocked |
| Offline: control-plane unreachable → local policy continues unchanged | No expansion |

Effort: ~2 days.

### 3.2 Schema migration pipeline

| Task | Acceptance |
|---|---|
| Wire `MigrationChain` to the JSON state store | Old-version state files upgraded on load |
| Pack manifest version checked at admission | Incompatible pack refused |
| RouteSnapshot schema version checked at resume | Incompatible snapshot re-evaluated |

Effort: ~1 day.

---

## Phase 4 — Real-system workflow canaries (spec 22.4)

**STOP: Requires external credentials and customer baselines.**

### 4.1 Prerequisites (human input needed)

| Item | Who provides | Notes |
|---|---|---|
| ERP API endpoint + credentials | Customer/ops | Read-only for fixture phase |
| CRM API endpoint + credentials | Customer/ops | Read + write for canary phase |
| SMTP/email API endpoint + credentials | Customer/ops | Send required |
| Baseline manual-minutes measurement | Customer/ops | Time 5–10 manual runs |
| Residual human-minutes measurement | Customer/ops | Time human involvement during automated runs |

### 4.2 Execution plan

| Step | Gate |
|---|---|
| Wire pack executors to live connectors via secrets broker | Health probe passes |
| 30-run Alpha: ≥90% verified completion, 0 unauthorized | Alpha SLO gate |
| 100-run Canary: ≥95% verified completion, <5% rescue, 0 bypass | Canary SLO gate |
| Record economics: gross/net value, payback | Readiness report updated |

Effort: ~1 week per workflow (including fix iterations).

---

## Phase 5 — Release certification (spec 21 + issue #11)

**STOP: Requires Apple Developer + Windows code-signing credentials.**

| Task | Acceptance |
|---|---|
| Apple Developer account + App Store Connect | Certificate available in keychain |
| macOS code signing + notarization | `codesign --verify` + `spctl --assess` pass |
| Windows code signing (EV cert or Azure Trusted Signing) | `signtool verify` passes |
| Updater manifest + rollback test | Update → rollback → verify |
| SBOM generated (cargo auditable or syft) | SBOM file in release artifacts |
| Release pipeline CI job runs all certification checks | `lumi-certification` green in CI |

Effort: ~3 days (after credentials obtained).

---

## Phase 6 — Spec 24 skills/subagents

Only if measured value appears from Work-mode usage. Do not build speculatively.

---

## Dependency graph

```
Phase 1 (Desktop shell)
    ↓
Phase 2 (Cua wire client)     ← no external blocker
    ↓
Phase 3 (Control plane)       ← no external blocker
    ↓
Phase 4 (Real canaries)       ← BLOCKED: credentials + baselines
    ↓
Phase 5 (Release cert)        ← BLOCKED: signing credentials
    ↓
Readiness report → GO
```

Phases 1–3 are buildable now. Phase 4 and 5 require human input.

---

## Milestone summary

| Milestone | Effort | Blocked? |
|---|---|---|
| Phase 1: Desktop shell | ~6 days | No |
| Phase 2: Cua wire client | ~3 days | No (Cua binary needed for integration test) |
| Phase 3: Control plane + migrations | ~3 days | No |
| Phase 4: Real-system canaries | ~1 week/workflow | YES: credentials |
| Phase 5: Release certification | ~3 days | YES: signing credentials |
| Readiness refresh | ~1 day | No |
| **Total buildable now** | **~12 days** | |
| **Total blocked on credentials** | **~1–3 weeks** | |

---

## Non-goals (spec 22 §22.11)

- Public marketplace
- Full Linux desktop
- First-party replacement for Cua
- Enterprise control-plane feature completeness
- Autonomous financial/legal execution without approval
- Dozens of providers
- Agent swarm architecture
