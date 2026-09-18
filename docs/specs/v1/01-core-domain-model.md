# 01 — Core Domain Model v1

Status: Normative

## 1.1 Goals

Define the stable vocabulary shared by task orchestration, providers, executors, policy, audit, workflow packs, and control-plane sync.

## 1.2 Identity types

All externally persisted IDs MUST be opaque strings.

Required IDs:

- tenant_id
- organization_id
- user_id
- device_id
- task_id
- run_id
- workflow_id
- workflow_version
- step_id
- action_id
- approval_id
- evidence_id
- artifact_id
- provider_request_id
- connector_instance_id

IDs MUST NOT encode secrets.

IDs SHOULD be globally unique.

## 1.3 Tenant

A Tenant is the security and data-isolation boundary.

A tenant MUST define:

- tenant_id;
- data-egress policy;
- provider allowlist;
- retention policy;
- connector registrations;
- organization policy version;
- allowed device classes;
- audit destination.

Cross-tenant reads/writes MUST be denied unless an explicitly designed administrative capability exists.

## 1.4 Principal

A Principal represents who or what is requesting authority.

Principal kinds:

- USER
- WORKFLOW
- SCHEDULE
- SERVICE
- ADMIN
- SYSTEM

Principal MUST include:

- principal_id;
- tenant_id;
- kind;
- authenticated_at;
- authentication_strength where relevant.

Model/provider identity is not itself an authorization principal.

## 1.5 Device

Device represents a managed or unmanaged local execution host.

Required fields:

- device_id;
- tenant_id;
- platform: MACOS | WINDOWS | LINUX_CORE;
- architecture;
- runtime_version;
- app_version;
- registered_at;
- trust_state;
- last_seen_at;
- policy_version;
- update_ring.

Trust states:

- UNREGISTERED
- REGISTERED
- TRUSTED
- REVOKED

A REVOKED device MUST NOT accept new unattended work.

## 1.6 Task

Task represents a user/business goal.

Required fields:

- task_id;
- tenant_id;
- principal;
- mode: WORK | WORKFLOW;
- goal;
- created_at;
- deadline optional;
- budget;
- privacy_constraints;
- status;
- requested_outputs.

Task MUST NOT directly contain raw provider credentials.

## 1.7 Run

Run is one execution attempt of a task.

Required fields:

- run_id;
- task_id;
- runtime_version;
- workflow_version optional;
- selected_provider(s);
- started_at;
- ended_at optional;
- state;
- budgets_consumed;
- failure optional.

A task MAY have multiple runs.

## 1.8 Workflow

Workflow is a reusable versioned automation contract.

Required fields:

- workflow_id;
- semantic version;
- owner;
- input schema;
- output schema;
- capability requirements;
- policy requirements;
- step graph;
- verification requirements;
- evidence policy;
- compatibility matrix.

## 1.9 Resource

Resource is a policy-addressable business/technical object.

Examples:

- file
- CRM quote
- email draft
- invoice
- browser origin
- local app
- database record
- external destination

Resource MUST have:

- resource_type;
- canonical identifier;
- tenant scope;
- optional sensitivity label.

## 1.10 Capability

Capability is permission to attempt a category of action.

Examples:

- browser.read
- browser.write
- files.read
- files.write
- crm.quote.update
- email.draft.create
- email.send
- shell.execute
- desktop.interact

Capability does not imply policy authorization for every target.

## 1.11 Artifact

Artifact is a user-visible output.

Required fields:

- artifact_id;
- task_id/run_id;
- type;
- location/ref;
- provenance;
- validation status;
- review/publication state;
- checksum when applicable.

## 1.12 Evidence

Evidence is minimal data used to explain or verify an action/outcome.

Evidence MUST be tenant-scoped and SHOULD be immutable after creation except for retention/deletion metadata.

## 1.13 Budget

Budget MAY include:

- wall-clock deadline;
- maximum model cost;
- maximum action count;
- maximum retries;
- maximum vision actions;
- maximum external writes.

Budget exhaustion MUST produce a controlled stop/exception.

## 1.14 Time

Persisted timestamps MUST use UTC RFC3339.

Local timezone MAY be stored as display/user context but not as the canonical persisted instant.

## 1.15 Serialization

Persisted protocol messages MUST include:

- schema_name;
- schema_version.

Unknown required enum values MUST fail explicitly rather than being silently coerced.
