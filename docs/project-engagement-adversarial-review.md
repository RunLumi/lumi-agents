# Project engagement adversarial review

Status: **in progress; this document is evidence for repair, not a readiness claim**
Date: 2026-09-21
Scope: PR #117 (`feat/project-agent-engagement`) reviewed against current `main`.

## Findings

| Severity | Finding | State | Evidence / required direction |
|---|---|---|---|
| Critical | Current `main` already had Spec 27 project automations backed by `automations.json`, `lumi-scheduler`, and `automations_tick`; PR #117 added `EngagementStore`, another automation schema, another scheduler loop, and another occurrence ledger. | Fixed on repair branch | The legacy desktop automation store, commands, tick path, dependency, and test were removed; Project Automations now use the engagement store and shared task runner. |
| High | PR #117 originally converted `@browser`/`@shell` text found in task prose into backend-selected capabilities. Quoted, pasted, or external content could therefore widen authority. | Fixed on repair branch | Structured `tools` selections are now the only authority input; prose is data. Frontend and Rust regression cases cover pasted/quoted mentions. |
| High | Resume context treated `Success + NotRequired` as verified progress for every action. | Fixed on repair branch | Only `Passed` resumes consequential work; `NotRequired` resumes non-consequential reads because they have no external effect. Runner E2E passes. |
| High | Provider storage code was present but dead: the module was not exported, `lumi-secrets` was not a dependency, startup did not restore, and provider commands did not save/revoke. | Fixed on repair branch | Added platform broker wiring, explicit opt-in persistence, startup restore, revocation, tuple validation, and redacted `ProviderSession` debug output. |
| High | `@chrome` and `@computer` appear in the capability model but are rejected as unavailable at execution. | Open | Keep them visibly unavailable until real adapters, permissions, revocation, and platform canaries exist. Never route them to shell or managed-browser fallbacks. |
| High | Scheduled execution initially depended on the in-memory provider session in PR #117. | Fixed on repair branch | Opt-in macOS Keychain / Windows Credential Manager recovery is wired; OS-specific canaries remain required. |
| Medium | Browser implementation is read-only by design. It does not provide authenticated Chrome, uploads/downloads, interactive mutation, or native computer control. | Open / intentional boundary | Document as unavailable capability states; do not claim the product model is complete until supported adapters exist. |
| Medium | Browser URLs are public HTTPS origin scoped and DNS-pinned in the worker, but the host uses origin allowlists and untrusted observations only. | Partially verified | Existing Rust and worker tests cover local/private address refusal, worker integrity, and untrusted page content. Real Chromium canary remains read-only. |
| Medium | Browser worker, provider, and scheduler readiness are surfaced through separate host checks rather than a single backend capability registry. | Open | Introduce one backend-owned capability catalog consumed by UI snapshots, mention resolution, scheduler admission, policy, and runtime tool construction. |
| Medium | DCO/contribution-origin checks for PR #117 require genuine author sign-offs. | External blocker | Never synthesize sign-offs. Preserve the exact failing check and request maintainer action if the final PR still contains unsigned commits. |

## Repairs already validated

- `cargo test -p lumi-desktop --locked`: focused unit and runner tests pass on the repair branch.
- `cargo test -p lumi-agent --locked`: pending after the next integrated rebase; the agent loop changes are still subject to the full workspace gate.
- Capability prose does not grant tools in the repaired frontend/backend path.
- Provider secret values remain out of ordinary persisted state and `ProviderSession` debug output.
- Browser observations remain explicitly untrusted and cannot authorize later actions.

## Merge decision

PR #117 is **not merge-ready** until the duplicate automation architecture is reconciled against current `main`, the full current-main check matrix is rerun on the final head, and any DCO/contribution-origin blocker is handled by an authorized maintainer. The current repair branch is an engineering continuation, not evidence that authenticated Chrome, native computer use, or a sustained unattended canary exists.
