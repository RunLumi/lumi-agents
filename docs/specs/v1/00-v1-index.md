# Lumi Agents Specification Suite v1

Status: **Normative baseline for v1 implementation**  
Version: **1.0**  
Date: **2026-09-18**

## 0.1 Purpose

This directory is the normative v1 specification for Lumi Agents.

The roadmap explains **where the product is going**.  
AGENTS.md explains **how contributors should think and work**.  
This suite defines **what v1 components must mean and how they interoperate**.

## 0.2 Normative language

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

- **MUST / MUST NOT**: required for v1 conformance.
- **SHOULD / SHOULD NOT**: default unless a documented reason exists.
- **MAY**: optional.

## 0.3 V1 product boundary

V1 covers one trusted runtime supporting:

- Work mode for substantial knowledge work;
- Workflow mode for hardened repeatable operations;
- provider-neutral model routing;
- connector/API execution;
- browser semantic execution;
- native desktop execution;
- controlled files/shell/artifact operations;
- durable state and resumability;
- local policy and approval;
- evidence and verification;
- workflow packs;
- scheduling/background triggers;
- extensibility through connectors/MCP;
- macOS and Windows employee-facing deployment;
- evals, observability, and workflow economics.

V1 does **not** require:

- a public marketplace;
- autonomous financial/legal execution without approval;
- Linux desktop parity;
- first-party replacements for all upstream computer-use engines;
- arbitrary always-on employee monitoring;
- fully autonomous self-modifying agents.

## 0.4 Specification map

| # | File | Contract |
|---|---|---|
| 01 | core-domain-model | IDs, principals, tenant boundaries, shared entities |
| 02 | task-run-state-machine | durable lifecycle and resumability |
| 03 | action-observation-protocol | normalized actions and observations |
| 04 | policy-approval-capabilities | authority and approval semantics |
| 05 | execution-router | execution-tier selection and fallback |
| 06 | browser-executor | browser semantic contract |
| 07 | native-desktop-executor | macOS/Windows desktop contract |
| 08 | files-shell-artifacts | local workspace, shell, files, artifacts |
| 09 | model-provider-routing | provider-neutral model capability contract |
| 10 | context-memory-retrieval | context classes and durable memory |
| 11 | audit-evidence-verification | proof, postconditions, audit |
| 12 | workflow-pack | production workflow packaging |
| 13 | scheduler-background-triggers | schedules/events/background execution |
| 14 | mcp-connectors-extension-sdk | connector/plugin capability model |
| 15 | desktop-app-device-distribution | employee app, signing, updates, device trust |
| 16 | evals-observability-economics | quality, SLOs, cost-center metrics |
| 17 | security-privacy-secrets | threat model and privacy rules |
| 18 | error-taxonomy-retry-recovery | canonical failures and recovery |
| 19 | api-control-plane-sync | optional cloud/control-plane boundary |
| 20 | versioning-compatibility-migrations | compatibility rules |
| 21 | release-certification | release/certification gates |
| 22 | v1-definition-of-done | v1 completion checklist |
| 23 | user-experience-handoff | task delegation, approval, progress, takeover |

## 0.5 Global invariants

All v1 components MUST preserve these invariants:

1. Models may propose but do not grant themselves authority.
2. Side effects MUST pass policy before execution.
3. Required approvals MUST bind to the normalized action being approved.
4. Required verification MUST determine success, not model self-report.
5. Secrets MUST NOT be placed in normal prompt/audit content.
6. Workflow logic MUST NOT depend directly on one provider's API schema.
7. Workflow definitions MUST NOT depend directly on Cua/Playwright-specific primitives.
8. Higher-semantic execution tiers SHOULD be preferred over vision/coordinates.
9. Replaying after failure MUST account for possibly completed side effects.
10. Cross-tenant data MUST remain isolated.
11. Data-egress constraints MUST survive provider fallback.
12. Background execution MUST be bounded and revocable.
13. Audit/evidence MUST be minimized enough to avoid becoming employee surveillance.
14. A v1 workflow MUST be measurable at the workflow outcome level.
15. The runtime MUST support cancellation.

## 0.6 Conformance

A component is v1-conformant only if:

- it implements the relevant MUST requirements;
- its external types are compatible with the v1 domain/protocol specs;
- it has contract tests;
- it participates in the required audit/policy/verification flow;
- it declares unsupported capabilities instead of silently degrading.

## 0.7 Source of truth

If this suite conflicts with older v0 specs, **v1 wins**.

If this suite conflicts with an ADR, the conflict MUST be resolved by updating one of them before implementation is merged.
