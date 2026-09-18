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


## Layered authority

Effective authority is the intersection of independent ceilings:

```text
platform hard invariants
  ∩ organization managed policy
  ∩ user policy
  ∩ workflow/pack manifest
  ∩ task temporary grant
= effective authority
```

Lower layers may narrow upper layers.

They may not widen them.

An "autonomy mode" is separate from this authority ceiling. Changing from supervised to bounded unattended execution does not grant a capability that policy denies.

## Autonomy modes

Lumi should eventually expose a small number of understandable modes, while keeping business policy separate.

### Supervised

Routine safe work can proceed; material effects request approval according to policy.

### Bounded unattended

The task may run without interactive approval only inside a reviewed manifest defining tools/resources/actions/time/budget.

Anything outside the manifest is denied or routed to an exception.

### Unrestricted development/testing

Only for explicitly acknowledged disposable/trusted environments and still subject to non-bypassable platform/organization ceilings.

Do not expose unrestricted mode as a one-click way around policy.

## Least-additional-permission escalation

When an executor needs more access, prefer the narrowest temporary grant that can complete the action.

Examples:

- one hostname instead of all network;
- one file path instead of a filesystem root;
- one app/window/account instead of global computer control;
- one business action/destination/value range instead of session-wide approval.

Broad escalation should be rare and visible.

## Protected authenticated sessions

Attaching to an existing authenticated browser/app session is a distinct sensitive capability.

Approval/policy should bind to concrete context where possible:

- provider/browser profile identity;
- process/runtime generation;
- account/workspace;
- allowed origins;
- target application.

Before a protected mutation, revalidate mutable context such as:

- current origin;
- active account;
- customer/tenant;
- payment destination;
- file identity;
- process/window identity.

Authorization at task start is not sufficient evidence that the target remains the same.

## External observation authority

Every externally supplied observation or event should carry provenance and an authority class.

Examples:

- USER
- ORGANIZATION_POLICY
- CONNECTOR_EVENT
- TOOL_RESULT
- AGENT_MESSAGE
- WEB_CONTENT
- DOCUMENT_CONTENT

Only trusted authority classes may steer policy-relevant intent.

Untrusted content can change facts/observations, not permissions.

## Exact refusal over unsafe fallback

When the runtime can prove that a requested delivery path is unsupported or unsafe, it should return a stable structured refusal before mutation.

Do not automatically escalate:

- background -> foreground;
- semantic -> global pointer;
- isolated browser -> existing authenticated profile;
- sandboxed shell -> host shell;
- narrow network -> arbitrary egress.

Fallback across a materially different trust boundary requires explicit policy/approval.

## Fail-closed parser agreement

Security validation and downstream execution must agree on the same normalized target.

High-risk boundaries include:

- URLs/origins;
- filesystem paths;
- executable/argv parsing;
- connector identifiers;
- account/workspace identifiers;
- manifest scopes.

A validator that accepts one interpretation while the executor acts on another is a policy bypass.
