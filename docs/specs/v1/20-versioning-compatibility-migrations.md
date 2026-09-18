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
