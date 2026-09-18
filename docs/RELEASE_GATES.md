# Release gates

## Internal alpha

Required before an employee installs a build:

- upstream computer-use binary pinned to an exact version and checksum;
- local policy gate cannot be bypassed by the planner/model;
- external side effects require approval by default;
- local stop/kill path tested;
- no production secret appears in logs or traces;
- deterministic smoke fixture passes on the target OS.

## Customer pilot

In addition to alpha:

- macOS build signed and notarized;
- Windows installer code-signed;
- updater artifacts signed and rollback tested;
- Keychain/Credential Manager used for local secrets;
- permission onboarding is explicit;
- browser-profile attachment is explicit and policy controlled;
- redacted evidence bundle exists for every material action;
- device can be remotely revoked;
- workflow eval threshold in `docs/ROADMAP.md` is met;
- dependency/license scan and SBOM are produced.

## Production

In addition to pilot:

- MDM/enterprise deployment path documented;
- update signing keys have backup/recovery procedure;
- security incident and kill-switch runbooks tested;
- cross-platform regression suite blocks release;
- customer-specific workflow packs are versioned independently from the runtime;
- every release can answer: what changed, which workflows are affected, how to roll back, and which evidence proves the build.
