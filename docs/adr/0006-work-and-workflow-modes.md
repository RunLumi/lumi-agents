# ADR 0006: Work mode and Workflow mode share one runtime

- Status: Accepted
- Date: 2026-09-18

## Decision

Lumi Agents supports two product modes on one core.

### Work mode

General-purpose substantial work:

- research;
- browser;
- files;
- shell/code;
- connectors;
- artifact creation;
- long-running tasks.

### Workflow mode

Hardened recurring operations with:

- explicit schema;
- permissions;
- postconditions;
- exception handling;
- evals;
- economics;
- compatibility matrix.

## Why

General agent work is ideal for discovery and flexible tasks.

Recurring business automation needs stronger contracts.

Sharing one runtime lets successful Work-mode trajectories become candidates for hardening into Workflow packs.

## Rule

Do not weaken Workflow-mode guarantees to make Work mode more autonomous.

Do not force exploratory Work-mode tasks into rigid packs prematurely.
