# 17 — Security, Privacy & Secrets v1

Status: Normative

## 17.1 Goal

Prevent lower-trust models/content/extensions from expanding authority or leaking sensitive data.

[Spec 29](29-project-connections-secrets.md) is the normative project-connection,
credential storage, Git hygiene and migration contract. Project installation
follows [Spec 28](28-project-skills-plugins.md).

## 17.2 Threat model

V1 explicitly addresses:

- prompt injection;
- credential leakage;
- destructive action;
- wrong target/destination;
- privilege escalation;
- cross-tenant data leakage;
- compromised extension/upstream;
- malicious update;
- stale UI state;
- duplicate side effects after crash;
- sensitive screenshots/traces;
- unsafe provider data egress;
- project/credential-reference copying and same-user executable-plugin escape.

## 17.3 Trust rule

These are lower-trust:

- model output;
- websites;
- email;
- documents;
- downloaded files;
- MCP/plugins;
- browser worker;
- vision model;
- third-party native driver;
- remote orchestration.

They MAY propose/observe but MUST NOT grant authority.

## 17.4 Secret references

Secrets MUST use opaque refs.

Plaintext secret MUST NOT be stored in:

- prompts;
- audit;
- traces;
- screenshots;
- workflow definitions;
- fixtures;
- crash dumps;
- project working directories, plugin packages or their credentials.json files.

Portable project configuration declares requirements. Actual secret values stay
in approved protected storage. `.gitignore` is supplementary Git hygiene, never
an access-control, encryption or sandbox boundary. Existing legacy credentials
require the explicit host-only migration process in Spec 29.

## 17.5 Secret resolution

Resolve at narrowest executor boundary.

Preferred stores:

- macOS Keychain;
- Windows protected credential storage;
- approved enterprise secret manager.

No silent fallback to plaintext files when protected storage is locked or
unavailable. Report a credential-store blocker. An explicitly authorized
memory-only session is ephemeral and does not establish unattended readiness.

## 17.6 Secret scope

Executor/extension receives only required secret.

No extension may enumerate entire secret store.

A reference is not a bearer capability. Validate authenticated caller, tenant,
registered project/environment, approved component, external account/audience,
operation/destination and current lease/revocation state under Spec 29. Prefer
brokered requests rather than raw tokens for untrusted executable components.

## 17.7 Data egress policy

Tenant policy MUST define:

- allowed providers;
- local-only categories;
- allowed regions;
- image/screenshot egress;
- connector destinations;
- export restrictions.

Fallback MUST preserve egress policy.

## 17.8 Prompt injection

Untrusted content cannot:

- change policy;
- reveal secret;
- install software;
- expand filesystem/network scope;
- alter provider rule;
- approve action;
- redirect protected data.

## 17.9 Least privilege

Workflow pack MUST request only required capabilities.

Permissions SHOULD be reviewable before activation.

## 17.10 Privilege escalation

No silent:

- admin/UAC elevation;
- macOS TCC changes;
- browser profile attachment;
- system-wide install;
- firewall/security changes.

## 17.11 Supply chain

Distributed components MUST support:

- pinned versions;
- checksums;
- signed artifacts;
- SBOM;
- dependency/advisory scan;
- license provenance.

## 17.12 Privacy defaults

Default:

- continuous screen recording off;
- microphone off;
- selective screenshots;
- configurable retention;
- evidence minimization;
- no generic employee surveillance;
- local-only routing supported.

## 17.13 Recording

If recording/capture beyond selective evidence is enabled, deployment MUST define:

- visible indicator;
- purpose;
- retention;
- access;
- applicable notice/consent/legal process.

## 17.14 Cross-tenant isolation

All persisted state, evidence, memory, connector instances, secret refs, and control-plane messages MUST be tenant-scoped.

Project-scoped connections MUST also remain isolated within a tenant. Clones,
worktree copies, shared package caches and global automation visibility cannot
implicitly transfer connection grants or session ownership.

## 17.15 Security incidents

Runtime MUST support:

- local stop;
- device revocation;
- unattended lease revocation;
- update disable/rollback;
- connector revoke where applicable.

## 17.16 Adversarial evals

V1 MUST include:

- prompt injection;
- action mutation after approval;
- destination substitution;
- secret-exfiltration attempt;
- malicious MCP/plugin;
- wrong-window action;
- crash/double-submit;
- local-only cloud fallback attempt;
- cross-tenant access attempt;
- cross-project reference substitution and copied project identity;
- ignored/previously tracked credential files and removed/negated ignore rules;
- shell/plugin bypass of file-tool restrictions and brokered read-only scope;
- vault lock, concurrent refresh/revoke, and no plaintext fallback.

## 17.17 Fail closed

Security-critical parser/evaluator uncertainty MUST fail closed.

Encrypted storage, process separation and package declarations alone do not prove
runtime isolation. Required sandbox/credential mediation that cannot be enforced
must block the component. Document the certified platform boundary; do not claim
security against an unrestricted same-user process or a compromised operating system.
