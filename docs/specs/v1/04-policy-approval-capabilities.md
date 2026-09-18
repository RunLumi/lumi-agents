# 04 — Policy, Approval & Capability Model v1

Status: Normative

## 4.1 Goal

Define who may request actions, what they may attempt, what policy permits, and when human approval is required.

## 4.2 Separation of concerns

Capability answers: "May this principal request this class of operation?"

Policy answers: "May this normalized action execute under current conditions?"

Approval answers: "Has an authorized human approved this exact action or bounded class?"

These MUST remain distinct.

## 4.3 Capability grant

A capability grant MUST include:

- principal;
- capability name;
- resource scope;
- tenant scope;
- optional target restrictions;
- expiry optional;
- source of grant.

Capabilities SHOULD be least-privilege.

## 4.4 Policy decision

Canonical decisions:

- ALLOW
- DENY
- REQUIRE_APPROVAL

Policy result SHOULD include:

- rule_id;
- reason code;
- human-readable explanation;
- required approval role if any;
- constraints;
- expiry.

## 4.5 Fail closed

Policy MUST fail closed when:

- policy unavailable;
- policy signature invalid;
- unknown risk class;
- unknown consequential capability;
- action malformed;
- tenant mismatch;
- approval invalid;
- device revoked.

## 4.6 Approval object

Approval MUST include:

- approval_id;
- tenant_id;
- approver principal;
- action_digest;
- issued_at;
- expires_at;
- optional bounded constraints;
- optional single-use marker.

## 4.7 Action digest

Digest MUST cover all material action fields.

Digest algorithm and canonical serialization MUST be versioned.

## 4.8 Default high-risk policy

V1 default policy SHOULD require approval for:

- FINANCIAL;
- LEGAL_CONSENT;
- DESTRUCTIVE;
- ADMIN;
- sensitive DATA_EXPORT;
- external COMMUNICATION unless narrowly pre-authorized;
- non-read-only vision/coordinate actions until workflow hardened.

## 4.9 Pre-authorization

Organizations MAY pre-authorize narrow routine actions.

Pre-authorization MUST specify:

- capability;
- resource scope;
- target/destination restrictions;
- value/volume limits if relevant;
- schedule/time restrictions optional;
- workflow/version restrictions;
- expiry/review date.

## 4.10 Delegation

A user MAY delegate approval authority to an organization role.

Delegation MUST be auditable and tenant-scoped.

## 4.11 Policy layers

Effective policy MAY be composed from:

1. product hard safety rules;
2. organization policy;
3. workflow policy;
4. user/session policy.

More specific policy MUST NOT weaken non-overridable product hard safety rules.

## 4.12 Content cannot modify policy

Web/email/document/UI content MUST NOT create or expand capability grants.

## 4.13 Device policy

Policy MAY restrict:

- allowed devices;
- managed-only execution;
- OS versions;
- app versions;
- provider/data region;
- network destinations;
- local-only mode.

## 4.14 Approval UX requirements

Approval prompt MUST show:

- what will happen;
- target/destination;
- material values;
- why approval is required;
- relevant evidence;
- whether action is reversible;
- expiry.

Avoid approval fatigue by hardening narrow routine actions rather than batching unrelated approvals.

## 4.15 Tests

V1 MUST test:

- action mutation invalidates approval;
- expired approval rejected;
- cross-tenant approval rejected;
- revoked device denied;
- missing policy fails closed;
- content injection cannot change policy;
- pre-authorized bounded action allowed while out-of-bound variant requires approval/denial.
