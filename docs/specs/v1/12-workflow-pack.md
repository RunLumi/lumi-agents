# 12 — Workflow Pack v1

Status: Normative

## 12.1 Goal

Define a reusable production automation unit that compounds customer implementation learning.

## 12.2 Pack contents

A v1 pack MUST include:

- manifest;
- semantic version;
- owner;
- input/output schema;
- required capabilities;
- permissions/policy;
- step graph;
- execution-tier preferences;
- postconditions;
- exception routes;
- evidence policy;
- eval scenarios;
- compatibility matrix;
- economic baseline fields.

## 12.3 Suggested structure

```text
packs/<pack-name>/
  pack.yaml
  README.md
  workflows/
  policies/
  evals/
    scenarios.yaml
    fixtures/
  adapters/
    README.md
```

## 12.4 Pack manifest

Manifest SHOULD include:

- pack_id;
- version;
- status: experimental|alpha|certified|deprecated;
- supported runtime version range;
- supported OS/apps;
- required connectors;
- required model capabilities;
- privacy classification;
- default budgets.

## 12.5 Inputs/outputs

Pack inputs and outputs MUST be schema-defined.

Unknown required fields SHOULD fail explicitly.

## 12.6 Steps

Each step MUST define:

- step_id;
- purpose;
- action/capability;
- preferred tier;
- allowed fallback tiers;
- preconditions;
- postconditions;
- risk class;
- approval rule;
- retry/idempotency;
- evidence;
- exception route.

## 12.7 Exceptions

Routine path and exception path MUST be explicit.

Exception MAY route to:

- user;
- manager/approver;
- specialist;
- alternate executor;
- retry;
- stop.

## 12.8 Compatibility

Pack MUST declare tested combinations of:

- runtime version;
- OS;
- app/browser version where material;
- connector version;
- provider capability requirements.

## 12.9 Economics

Pack SHOULD track:

- manual minutes/run;
- residual human minutes/run;
- runs/month;
- loaded labor cost;
- runtime variable cost;
- support cost;
- implementation hours;
- payback hypothesis.

## 12.10 Hardening lifecycle

Recommended:

EXPERIMENTAL -> ALPHA -> CERTIFIED -> DEPRECATED

Certification requires spec 21 gates.

## 12.11 Work-to-pack conversion

A successful Work-mode task MAY become pack candidate when:

- task repeats;
- business value clear;
- inputs/outputs stable;
- routine vs exception pattern exists.

Hardening MUST add:

- deterministic steps;
- policy;
- postconditions;
- fixtures;
- evals;
- compatibility;
- exception handling.

## 12.12 Reuse metrics

Track:

- percent shared logic;
- customer-specific engineering hours;
- adapter reuse;
- deployment time.

## 12.13 Pack isolation

Customer-specific secrets/data MUST NOT be embedded in generic pack.

Customization MUST use config/policy/adapters.

## 12.14 Tests

Pack certification MUST include:

- happy path;
- expected exception;
- auth/session failure;
- stale state;
- policy denial;
- approval;
- retry/recovery;
- verifier failure.
