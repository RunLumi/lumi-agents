# 20 — Versioning, Compatibility & Migrations v1

Status: Normative

## 20.1 Goal

Allow independent evolution of runtime, specs, workflow packs, providers, executors, state, and desktop app without unsafe silent breakage.

## 20.2 Versioned surfaces

Must be versioned:

- persisted protocol schemas;
- workflow pack manifest;
- organization policy;
- audit event;
- model capability contract;
- executor worker protocol;
- control-plane API;
- durable state/checkpoints.

## 20.3 Semantic versioning

Public pack/SDK/runtime contracts SHOULD use semantic versioning.

Breaking schema/behavior change requires major version unless compatibility adapter exists.

## 20.4 Schema envelope

Persisted message MUST include:

- schema_name;
- schema_version.

## 20.5 Compatibility declaration

Runtime SHOULD expose supported ranges for:

- pack schema;
- policy schema;
- worker protocol;
- control-plane API.

## 20.6 Unknown fields

Unknown optional fields MAY be ignored.

Unknown required versions MUST fail explicitly.

## 20.7 Migration

Durable state migration MUST be:

- versioned;
- deterministic;
- tested;
- reversible where practical;
- backed up before destructive transformation.

## 20.8 Workflow migration

If in-flight workflow version changes, runtime MUST NOT silently switch semantics.

Options:

- finish pinned version;
- explicit migration;
- cancel/restart with approval.

## 20.9 Provider changes

Provider/model update MUST NOT require workflow schema change if capability contract unchanged.

## 20.10 Executor changes

Cua/Playwright/native implementation MAY change behind adapter without workflow schema change.

## 20.11 Deprecation

Deprecated contract SHOULD define:

- replacement;
- warning period;
- removal version;
- migration path.

## 20.12 Desktop/runtime compatibility

Desktop app SHOULD refuse or degrade safely when runtime protocol incompatible.

## 20.13 Rollback

Release migration MUST document whether rollback is supported.

If not, release gate requires explicit irreversible migration plan.

## 20.14 Tests

V1 MUST include:

- previous minor schema fixture;
- unknown optional field;
- unsupported major version;
- checkpoint migration;
- workflow version pin;
- worker protocol mismatch;
- downgrade/rollback where supported.


## 20.15 Capability negotiation

Runtime/environment descriptor MUST advertise supported capabilities separately from product version.

Clients MUST NOT infer feature support from version number alone when capabilities can vary by:

- platform;
- build;
- managed policy;
- optional executor/provider;
- staged rollout;
- runtime generation.

Capability examples:

- task.resume.v1;
- task.fork.v1;
- approval.digest.v1;
- browser.semantic.v1;
- native.background_input.v1;
- workflow_pack.v1;
- artifact.docx.v1.

## 20.16 Safe downgrade

When an environment/runtime loses a capability after downgrade or configuration change:

- clients MUST stop offering or invoking that capability;
- cached optimistic state MUST NOT override current environment descriptor;
- persisted data using newer semantics MUST fail explicitly or use a documented compatibility projection;
- consequential behavior MUST NOT silently degrade to a weaker trust boundary.

## 20.17 Independently versioned surfaces

Web, desktop, mobile, local runtime, provider adapters, and control plane MAY upgrade independently.

Protocol changes MUST define:

- producer version;
- consumer compatibility;
- persisted-history compatibility;
- downgrade behavior;
- capability flag if behavior is optional.

## 20.18 Runtime generation

ExecutionEnvironment SHOULD expose a runtime_generation that changes whenever process/runtime identity changes in a way that invalidates local handles.

Generation-bound handles MUST NOT survive restart/rebind by assumption.

## 20.19 Additional tests

V1 MUST additionally include:

- old client/new runtime;
- new client/old runtime;
- runtime downgrade losing a capability;
- stale cached capability after downgrade;
- stale generation-bound handle;
- persisted history containing optional newer fields;
- unsupported consequential feature refuses rather than silently weakening behavior.
