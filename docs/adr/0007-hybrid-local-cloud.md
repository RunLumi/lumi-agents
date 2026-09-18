# ADR 0007: Hybrid local/cloud architecture

- Status: Accepted
- Date: 2026-09-18

## Decision

Lumi is local-first for authority and employee-machine access, with optional cloud orchestration under tenant policy.

Local owns:

- policy;
- secret resolution;
- native app access;
- local files;
- local-only inference when configured;
- kill switch;
- executor gate;
- sensitive evidence decisions.

Cloud may provide:

- orchestration metadata;
- enterprise policy distribution;
- fleet visibility;
- hosted inference when allowed;
- schedules/events;
- cross-device handoff.

## Rule

Cloud unavailability must not grant more authority.

Local-only workloads must not silently fail over to cloud providers.

Managed unattended execution requires a revocable lease so the organization can stop it remotely.
