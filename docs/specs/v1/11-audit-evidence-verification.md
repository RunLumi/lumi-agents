# 11 — Audit, Evidence & Verification v1

Status: Normative  
Supersedes: audit-event-v0.md

## 11.1 Goal

Make Lumi able to explain what happened and prove task success without collecting unnecessary surveillance data.

## 11.2 Audit event

Every material action MUST generate a structured event with:

- event_id;
- timestamp;
- tenant_id;
- task_id;
- run_id;
- workflow/version optional;
- step/action IDs;
- principal;
- normalized risk;
- resource/target;
- policy decision/rule;
- approval ref optional;
- executor tier/adapter/version;
- execution result;
- verification result;
- failure category optional;
- evidence refs.

## 11.3 Event immutability

Audit events SHOULD be append-only.

Corrections SHOULD create follow-up events rather than overwrite history.

## 11.4 Evidence types

Preferred evidence order:

1. structured remote/local state;
2. resource ID/reference;
3. checksum/hash;
4. diff/before-after fields;
5. log excerpt;
6. selective screenshot;
7. full screenshot only when necessary.

## 11.5 Evidence privacy

Evidence MUST support:

- redaction;
- tenant-scoped access;
- retention/deletion;
- sensitivity labels.

Continuous screen recording is not default evidence.

## 11.6 Postcondition

Postcondition is a machine-checkable or human-checkable claim required after action.

Examples:

- record status == APPROVED;
- draft email exists;
- file checksum equals expected;
- remote total reconciles;
- API record version advanced.

## 11.7 Verification statuses

- PASSED
- FAILED
- AMBIGUOUS
- NOT_REQUIRED

## 11.8 Success rule

If a required postcondition is FAILED or AMBIGUOUS, task/step MUST NOT be reported as verified successful.

Executor SUCCESS alone is insufficient.

## 11.9 Independent verification

For consequential workflows, verification SHOULD use a different observation path when practical.

Example:

- UI click submits;
- API/DOM query verifies durable record.

## 11.10 Model verification

A model MAY assist verification of unstructured artifacts.

Model-only judgment SHOULD NOT verify high-risk side effects if deterministic evidence exists.

## 11.11 Approval evidence

Approval event MUST preserve link to action digest and approver.

## 11.12 Tamper evidence

Managed deployments SHOULD support tamper-evident event sequencing/signing where practical.

## 11.13 Audit export

Organizations MAY export audit.

Export itself is DATA_EXPORT and MUST follow policy.

## 11.14 Tests

V1 MUST test:

- executor success + verifier fail;
- ambiguous verification;
- approval/action link;
- redacted evidence;
- retention deletion;
- cross-tenant evidence isolation;
- selective screenshot policy.
