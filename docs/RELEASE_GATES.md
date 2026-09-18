# Release gates

A Lumi release is allowed to move forward only when the trust boundary, workflow reliability, and rollback story are stronger than the previous ring.

## Ring 0: internal development

Required:

- all Rust checks green;
- policy tests green;
- dependency/advisory/license checks green;
- no secrets/customer data in repository or fixtures;
- upstream binaries pinned by exact version/checksum;
- deterministic fixtures for changed behavior;
- protocol compatibility tests for changed contracts.

## Ring 1: internal alpha

Before an employee installs a build:

- local policy cannot be bypassed by planner/model;
- external/destructive actions require policy or approval;
- local emergency stop works;
- task cancellation works;
- no plaintext production secret appears in prompts/logs/traces;
- deterministic smoke fixtures pass on target OS;
- required postcondition verifier can block false success;
- updater is disabled or signed/controlled.

Alpha workflow gate:

- >=30 repeated runs per target workflow;
- >=90% verified completion in controlled scenarios;
- 0 unauthorized side effects;
- failure taxonomy recorded.

## Ring 2: dogfood

In addition:

- signed macOS and Windows builds;
- macOS notarization;
- Keychain/Windows protected secret storage;
- explicit permission onboarding;
- browser-profile attachment explicit/policy-controlled;
- evidence redaction tested;
- crash/restart recovery tested;
- ambiguous side effects do not auto-retry;
- provider budgets enforced;
- prompt-injection suite passes.

## Ring 3: customer canary

In addition:

- signed updater;
- rollback tested;
- SBOM generated;
- artifact checksums published;
- device identity and revocation;
- customer data-egress policy tested;
- local-only mode tested when offered;
- support/runbook exists;
- incident/kill-switch runbook exercised;
- compatibility matrix documented.

Canary workflow gate:

- >=100 representative runs per certified workflow;
- >=95% verified completion;
- <5% unexpected human-rescue events on hardened routine paths;
- zero policy bypasses in release corpus;
- expected human approvals are not counted as rescue.

## Ring 4: stable

In addition:

- MDM/enterprise deployment path documented where applicable;
- update signing keys have backup/recovery process;
- cross-platform regression matrix blocks release;
- escaped production failures are represented in regression corpus where reproducible;
- customer workflow packs are independently versioned;
- release notes identify affected contracts/workflows/providers;
- rollback target is known and tested.

For narrow hardened workflows, target 99%+ verified completion on explicitly supported OS/app/version combinations.

Consequential actions may still require approval regardless of technical success rate.

## Mandatory release evidence

Every external release must be able to answer:

1. What changed?
2. Which protocol/contracts changed?
3. Which workflows/providers/apps are affected?
4. Which security boundary changed?
5. Which evals prove the behavior?
6. What remains unverified?
7. How do we roll back?
8. What third-party/license changes entered?
9. Which artifacts are signed and checksum-verified?
10. Which customer data/evidence policy changed?

If these answers are unclear, do not ship.
