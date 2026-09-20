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
- Project-native Automations per spec 13;
- time/event triggers with explicit timezone and durable run history;
- automation Review Queue/history with manual run, pause/resume, retry and continue-interactively;
- global and project-filtered Automations management per spec 27;
- project Skill/Plugin installation and connection readiness per specs 28–29;
- background schedule/event support;
- extension/connector manifest;
- employee desktop shell;
- signed update path;
- device registration/revocation;
- evals, observability, and workflow economics.

Document Workspace (Spec 30) is a separately enabled, phased feature. It is not
promoted wholesale into this core-V1 MUST list by research alone. Any included
preview/edit capability must satisfy its applicable gate in 22.15; unqualified
native Office editors remain disabled without blocking unrelated qualified work.

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

## 22.8 Automations gate

V1 MUST demonstrate at least:

- one Project-native Automation imported or created from structured configuration;
- manual test before unattended activation;
- one daily/cron run with explicit timezone;
- one event + follow-up-offset Automation;
- restart without losing definitions, next-run state, or run history;
- fresh-context recurring runs;
- bounded unattended authority intersected with current policy;
- overlap prevention;
- deduplication;
- missed-run policy;
- retry/backoff without replaying ambiguous side effects;
- repeated failure auto-pause;
- semantic NO_ACTION/NO_ALERT recorded as successful without unnecessary notification;
- WAITING_APPROVAL with zero material side effect before approval;
- review/history item that can continue interactively;
- global and project-filtered views use the same Automation ID/revision and commands;
- global creation requires a project; copied definitions do not copy connection grants;
- per-user read/mark-all-read changes no approval, incident or schedule state;
- recurring success remains Active and Completed correctly means schedule exhaustion;
- shared-resource admission prevents different automations controlling one browser session concurrently;
- missing/revoked project connections and changed extension snapshots block unsafe scheduled use.

The `RunLumi/ads-agents` fixture SHOULD prove the intraday and daily cases:
scheduled Ads work remains externally read-only while still persisting required
Project reports/worklogs. Public CI uses a sanitized fixture; live private access
is a separate proof requirement. Detailed management tests are in
[Spec 27](27-global-and-project-automations.md).

## 22.9 Distribution gate

macOS:
- signed;
- notarized;
- updater/rollback tested.

Windows:
- code-signed;
- installer/updater/rollback tested.

## 22.10 Documentation gate

Must be current:

- AGENTS.md;
- roadmap;
- architecture;
- v1 specs including specs 26–30;
- security model;
- provider docs;
- release gates;
- third-party notices;
- workflow-pack docs.

## 22.11 Economics gate

For target paid workflow, runtime MUST record enough to compute:

- cost per verified success;
- variable runtime cost;
- manual minutes displaced;
- residual human minutes;
- simple payback hypothesis.

For Work-mode Project tasks, cost/time SHOULD be attributable to Project and Task so repeated work can be evaluated rather than disappearing into chat history.

## 22.12 What does not block v1

Not required:

- public marketplace;
- full IDE replacement;
- full Office suite, macros, universal Office fidelity or Excel-compatible calculation engine;
- unqualified native DOCX/PPTX editing before explicit feature-scope promotion;
- real-time document collaboration or mandatory Office document server;
- whole-disk indexing;
- full Linux desktop;
- first-party replacement for Cua;
- enterprise control plane feature completeness;
- autonomous financial/legal commitments;
- dozens of providers;
- agent swarm architecture;
- language-server parity for every ecosystem;
- a custom project-local encrypted vault or automatic credential sync;
- universal foreign plugin runtime compatibility.

## 22.13 Final release decision

V1 SHOULD NOT ship because the feature list is complete.

It ships when the trust/reliability/economics evidence is strong enough that the team would confidently run the same build on its own email, files, CRM, code, Project repositories, and business systems.

For Work mode specifically, the team should be willing to open an important real repository with uncommitted work and delegate a bounded multi-file task without first making a sacrificial copy.

## 22.14 Project extension and connection gate

Before claiming the project extension feature ready, demonstrate
[Spec 28](28-project-skills-plugins.md) and
[Spec 29](29-project-connections-secrets.md) through the actual runtime boundary:

- native/compatibility Skill discovery executes no code and cannot silently shadow names;
- project installation, activation and connection setup are distinct, scoped and reviewable;
- exact package/source locks and run snapshots survive restart and safe updates;
- unsupported required sandbox/permission semantics fail closed;
- credentials stay in the approved broker/backend, not project credentials.json;
- A/B project, account, caller and destination isolation hold even with copied IDs/refs;
- authorized setup safely maintains .gitignore, while runtime secret protection remains independent of it;
- tracked/legacy credentials require explicit remediation; vault unavailability never causes plaintext fallback;
- refresh/revoke/update/uninstall cannot revive old authority or delete another project's credentials;
- clone/export/worktree and model/log/trace paths contain no secret values.

Use synthetic adversarial fixtures and separately record supported-platform
credential/sandbox canaries. Passing document/example checks does not pass this gate.

## 22.15 Document Workspace capability gate (when included)

For each [Spec 30](30-document-workspace-preview-edit.md) capability exposed in a
release, demonstrate the relevant requirements below. This is a feature
qualification gate, not a demand to ship every proposed Office adapter in core V1.
Unqualified capabilities remain disabled and clearly unavailable.

- Files, Artifacts and Automation results open the same authorized file/version;
- staged React integration preserves the existing desktop/host boundary;
- exposed text/Markdown, CSV, raster-image and PDF scope works offline as specified;
- exposed DOCX/XLSX/PPTX preview or native editing passes its declared feature profile;
- unsupported document features, approximate previews and stale formula caches are explicit;
- no-op saves leave sources unchanged; edited Office copies and qualified replacement preserve promised content;
- candidate bytes reopen and validate; all publication uses the normal action/intent/policy/approval/execution/verification/audit pipeline;
- source conflicts, storage failure before dispatch, duplicate replay and crash recovery cannot bypass that pipeline;
- document HTML/SVG/XML cannot invoke host commands, read secrets, run macros or fetch unapproved resources;
- malformed/oversized files, worker crashes, cancel, disk-full and Windows file locks fail without losing work;
- exact package/transitive/font/binary licenses, integrity and advisory checks are recorded;
- real WKWebView/WebView2 keyboard/IME/accessibility and resource-budget tests pass for shipped paths.

Spec 30.18 defines the detailed corpus. Missing native Office editing remains an
explicit incomplete part of the full feature target, not a feature quietly
redefined as preview-only. Library documentation and repository CI alone do not
certify format preservation, safe execution or native-app performance.
