# 15 — Desktop App, Device Trust & Distribution v1

Status: Normative

## 15.1 Goal

Define employee-facing macOS/Windows app responsibilities without moving policy authority into the UI.

## 15.2 Desktop shell

Tauri is planned v1 desktop shell.

Desktop UI owns:

- onboarding;
- permission status;
- task/progress view;
- approvals;
- exception queue;
- audit/evidence viewer;
- provider settings;
- privacy indicators;
- local emergency stop;
- update UX;
- device registration.

UI MUST NOT be policy authority.

## 15.3 Device registration

Registration creates device identity bound to tenant.

Device record MUST include trust state and runtime/app versions.

## 15.4 Device keys

Managed device SHOULD have cryptographic identity stored using platform-protected facilities.

## 15.5 macOS distribution

External macOS build MUST be:

- code signed;
- notarized;
- stable bundle ID;
- permission onboarding for required TCC capabilities;
- updater signed.

## 15.6 Windows distribution

External Windows build MUST be:

- code signed;
- stable publisher identity;
- installer/update path defined;
- UIA/session limitations documented;
- updater signed.

## 15.7 Permissions UX

App MUST explain:

- requested permission;
- why needed;
- which workflow uses it;
- impact if denied.

Permission denial MUST not trigger hidden workaround/escalation.

## 15.8 Update rings

Recommended:

- DEV
- INTERNAL
- CANARY
- STABLE

Device MAY be assigned ring by organization policy.

## 15.9 Update safety

Update metadata/artifacts MUST support:

- signature;
- checksum;
- version;
- minimum compatible protocol;
- rollback target.

## 15.10 Rollback

Stable release process MUST define rollback.

State migrations MUST account for runtime downgrade limitations.

## 15.11 MDM

Enterprise deployment SHOULD support:

- silent install where policy permits;
- preconfiguration;
- device enrollment;
- update control;
- network/proxy configuration;
- policy distribution.

## 15.12 Revocation

Managed organization MUST be able to revoke:

- device;
- unattended lease;
- organization policy.

Revocation MUST fail closed for new privileged work.

## 15.13 Local stop

Emergency stop MUST:

- be visible/reachable;
- stop new actions promptly;
- preserve state/evidence;
- not silently resume without user/policy.

## 15.14 Privacy indicator

App SHOULD visibly indicate:

- active task;
- screen capture when occurring;
- microphone capture if ever enabled;
- unattended/background state.

## 15.15 Tests

V1 MUST test:

- install/update/rollback;
- revoked device;
- permission denied/revoked;
- emergency stop;
- corrupt update;
- unsupported OS;
- state compatibility across update.
