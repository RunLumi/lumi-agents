# 14 — MCP, Connectors & Extension SDK v1

Status: Normative

## 14.1 Goal

Allow extensibility without treating third-party tools as trusted authorities.

## 14.2 Integration classes

V1 defines:

- CONNECTOR: business/API integration;
- MCP_SERVER: MCP capability provider;
- EXECUTOR_ADAPTER: browser/native/app execution engine;
- MODEL_ADAPTER: model provider;
- ARTIFACT_ADAPTER: artifact generator/validator.

## 14.3 Integration manifest

Every extension MUST declare:

- id;
- version;
- type;
- origin;
- license;
- capabilities;
- side effects;
- network destinations;
- filesystem access;
- secret references required;
- data categories accessed;
- platform requirements;
- update source.

## 14.4 Trust

Extension output is lower-trust input.

Extension MUST NOT:

- grant capabilities;
- modify policy;
- read arbitrary secrets;
- bypass audit;
- directly approve actions.

## 14.5 Secret scoping

Extension gets only required credential refs.

A connector needing CRM token MUST NOT receive email/browser secrets.

## 14.6 Side effects

Side-effecting connector/MCP operations MUST normalize to ActionProposal and pass policy.

## 14.7 MCP tool import

MCP tool metadata MAY seed capability mapping.

Lumi SHOULD require admin/user confirmation before enabling newly discovered side-effecting tools.

## 14.8 Network egress

Extension manifest SHOULD declare expected domains/endpoints.

Unexpected destination MAY be denied by policy.

## 14.9 Version pinning

Production integration SHOULD pin exact compatible version/range.

Auto-update MUST NOT silently introduce new capabilities.

## 14.10 Extension isolation

Where feasible, untrusted extensions SHOULD run in lower-privilege process/sandbox.

## 14.11 SDK contracts

Public SDK SHOULD expose:

- protocol types;
- action/observation types;
- capability registration;
- health;
- cancellation;
- normalized errors;
- evidence refs.

SDK SHOULD NOT expose internal policy bypass hooks.

## 14.12 License policy

Extension loader MUST surface license/provenance metadata.

Enterprise policy MAY block certain licenses/origins.

## 14.13 Tests

V1 MUST test:

- side-effecting MCP call passes policy;
- extension cannot access unrelated secret;
- version adds capability -> requires review;
- unexpected network target denied;
- extension crash isolated;
- cross-tenant connector isolation.


## 14.14 Canonical extension vocabulary

Lumi uses these distinct extension concepts:

### TOOL

A typed callable capability.

Tool existence does not grant authority.

### SKILL

Reusable procedural/model context.

A Skill MAY bundle:

- instructions;
- examples;
- references;
- scripts;
- assets;
- validation/eval material.

Skill activation is context, not authorization.

### AGENT

A bounded delegated reasoning/execution role.

Agent definition MUST declare:

- purpose;
- context scope;
- required model capabilities;
- allowed tools/skills;
- budget/deadline;
- output contract.

Agent MUST NOT inherit all parent secrets/capabilities by default.

### HOOK

A deterministic lifecycle handler.

Candidate hook points include:

- TaskStarted;
- BeforePlan;
- BeforeAction;
- AfterAction;
- BeforeVerification;
- ApprovalRequested;
- ApprovalResolved;
- TaskPaused;
- TaskCompleted;
- TaskFailed;
- ContextCompacting.

Hook MAY validate, enrich, log, narrow, deny, or schedule policy-authorized follow-up work.

Hook MUST NOT widen policy or grant itself capability.

### COMMAND

An explicit user-invoked operation.

Command maps to normal Lumi protocol/policy. It is not a privileged backdoor.

### WORKFLOW_PACK

A production automation package with explicit policy, postconditions, exceptions, evidence, evals, compatibility, and economics.

A Workflow Pack MAY compose Tools, Skills, Agents, Hooks, and Connectors.

## 14.15 Extension manifest additions

In addition to section 14.3, extensible packages SHOULD declare:

- extension kind;
- minimum runtime/protocol;
- hooks;
- tools;
- skills;
- agents;
- commands;
- supported platforms;
- signing/provenance metadata where available.

Paths/resources SHOULD resolve relative to the extension root rather than hardcoded install locations.

## 14.16 Organization policy ceiling

Organization-managed policy MAY restrict:

- extension origins;
- extension signing requirements;
- extension kinds;
- hooks;
- network/filesystem scope;
- MCP servers;
- marketplaces/catalogs;
- dangerous/unrestricted modes.

Project/user extensions MUST NOT widen organization policy.

## 14.17 Public marketplace gate

Do not ship a broad public extension marketplace until the runtime has:

- package provenance/signature verification;
- enforceable permission declarations;
- update-review policy;
- uninstall/revocation;
- license metadata;
- dependency/artifact provenance;
- malicious-extension eval corpus;
- organization allow/deny policy.

V1 SHOULD prioritize first-party and private organization extensions.

## 14.18 Hook safety

Hook execution MUST have:

- timeout/deadline;
- bounded input/output schema;
- failure semantics;
- provenance;
- applicable capability scope.

Security-critical deny/fail-closed logic belongs in deterministic policy, not solely in model-prompt hooks.

## 14.19 Additional tests

V1 MUST additionally test:

- Skill/Agent/Hook cannot self-grant capability;
- organization policy blocks disallowed project extension;
- hook timeout does not hang task runtime;
- hook rewrite causing material ActionProposal change triggers re-authorization;
- extension relative paths remain portable across install roots;
- third-party extension crash is isolated where process isolation is configured.
