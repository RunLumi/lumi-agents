# Security Model

## Security objective

A compromised model, malicious webpage, poisoned document, plugin, or upstream driver must not gain more authority than the workflow policy grants.

## Threats

- prompt injection;
- credential leakage;
- wrong-target actions;
- destructive actions;
- data exfiltration;
- cross-tenant leakage;
- compromised dependencies;
- malicious update channel;
- privilege escalation;
- stale UI state;
- replayed side effects after crash;
- over-collection of employee data.

## Local policy

Policy is deny-by-default for unspecified side effects.

Canonical business risk classes:

- READ
- LOCAL_WRITE
- EXTERNAL_WRITE
- COMMUNICATION
- DATA_EXPORT
- CREDENTIAL
- FINANCIAL
- LEGAL_CONSENT
- DESTRUCTIVE
- ADMIN

Policy evaluates business effects, not clicks.

## Approval binding

Approval must bind to:

- normalized action digest;
- workflow;
- principal;
- target/resource;
- destination;
- material value when relevant;
- parameters;
- expiry.

Material action changes invalidate approval.

## Prompt injection

Untrusted content cannot:

- expand permission;
- modify policy;
- reveal credentials;
- install software;
- alter provider/data-egress rules;
- authorize payment/legal/destructive actions;
- redirect protected data.

## Secrets

Secrets use references and are resolved at the executor boundary.

Preferred storage:

- macOS Keychain;
- Windows protected credential facilities;
- approved enterprise secret store.

Never expose raw secrets to normal model context or logs.

## Side effects and idempotency

Every consequential action needs:

- action ID;
- idempotency key where supported;
- precondition;
- postcondition;
- retry classification;
- ambiguity handling.

After crash, verify whether a side effect already happened before retrying.

## Evidence

Prefer:

- structured before/after state;
- resource IDs;
- hashes;
- policy decision;
- approval ID;
- verifier result;
- selective screenshots.

Do not continuously record screens by default.

## Kill controls

Required:

- local emergency stop;
- per-task cancel;
- provider budget;
- action budget;
- remote device/workflow revocation for managed deployments.

## Updates

Release artifacts require:

- signing;
- checksums;
- SBOM;
- provenance;
- rollback path;
- canary rings.

A compromised updater is a catastrophic threat and is treated accordingly.

## Security evals

Maintain adversarial tests for:

- prompt injection;
- policy bypass;
- approval substitution;
- destination swapping;
- secret leakage;
- stale target;
- double-submit after resume;
- plugin/MCP overreach;
- image-based malicious instructions;
- data-egress violations.
