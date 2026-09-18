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


## 24.15 Skill package contents

A Skill MAY bundle:

- instructions;
- examples;
- reference documents;
- deterministic helper scripts;
- assets/templates;
- eval cases;
- compatibility metadata.

These resources MUST resolve relative to the Skill package/root rather than assume one global install path.

Skill scripts/tools remain subject to normal capability, sandbox, and policy controls.

## 24.16 Skill activation

Skill activation MAY be:

- explicit by user/workflow;
- selected by orchestrator;
- context-triggered by model/router.

Activation changes context, not authority.

A model deciding "this Skill applies" MUST NOT gain additional filesystem/network/tool permissions.

## 24.17 Hook distinction

Hook is a separate deterministic extension primitive defined by spec 14.

Do not encode required deterministic lifecycle enforcement as a Skill prompt when a Hook/policy/verifier is the correct primitive.

Examples:

- deterministic pre-action validation -> Hook/policy;
- reusable domain procedure -> Skill;
- delegated specialist reasoning -> Agent;
- production recurring automation -> Workflow Pack.

## 24.18 Independent-review agents

Independent specialist agents MAY be used when diversity measurably reduces correlated misses.

The parent/orchestrator MUST define:

- independent scope;
- aggregation method;
- confidence/quality threshold;
- disagreement behavior;
- cost/budget ceiling;
- final verification.

Consensus alone is not proof.

If multiple reviewers copy the same context/model failure mode, their agreement provides little additional assurance.

## 24.19 Completion review

A subagent or completion reviewer MAY evaluate whether work appears ready to stop.

It MUST NOT replace required deterministic postconditions or human approvals.

## 24.20 Additional tests

V1 SHOULD additionally test:

- Skill activation does not widen tool/capability authority;
- Skill-relative resources resolve across installation roots;
- deterministic Hook denial overrides model-selected Skill;
- multiple independent reviewers disagree -> orchestrator does not fabricate consensus;
- completion reviewer says done while required verifier fails -> task remains unverified.
