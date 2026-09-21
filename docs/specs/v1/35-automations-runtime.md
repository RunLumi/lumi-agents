# 35. Project automations runtime

Status: **Project engagement store and local scheduler implemented on repair branch**
Date: 2026-09-21

## Model

```text
Automation -> occurrence admission -> project-bound Task -> Run -> evidence/artifact
```

`EngagementStore` is the authoritative desktop Project Automation definition,
revision, occurrence, idempotency and run-history store. Foreground and scheduled
work use the same selected-task runner, policy gate, provider boundary and
verification path. A crash after claim becomes `INTERRUPTED` and is never blindly
replayed.

The runtime supports one-time, interval, daily and weekly schedules with IANA
timezone resolution, DST handling, preview, pause/resume, revision checks, manual
Run now, bounded authorization, overlap refusal and expiry. It is local-only while
the app is running; it does not imply cloud or OS-daemon execution.

## Security and recovery

Unattended authority is scoped to Project, automation revision, selected capability,
browser origins, expiry and run deadline. Shell, Chrome and computer are refused
for unattended work. Browser-origin changes pause affected automations. Provider
recovery is opt-in through the existing macOS Keychain / Windows Credential Manager
broker; session-only mode remains available elsewhere.

The retired desktop `automations.json`/shell-tick path must not coexist with this
store. Generic scheduler library code may remain for other runtime contracts, but
Project engagement has one owner.

## Remaining acceptance

Run restart, missed-window, overlap, duplicate, expired-authority, provider-missing,
browser-unavailable and interrupted-side-effect canaries. Verify resulting Tasks,
evidence, artifacts and history independently of executor return values.
