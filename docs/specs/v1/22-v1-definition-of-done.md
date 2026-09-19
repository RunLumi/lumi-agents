# 22 — Lumi Agents v1 Definition of Done

Status: Normative release target

## 22.1 Product promise

V1 is complete when Lumi can reliably perform substantial real work and hardened repeatable workflows across browser and desktop environments while preserving local authority, provider neutrality, verification, evidence, human exception handling, and durable Project context.

For Work mode, a user MUST be able to open a real folder/repository as a Project, delegate work against it, inspect what changed, and resume later without losing scope or safety.

## 22.2 Required platform capabilities

V1 MUST have:

- Work mode task runner;
- durable Folder-as-Project support per spec 26;
- Project-scoped file search/read/create/edit/move/delete;
- Project-bound shell/code execution;
- Git inspection and bounded local Git operations;
- Project task history/change-set visibility;
- Workflow mode workflow-pack runner;
- durable task/run state;
- checkpoint/resume;
- cancellation;
- local policy;
- scoped approvals;
- audit/evidence;
- postcondition verification;
- connector/API execution contract;
- Playwright browser executor;
- macOS/Windows native executor through Lumi adapter;
- controlled files;
- sandboxed/bounded shell execution;
- artifact generation/validation;
- provider-neutral model router;
- at least OpenAI, Anthropic, Gemini plus one local/OpenAI-compatible path;
- context/memory separation;
- Project-scoped context/memory boundaries;
- background schedule/event support;
- extension/connector manifest;
- employee desktop shell;
- signed update path;
- device registration/revocation;
- eval/observability/economics.

## 22.3 Required safety

V1 MUST demonstrate:

- zero known policy bypass in release corpus;
- action mutation invalidates approval;
- cross-tenant isolation;
- Project root isolation;
- traversal/symlink/junction escape refusal;
- repository/project instructions cannot widen authority;
- pre-existing user changes are not silently clobbered;
- local-only provider policy enforced;
- prompt-injection suite;
- secret isolation;
- crash/double-submit recovery;
- local emergency stop;
- device/lease revocation;
- signed external builds.

## 22.4 Required workflows

Before declaring v1:

- at least 3 economically meaningful hardened workflow packs;
- each with >=100 representative canary runs;
- >=95% verified completion;
- 0 unauthorized side effects;
- measured baseline/residual human work;
- exception path;
- compatibility matrix.

At least one workflow SHOULD require native desktop automation, otherwise desktop layer value is not proven.

## 22.5 Work mode gate

Work mode MUST demonstrate representative tasks involving at least:

- opening an existing local folder/repository as a durable Project;
- reopening the Project after application/runtime restart;
- discovering Project structure/instructions safely;
- searching and reading Project files;
- creating and editing Project files;
- preserving pre-existing user changes;
- showing an inspectable Lumi change set;
- Git status/diff and at least one bounded local Git operation when Git is present;
- bounded Project-local shell/code execution;
- project-relevant validation such as test/build/lint/typecheck;
- browser research;
- artifact generation;
- one connector/API;
- one long-running/resumable Project-bound task.

The gate MUST include at least one task that performs a multi-file change in a dirty working tree without silently overwriting unrelated user edits.

Results MUST include inspectable artifacts/evidence and honest validation state.

## 22.6 Provider neutrality gate

Same representative workflow MUST run through at least two model providers without business-logic changes.

One local/OpenAI-compatible path MUST pass core contract tests.

A Project's file/Git/shell semantics MUST NOT depend on a specific model provider.

## 22.7 Recovery gate

Demonstrate:

- app restart;
- Project reopen;
- Project root temporarily unavailable;
- external file modification during a paused task;
- network interruption;
- provider failure;
- approval delay;
- browser worker crash;
- native executor failure;
- ambiguous side effect.

No duplicate consequential side effect.

No silent overwrite of externally modified Project files.

## 22.8 Distribution gate

macOS:
- signed;
- notarized;
- updater/rollback tested.

Windows:
- code-signed;
- installer/updater/rollback tested.

## 22.9 Documentation gate

Must be current:

- AGENTS.md;
- roadmap;
- architecture;
- v1 specs including spec 26;
- security model;
- provider docs;
- release gates;
- third-party notices;
- workflow-pack docs.

## 22.10 Economics gate

For target paid workflow, runtime MUST record enough to compute:

- cost per verified success;
- variable runtime cost;
- manual minutes displaced;
- residual human minutes;
- simple payback hypothesis.

For Work-mode Project tasks, cost/time SHOULD be attributable to Project and Task so repeated work can be evaluated rather than disappearing into chat history.

## 22.11 What does not block v1

Not required:

- public marketplace;
- full IDE replacement;
- whole-disk indexing;
- full Linux desktop;
- first-party replacement for Cua;
- enterprise control plane feature completeness;
- autonomous financial/legal commitments;
- dozens of providers;
- agent swarm architecture;
- language-server parity for every ecosystem.

## 22.12 Final release decision

V1 SHOULD NOT ship because the feature list is complete.

It ships when the trust/reliability/economics evidence is strong enough that the team would confidently run the same build on its own email, files, CRM, code, Project repositories, and business systems.

For Work mode specifically, the team should be willing to open an important real repository with uncommitted work and delegate a bounded multi-file task without first making a sacrificial copy.
