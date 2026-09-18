# 22 — Lumi Agents v1 Definition of Done

Status: Normative release target

## 22.1 Product promise

V1 is complete when Lumi can reliably perform substantial real work and hardened repeatable workflows across browser and desktop environments while preserving local authority, provider neutrality, verification, evidence, and human exception handling.

## 22.2 Required platform capabilities

V1 MUST have:

- Work mode task runner;
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

- browser research;
- local files;
- shell/code;
- artifact generation;
- one connector/API;
- one long-running/resumable task.

Results MUST include inspectable artifacts/evidence.

## 22.6 Provider neutrality gate

Same representative workflow MUST run through at least two model providers without business-logic changes.

One local/OpenAI-compatible path MUST pass core contract tests.

## 22.7 Recovery gate

Demonstrate:

- app restart;
- network interruption;
- provider failure;
- approval delay;
- browser worker crash;
- native executor failure;
- ambiguous side effect.

No duplicate consequential side effect.

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
- v1 specs;
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

## 22.11 What does not block v1

Not required:

- public marketplace;
- full Linux desktop;
- first-party replacement for Cua;
- enterprise control plane feature completeness;
- autonomous financial/legal commitments;
- dozens of providers;
- agent swarm architecture.

## 22.12 Final release decision

V1 SHOULD NOT ship because the feature list is complete.

It ships when the trust/reliability/economics evidence is strong enough that the team would confidently run the same build on its own email, files, CRM, code, and business systems.
