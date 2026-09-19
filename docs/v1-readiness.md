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
- Spec 26 now makes Folder-as-Project a V1 Work-mode requirement. Existing workspace primitives are useful substrate, but durable Project identity, Open Folder/Recent, Project-bound task resume, change-set ownership, Git semantics and external-edit conflict handling are not yet proven end to end.

## Spec 22 gates

| Gate | Current evidence | Remaining requirement |
|---|---|---|
| 22.2 platform | Policy, state, audit, verifier, execution/model contracts, task-scoped workspace operations, scheduler and extension admission have code/tests | Integrate them into a durable environment service; implement Spec 26 Project identity/Open Folder/root policy/Git/change sets; prove a real bounded workflow |
| 22.3 safety | Adversarial and approval fixtures pass | Fix ignored durable-intent errors/direct replay risks; validate integrated device revocation, stop, evidence controls and real executor behavior |
| 22.4 workflows | Three synthetic pack examples | Each release-certified pack needs >=100 representative real-system canary runs, >=95% verified completion, zero unauthorized effects, measured baseline/residual work and a certified matrix |
| 22.5 Work mode | Component and fixture loops; safe workspace primitives exist | Open a dirty real repository as a durable Project; multi-file edit; preserve unrelated user work; run validation; inspect Lumi-only change set; restart/reopen/resume; plus live representative browser/artifact/connector work |
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
| Folder-as-Project Work mode | [#50](https://github.com/RunLumi/lumi-agents/issues/50) | Open dirty real repo; bounded multi-file change; preserve user edits; validate; Lumi-only change set; restart/reopen/resume; zero root escape |
| Executor, artifact and approval boundaries | [#44](https://github.com/RunLumi/lumi-agents/issues/44) | Real worker protocol verified; artifact escapes refused; sensitivity, device state and required evidence enforced before customer canaries |
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
