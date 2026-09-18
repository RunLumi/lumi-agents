# Workflow Packs

## Definition

A workflow pack is a versioned production automation unit.

It packages the business contract, permissions, execution preferences, postconditions, exception behavior, evidence policy, evals, and economics needed to run a repeatable workflow safely.

## Why packs matter

Customer implementation work must compound.

A deployment that disappears into bespoke code is service revenue.

A deployment that produces a reusable pack is product leverage.

## Suggested structure

```text
packs/quote-to-draft/
  pack.yaml
  README.md
  workflows/
    intake.yaml
  policies/
    default.yaml
  evals/
    scenarios.yaml
    fixtures/
  adapters/
    README.md
```

## Pack metadata

Include:

- name/version;
- business owner;
- supported platforms;
- apps/connectors;
- required capabilities;
- permissions;
- input/output schemas;
- evidence policy;
- privacy classification;
- expected exception path;
- economics fields;
- compatibility matrix.

## Workflow step contract

Each step records:

- preferred execution tier;
- action;
- preconditions;
- postconditions;
- idempotency/retry;
- risk class;
- approval requirement;
- evidence;
- exception route.

## Success

Pack success is machine-verifiable where possible.

A pack should not say "done" because the model reached the last step.

## Economics

Track:

- baseline manual minutes/run;
- residual human minutes/run;
- runs/month;
- loaded labor cost;
- runtime variable cost;
- support cost;
- simple payback;
- implementation engineering hours.

## Reuse targets

For mature same-pack deployments:

- third deployment: <1 engineering day;
- tenth deployment: <4 engineering hours;
- >=70% workflow logic reused.

These are internal product targets, not customer guarantees.

## Hardening path

```text
Work mode success
  -> save trajectory
  -> identify deterministic steps
  -> define policy
  -> add postconditions
  -> add fixtures
  -> add exception handling
  -> repeat evals
  -> certify pack
```

This is how general agent work becomes durable automation.
