# Extension Model

## Purpose

Lumi needs a precise vocabulary for extensibility so "plugin", "agent", "skill", "tool", and "workflow" do not collapse into one unsafe abstraction.

## Canonical extension types

### Tool

A callable capability with typed input/output.

Examples:

- read a CRM record;
- search files;
- navigate a browser;
- create a spreadsheet artifact.

A tool may have side effects and therefore does not imply authorization.

### Skill

Reusable procedural knowledge and context for a model.

A Skill may include:

- metadata describing when it applies;
- instructions;
- examples;
- reference docs;
- helper scripts;
- eval cases.

Skill activation may be model-driven or explicit.

A Skill does not grant permission.

### Agent

A bounded delegated reasoning/execution role.

An Agent definition declares:

- purpose;
- model capability requirements;
- available tools/skills;
- context scope;
- budget;
- output contract.

Subagents never inherit all parent authority automatically.

### Hook

A deterministic handler for runtime lifecycle events.

Candidate events:

- TaskStarted
- BeforePlan
- BeforeAction
- AfterAction
- BeforeVerification
- TaskCompleted
- TaskFailed
- TaskPaused
- ApprovalRequested
- ApprovalResolved
- ContextCompacting

Hooks may:

- validate;
- enrich context;
- log;
- emit metrics;
- narrow/deny;
- schedule safe follow-up work if policy allows.

Hooks may not widen policy ceilings.

### Connector / MCP server

An external capability provider.

It must declare:

- provenance/version;
- network destinations;
- tools;
- side effects;
- filesystem scope;
- secret needs;
- license.

Connector output is lower-trust input.

### Command

An explicit user-invoked operation such as:

- run workflow;
- review artifact;
- pause task;
- connect account.

Commands map to normal Lumi protocol actions. They are not a privileged backdoor.

### Workflow Pack

A versioned production automation product with:

- schema;
- permissions;
- action semantics;
- postconditions;
- exception paths;
- evidence;
- evals;
- compatibility matrix;
- economics.

A Workflow Pack may reference Tools, Skills, Agents, Hooks, and Connectors.

## Extension manifest

A future extension manifest should declare at least:

```yaml
name:
version:
kind:
license:
source:
capabilities:
network:
filesystem:
secrets:
hooks:
tools:
skills:
agents:
platforms:
minimum_runtime:
```

## Authority rule

```text
extension capability
∩ organization policy
∩ user policy
∩ workflow policy
∩ task temporary grant
= effective authority
```

An extension cannot self-authorize.

## Portability

Extensions should use a runtime-provided root/path reference rather than hardcoded installation locations.

The package format should remain declarative enough that a Skill/Tool/Hook package does not require linking against private Rust internals.

## Marketplace posture

Do not build a public marketplace until:

- extension signatures/provenance exist;
- permission declarations are enforceable;
- update policy exists;
- dependency/license metadata exists;
- uninstall/revocation is reliable;
- malicious-extension evals exist.

Start with signed first-party/private organization extensions.
