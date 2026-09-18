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
