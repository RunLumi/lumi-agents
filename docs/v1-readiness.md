# V1 and market-proof readiness

Reviewed 2026-09-19 against remote `main` at `96fa46c4dba7aa70e86830232e2ba29e371b8d84` (PRs #17–#39).

**NO-GO for V1 release, unattended customer operation, or claims of role capacity replacement.**

The repository contains substantial implementation, but integration and business evidence are weaker than its crate count suggests. The [implementation truth map](implementation-truth.md) distinguishes code, fixtures, missing wiring and external dependencies. A passing fixture or configuration boolean is not evidence of a live effect.

## Verified baseline

- 23 workspace crates, three example Workflow Packs and one Playwright worker.
- 392 local Rust tests pass: 391 in the sandbox and the isolated OS Keychain integration test outside it. The sandbox denies Keychain authorization; running that test with host access succeeds.
- Root CI defines Rust format, clippy and tests on Ubuntu/macOS/Windows and a dependency-policy job. It does **not** build or certify the Tauri app.
- Desktop, handoff, versioning and release-check code landed in PRs #36–#38. The previous readiness report incorrectly marked these as absent.
- The shipped CLI is a dry-run smoke path. Tauri commands return sample data, and its stop button only changes a boolean at this baseline.
- Workflow evaluation pre-seeds expected records/artifacts and uses fixture executors. Repeating it 100 times is engineering evidence only.

## Spec 22 gates

| Gate | Current evidence | Remaining requirement |
|---|---|---|
| 22.2 platform | Policy, state, audit, verifier, execution/model contracts, workspace operations, scheduler and extension admission have code/tests | Integrate them into a durable environment service and a real bounded workflow |
| 22.3 safety | Adversarial and approval fixtures pass | Fix ignored durable-intent errors/direct replay risks; validate integrated device revocation, stop, evidence controls and real executor behavior |
| 22.4 workflows | Three synthetic pack examples | Each release-certified pack needs >=100 representative real-system canary runs, >=95% verified completion, zero unauthorized effects, measured baseline/residual work and a certified matrix |
| 22.5 Work mode | Component and fixture loops | Live representative tasks with inspected artifacts and recovery; horizontal feature parity is not a pilot prerequisite |
| 22.6 providers | Four provider adapter families pass hermetic contracts | Same workflow on two live providers, inside the same privacy envelope |
| 22.7 recovery | State/journal/resume fixtures | Storage failure, direct replay and restart must fail closed at actual execution entrypoints |
| 22.8 distribution | Signing/update configuration checker | Actual signed/notarized macOS and signed Windows artifacts, validated update/rollback, provenance and SBOM |
| 22.9 documentation | Numbered specs and ADRs | Keep implementation truth, pilot procedure and issue graph synchronized |
| 22.10 economics | Arithmetic over caller inputs and declared pack baselines | Measured work units, all human-time categories and full runtime/support/deployment costs |

The native-desktop requirement in spec 22 remains an eventual V1 target. It is not a reason to force native execution into an API-accessible lighthouse pilot.

## P0 market-proof work graph

| Work | Tracking | Exit evidence |
|---|---|---|
| Durable-intent and replay safety | [#43](https://github.com/RunLumi/lumi-agents/issues/43) | Executor never called after failed intent persistence; ambiguous/applied effects not replayed |
| One lighthouse and real-system Gate A | [#40](https://github.com/RunLumi/lumi-agents/issues/40) | Named system/supervisor, representative queue, 100 measured live/staging runs |
| Minimal Role Pack and honest scorecard | [#41](https://github.com/RunLumi/lumi-agents/issues/41) | Packs compose without authority growth; synthetic evidence cannot grant live maturity |
| Independent deployment reuse and incidents | [#42](https://github.com/RunLumi/lumi-agents/issues/42) | Three independent deployments; measured customization and regression learning |
| Employee desktop | [#10](https://github.com/RunLumi/lumi-agents/issues/10) | Runtime-derived queue/approvals/exceptions/evidence and effective emergency stop |
| Distribution security | [#11](https://github.com/RunLumi/lumi-agents/issues/11) | Artifact-bound release proof, signing, updater and rollback |

Existing #10/#11 remain open: scaffolds did not satisfy their acceptance criteria. No obsolete open issue was found in the reviewed two-issue baseline.

## External dependencies

**BLOCKER — real system and accountable pilot owner.** No application tenant, queue, approved permission scope, access owner or measured baseline has been supplied. The owner named Facebook Ads Ops, HR Ops and Legal Ops as candidate families; this is not system access. Smallest input: one selected environment and responsibility, supervisor, scoped credential references and representative work. Prepare admission, measurement and regressions while access is unresolved.

**BLOCKER — live provider credentials.** No live provider run has been evidenced. Smallest input: an allowed provider instance and locally stored credential reference under the pilot data policy. Contract tests can continue independently.

**BLOCKER — distribution credentials and artifact chain.** Apple/Windows signing authority and release identities have not been evidenced. Smallest input: approved signing identities in the release environment and an owned update destination. Packaging, validation and rollback preparation can proceed; configuration is not signed-release evidence.

**BLOCKER — sustained operation and independent customers.** Four consecutive production weeks and three independent deployments require elapsed operation and access. Code cannot replace those observations.

## Release decision

Fixture success, customer canary readiness, V1 conformance and market repeatability are separate decisions. No market gate is passed at this baseline, and no savings or worker-replacement claim is supported.

## Safety repair under review

The current safety change repairs the durable-intent barrier, binds ambiguity
resolution to the original action and verifier plan, validates executor response
identity, prevents scoped business-key replay across runs, and persists retry
admission. State writes reject stale generations and use a transaction lock;
an abandoned lock is an explicit recovery condition, never automatically stolen.
Approval digests now bind sensitivity, risk, actor and idempotency metadata.

Pack preparation now rejects unsupported preconditions and missing client keys,
keeps action identity stable within a run, and resolves nested/braced templates
without interpreting input values as new templates. The invoice example's file
postcondition now refers to its actual period-specific output path.

These are deterministic safety regressions, not live workflow or release proof.
General pack `ApprovalRule::Always` still needs the guarded execution path; the
role bridge uses the explicit approval gate. Device-state integration, durable
end-to-end audit/evidence delivery and actual customer adapters remain open.
