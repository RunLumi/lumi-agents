# Lumi Agents Specification Suite v1

Status: **Normative baseline for v1 implementation**  
Version: **1.0**  
Date: **2026-09-20**

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
- durable Folder-as-Project workspaces for repository/document work;
- safe project-scoped file editing, search, shell/code, Git, task history, and resume;
- parallel delegated Tasks with isolated/conflict-aware writes and shared-resource arbitration;
- local document preview and bounded basic editing with explicit format/preservation limits;
- Workflow mode for hardened repeatable operations;
- provider-neutral model routing;
- connector/API execution;
- browser semantic execution;
- native desktop execution;
- controlled files/shell/artifact operations;
- durable state and resumability;
- local policy and approval;
- evidence and verification;
- one evidence-first Review Queue that separates routine history from actionable attention;
- workflow packs;
- scheduling/background triggers;
- global and project-filtered Automations views over the same records;
- safe Task-to-Automation conversion through a disabled draft and fresh admission;
- project-local Skills/Plugins with reviewed install/activation and pinned run dependencies;
- project-scoped connections backed by protected secret storage, not repository token files;
- extensibility through connectors/MCP;
- macOS and Windows employee-facing deployment;
- evals, observability, and workflow economics;
- remote supervision that preserves ExecutionEnvironment ownership of local state and credentials.

V1 does **not** require:

- a public marketplace;
- a full IDE replacement;
- a full Office suite, macro engine or universal lossless Office round-trip guarantee;
- whole-disk indexing;
- a second global scheduler or global super-project;
- a custom project-local credential vault or automatic credential synchronization;
- universal foreign plugin-runtime compatibility;
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
| 13 | scheduler-background-triggers | Project-native Automations, schedules/events, run history, review delivery |
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
| 24 | skills-subagents | reusable skills and bounded subagent orchestration |
| 25 | v1-implementation-order | implementation dependency graph and sequencing |
| 26 | project-workspace-folder-as-project | durable Project, folder roots, files, Git, shell, indexing, task binding |
| 27 | [global-and-project-automations](27-global-and-project-automations.md) | global menu, project view, shared CRUD, read state, resource concurrency |
| 28 | [project-skills-plugins](28-project-skills-plugins.md) | project directories, install/activation, compatibility, locks, update/revocation |
| 29 | [project-connections-secrets](29-project-connections-secrets.md) | portable requirements, protected credentials, broker isolation, Git hygiene, migration |
| 30 | [document-workspace-preview-edit](30-document-workspace-preview-edit.md) | local multi-format preview/basic edit, React/Tauri boundary, preservation, safe save, OSS qualification |
| 31 | [delegated-work-product-contract](31-delegated-work-product-contract.md) | end-to-end Work → Review → Automate contract, parallel delegation, attention efficiency, remote supervision, market-proof gates |

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
16. A Project root is a default filesystem authority boundary, not permission to access sibling/home/system paths.
17. Project/repository content may guide work but MUST NOT widen policy, secrets, filesystem, network, provider, or external-action authority.
18. Active Tasks MUST remain bound to their durable Project/ExecutionEnvironment identity across resume and remote supervision.
19. Pre-existing user changes MUST NOT be silently overwritten or claimed as agent-generated work.
20. An Automation MUST NOT gain authority from its prompt, Skill, repository changes, trigger payload, retry, or reroute.
21. Every admitted Automation run MUST create durable run history even when its semantic outcome is NO_ACTION/NO_ALERT or delivery is intentionally silent.
22. Automation delivery success and task execution success MUST remain separate facts.
23. Global and project Automations views MUST use the same identities and authorization; read markers never approve actions or resolve incidents.
24. Installing a Skill/Plugin MUST NOT imply activation, execution permission, authentication or cross-project availability.
25. Credentials MUST be protected outside the project working tree; portable requirements and opaque references are not secret grants, and .gitignore is not a security boundary.
26. Copied project IDs, package caches, connection references and worktrees MUST NOT transfer credential authority.
27. Document preview MUST NOT execute embedded code, gain host authority or silently send content to external services.
28. Document saves MUST protect the original and concurrent edits; preview success, format preservation and recalculated business results are separate claims.
29. Parallel Tasks MUST NOT silently overwrite one another or pre-existing human work; concurrency never widens authority.
30. Routine verified Automation outcomes SHOULD remain quiet while actionable attention reaches one durable Review Queue.
31. Converting a Task to an Automation MUST NOT copy transient approvals, credentials, leases, sessions, or stale authority.
32. Remote supervision MUST preserve ExecutionEnvironment ownership of local files, credentials, sessions, permissions, and executor state.
33. Product success MUST be measured on verified useful work and human attention, not agent activity volume.

## 0.6 Conformance

A component is v1-conformant only if:

- it implements the relevant MUST requirements;
- its external types are compatible with the v1 domain/protocol specs;
- it has contract tests;
- it participates in the required audit/policy/verification flow;
- it declares unsupported capabilities instead of silently degrading.

Specifications describe targets, not evidence that features have shipped.

## 0.7 Source of truth

If this suite conflicts with older v0 specs, **v1 wins**.

If this suite conflicts with an ADR, the conflict MUST be resolved by updating one of them before implementation is merged.

Spec 27 specializes the UI requirements in Specs 13 and 23: a global Automations
entry and project-filtered entry are required, not alternative implementations.
Spec 28 specializes installation/loading in Specs 14 and 24. Spec 29 specializes
project credentials in Specs 17 and 26: being inside an authorized root does not
make protected local state or credential material readable to an agent. These
specializations preserve the existing scheduler and policy contracts.

Spec 30 specializes Files/Artifacts/Review preview and bounded edits in Specs
08/23/26. Its renderers do not acquire file/secret authority, and its format
support matrix does not override artifact verification or safe-save requirements.

Spec 31 is the cross-cutting delegated-work product contract. It does not replace
the component specs it cites and introduces no second orchestrator, scheduler,
permission source, or implementation claim. Where Spec 31 requires a product
behavior, the underlying component MUST still satisfy its owning spec.
