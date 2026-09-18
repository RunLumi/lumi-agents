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


## 11.15 Durable action phases

For consequential work, audit/state SHOULD allow reconstruction of:

1. normalized intent persisted;
2. policy evaluated;
3. approval requested/resolved when applicable;
4. dispatch started;
5. executor outcome recorded;
6. verification result recorded;
7. final action/task outcome committed.

A durable-intent acknowledgment MUST NOT be presented as successful execution.

This ordering is required for safe recovery after crash/restart.

## 11.16 Executor outcome vs verification

Audit MUST preserve executor outcome separately from verification status.

Executor outcomes:

- DELIVERED;
- REFUSED;
- NO_EFFECT;
- AMBIGUOUS;
- ERROR;
- CANCELLED.

Verification remains:

- PASSED;
- FAILED;
- AMBIGUOUS;
- NOT_REQUIRED.

Examples:

- DELIVERED + FAILED verifier => workflow action not successful.
- REFUSED + NOT_REQUIRED => may be correct if refusal was expected.
- AMBIGUOUS => MUST block blind retry for consequential effects.

## 11.17 Privacy-minimized local history

The baseline local action-history profile SHOULD retain structured operational metadata while excluding raw content not required for accountability.

Do not persist by default:

- plaintext secrets;
- typed text;
- clipboard contents;
- raw tool arguments/results;
- screenshots/video/audio;
- accessibility trees;
- window titles;
- URLs;
- local file/profile paths;
- arbitrary free-form model reasoning.

Evidence policy MAY opt into specific content when necessary, with explicit retention/access rules.

There MUST NOT be a customer-production "debug mode" that silently disables privacy policy.

## 11.18 Additional tests

V1 MUST additionally test:

- crash between intent persistence and dispatch;
- crash after external effect but before finalization;
- executor outcome and verifier status remain distinct;
- baseline local history excludes forbidden content classes;
- explicit evidence policy permits only the selected additional content;
- AMBIGUOUS action cannot be auto-replayed without recovery verification.
