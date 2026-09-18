# 24 — Skills & Subagents v1

Status: Normative

## 24.1 Goal

Enable reusable task expertise and bounded parallelism without turning the system into an uncontrolled "agent swarm."

## 24.2 Skill

A Skill is a reusable task-oriented instruction/capability bundle.

Skill MAY include:

- purpose;
- required tools/capabilities;
- instructions;
- input/output schema;
- examples;
- validation;
- references.

Skill MUST NOT grant authority beyond task/workflow policy.

## 24.3 Skill provenance

Skill SHOULD record:

- skill_id;
- version;
- author/source;
- license/provenance;
- supported modes;
- required capabilities;
- risk notes.

## 24.4 Skill loading

Skill content is instruction context, not authorization.

External/community skill MUST be treated as untrusted until approved.

## 24.5 Skill versioning

Workflow pack SHOULD pin compatible skill version/range.

Breaking behavior change requires major version.

## 24.6 Subagent default

Default architecture is one strong orchestrator plus deterministic tools.

Subagents are optional.

## 24.7 Allowed subagent reasons

Use subagents when measured benefit exists for:

- independent parallel research;
- isolated browser sessions;
- specialist code review;
- independent verification;
- bounded artifact sections;
- partitioned data analysis.

## 24.8 Bad reasons

Do not create subagents for:

- decorative multi-agent demos;
- "consensus" as substitute for deterministic verification;
- duplicating identical context;
- hiding unclear orchestration architecture.

## 24.9 Subagent scope

Each subagent MUST receive:

- explicit goal;
- scoped context;
- scoped capabilities;
- budget;
- deadline;
- output schema.

It MUST NOT inherit all parent secrets/permissions automatically.

## 24.10 Authority

Subagent proposed side effects still pass parent/local policy.

Subagent cannot approve its own consequential action.

## 24.11 Communication

Subagent output SHOULD be structured.

Parent orchestrator remains responsible for integrating results and final verification.

## 24.12 Isolation

Parallel browser/native work MUST avoid conflicting control of same user session unless explicitly coordinated.

## 24.13 Economics

Subagent fan-out SHOULD be measured for:

- latency improvement;
- verified quality;
- cost;
- failure rate.

If parallelism raises cost without outcome improvement, remove it.

## 24.14 Tests

V1 SHOULD test:

- scoped capability inheritance;
- budget enforcement;
- cancellation propagation;
- subagent failure isolation;
- independent verification disagreement;
- no self-approval.
