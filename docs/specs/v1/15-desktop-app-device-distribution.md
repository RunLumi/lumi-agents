# 15 — Desktop App, Device Trust & Distribution v1

Status: Normative

## 15.1 Goal

Define employee-facing macOS/Windows app responsibilities without moving policy authority into the UI.

The desktop app is the primary local control surface for ExecutionEnvironments and Folder-as-Project Work mode.

## 15.2 Desktop shell

Tauri is planned v1 desktop shell.

Desktop UI owns:

- onboarding;
- permission status;
- Project open/create/recent UX;
- Project/task navigation;
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

The Project contract is defined by spec 26.

## 15.3 Project entry points

Desktop Work mode MUST expose first-class entry points for:

- Open Folder;
- Open Recent Project;
- Clone Repository;
- New Task in Project;
- Resume Task.

A Project MUST be bound to the ExecutionEnvironment that owns its local roots.

The UI MUST NOT present a remote client's filesystem as if it were the Project's local filesystem.

Opening a folder MUST NOT implicitly grant access to sibling directories, home directory, credential stores, browser profiles, or unrelated repositories.

## 15.4 Project surface

For an opened Project, the desktop app SHOULD provide a coherent surface for:

- files/search;
- Project metadata;
- Git status/branch when available;
- Lumi-produced change set;
- tasks and task history;
- validations;
- artifacts;
- approvals/exceptions;
- evidence;
- relevant command activity;
- current cost/time/budget where available.

The main interaction SHOULD remain goal-oriented delegation.

V1 does not require Lumi to become a full IDE.

The user MAY continue editing the same Project in an external editor. Lumi MUST account for external changes as defined by spec 26.

## 15.5 Device registration

Registration creates device identity bound to tenant.

Device record MUST include trust state and runtime/app versions.

Project metadata MUST reference the owning ExecutionEnvironment/device without copying local secrets or authority to remote surfaces.

## 15.6 Device keys

Managed device SHOULD have cryptographic identity stored using platform-protected facilities.

## 15.7 macOS distribution

External macOS build MUST be:

- code signed;
- notarized;
- stable bundle ID;
- permission onboarding for required TCC capabilities;
- updater signed.

## 15.8 Windows distribution

External Windows build MUST be:

- code signed;
- stable publisher identity;
- installer/update path defined;
- UIA/session limitations documented;
- updater signed.

## 15.9 Permissions UX

App MUST explain:

- requested permission;
- why needed;
- which Project/task/workflow uses it;
- impact if denied.

Permission denial MUST not trigger hidden workaround/escalation.

Folder selection grants only the Project root scope represented by the resulting Project configuration. It does not grant arbitrary disk access.

## 15.10 Update rings

Recommended:

- DEV
- INTERNAL
- CANARY
- STABLE

Device MAY be assigned ring by organization policy.

## 15.11 Update safety

Update metadata/artifacts MUST support:

- signature;
- checksum;
- version;
- minimum compatible protocol;
- rollback target.

## 15.12 Rollback

Stable release process MUST define rollback.

State migrations MUST account for runtime downgrade limitations.

Project metadata, task history, root identifiers, and capability snapshots MUST remain readable or migrate safely across supported upgrades/downgrades.

## 15.13 MDM

Enterprise deployment SHOULD support:

- silent install where policy permits;
- preconfiguration;
- device enrollment;
- update control;
- network/proxy configuration;
- policy distribution.

Managed configuration MAY restrict which roots/projects may be opened or indexed.

## 15.14 Revocation

Managed organization MUST be able to revoke:

- device;
- unattended lease;
- organization policy.

Revocation MUST fail closed for new privileged work.

A revoked ExecutionEnvironment MUST NOT continue admitting new Project mutations.

## 15.15 Local stop

Emergency stop MUST:

- be visible/reachable;
- stop new actions promptly;
- preserve state/evidence;
- not silently resume without user/policy.

Stopping execution MUST NOT corrupt Project metadata or silently discard the current Lumi change set.

## 15.16 Privacy indicator

App SHOULD visibly indicate:

- active task;
- active Project;
- screen capture when occurring;
- microphone capture if ever enabled;
- unattended/background state.

Project mode by itself MUST NOT imply continuous screen capture or continuous file-content upload.

## 15.17 Tests

V1 MUST test:

- install/update/rollback;
- revoked device;
- permission denied/revoked;
- emergency stop;
- corrupt update;
- unsupported OS;
- state compatibility across update;
- Open Folder creates the correct Project identity;
- Open Recent revalidates the same local root;
- missing/moved Project root is surfaced clearly;
- folder selection does not authorize siblings;
- external editor changes are reflected without silent clobber;
- remote supervision cannot substitute a remote local path;
- capability downgrade hides/refuses unsupported Project operations.
