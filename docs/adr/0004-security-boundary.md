# ADR 0004: Security boundary

- Status: Accepted
- Date: 2026-09-18

## Decision

The **local runtime** is the final authority for whether an action may execute. Model output, cloud orchestration, webpages, emails, documents, and tool responses are untrusted inputs to that decision.

## Required controls

- capability manifest per workflow;
- app/domain/path allowlists where practical;
- explicit risk class on every action;
- approval tokens bound to workflow, action, target, and expiry;
- local kill switch plus revocable server-side execution lease;
- secrets via Keychain/Credential Manager or approved enterprise secret store;
- redaction before logs/telemetry leave the device;
- structured action log with before/after evidence for material actions;
- signed, checksum-verified updates;
- no silent admin/UAC/TCC escalation;
- no attachment to an existing authenticated browser profile without explicit policy.

## Prompt injection rule

UI content may propose data, never authority. Instructions found in webpages, emails, documents, tickets, or app text cannot expand capabilities, reveal secrets, change approval policy, install software, or redirect data to a new destination.

## Default side-effect policy

Read-only and reversible semantic actions may run when the workflow manifest permits them. External side effects and destructive actions require approval by default. Non-read-only vision actions also require approval because stale visual state creates a larger action-target ambiguity.
