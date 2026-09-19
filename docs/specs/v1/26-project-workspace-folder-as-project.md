# 26 — Project Workspace & Folder-as-Project v1

Status: Normative product contract

## 26.1 Goal

Define the durable **Project** abstraction that lets a user open a local folder or repository and delegate substantial work against it over time.

The intended experience is:

```text
Open Folder / Clone Repository
  -> Project
  -> inspect + search + edit + create + move + delete
  -> run bounded shell/build/test/lint
  -> inspect and operate Git safely
  -> create/resume multiple durable tasks
  -> preserve project-scoped context and evidence
```

A Project is not an editor tab and is not merely a task working directory.

A Project is a durable, policy-bounded working context rooted in one ExecutionEnvironment.

## 26.2 Domain hierarchy

V1 uses this ownership hierarchy:

```text
ExecutionEnvironment
  -> Project
      -> Task
          -> Run
              -> Action
```

The ExecutionEnvironment owns the actual machine state:

- local filesystem;
- installed applications;
- authenticated browser/app sessions;
- local credentials;
- native permissions;
- runtime generation;
- executor capabilities.

A Project owns durable working context for one user-selected body of work.

A Task owns a goal and execution state inside a Project.

A Run is one execution attempt/session of a Task.

Remote clients MAY supervise a Project but MUST NOT pretend to own its local paths, credentials, sessions, or filesystem.

## 26.3 Project identity

A v1 Project MUST have a stable `project_id` independent of display name and filesystem path.

Project metadata MUST include at least:

- project_id;
- tenant_id;
- owner/principal;
- execution_environment_id;
- display_name;
- one or more authorized roots;
- primary root;
- created_at;
- last_opened_at;
- project type or detected source type;
- capability snapshot;
- project policy reference;
- instruction-source metadata;
- indexing state;
- Git repository metadata when applicable.

Project identity MUST survive application restart.

Moving or renaming an authorized folder SHOULD preserve Project identity when the runtime can prove it is the same resource. Otherwise the user MUST explicitly relink it.

## 26.4 Project creation modes

The desktop application MUST support at least:

1. **Open Folder**
   - user selects an existing local folder;
   - selected folder becomes the primary project root.

2. **Open Repository Folder**
   - same as Open Folder, with Git metadata discovered when present.

3. **Clone Repository**
   - user supplies or selects an authorized remote repository;
   - Lumi clones into a user-approved parent directory;
   - the resulting local checkout becomes the Project root.

4. **Open Recent Project**
   - restores durable project identity and revalidates the local root before use.

A missing or moved project root MUST NOT be silently recreated at a different path.

## 26.5 Authorized roots and default filesystem boundary

A Project MUST define one or more explicit authorized filesystem roots.

The primary root is the default boundary for:

- file reads;
- file writes;
- search/indexing;
- shell current working directory;
- project-local artifact generation;
- project instruction discovery;
- Git operations.

By default, an agent MUST NOT read or mutate files outside authorized roots.

Access outside Project roots requires an explicit capability/policy grant scoped as narrowly as practical.

Examples:

Good:
```text
read: /Users/alice/projects/acme/**
write: /Users/alice/projects/acme/**
read: /Users/alice/.config/tool/config.json
```

Bad:
```text
read/write: /Users/alice/**
```

Home directory, credential stores, SSH directories, browser profiles, system configuration, and unrelated repositories MUST NOT become implicitly authorized because the Project lives under the same parent directory.

## 26.6 Project vs task workspace

Spec 08 task workspaces and Project roots are related but distinct.

A Project is durable across tasks.

A task workspace MAY be:

- the Project root directly;
- a project subdirectory;
- an isolated worktree/checkout;
- a temporary staging directory linked to the Project;
- a sandbox/container projection of the Project.

The runtime MUST record which physical/logical workspace a Task is operating against.

A task MUST NOT silently switch from an isolated workspace to the live Project root.

When multiple tasks mutate the same Project concurrently, the runtime MUST either:

- isolate them;
- serialize conflicting writes;
- or explicitly detect and surface conflicts.

## 26.7 Project discovery

On open, Lumi SHOULD perform bounded discovery sufficient to understand how to work in the Project.

Discovery MAY include:

- directory tree summary;
- Git repository root and status;
- current branch;
- tracked/untracked changes;
- README files;
- AGENTS.md or equivalent project guidance;
- package/build manifests;
- lockfiles;
- workspace/monorepo manifests;
- test configuration;
- lint/format configuration;
- CI configuration;
- docs/specs directories;
- common entrypoints;
- recently changed files.

Discovery MUST be bounded by file-count/size/time budgets.

Lumi MUST NOT eagerly upload or embed the entire Project into a model context.

## 26.8 Project instruction files

Project guidance MAY be discovered from files such as:

- `AGENTS.md`;
- `README.md`;
- contributor instructions;
- tool-specific project instruction files;
- organization-defined project policy files.

Instruction files provide working guidance. They do not grant authority.

Project content MUST NOT be able to:

- widen filesystem scope;
- grant secrets;
- disable organization policy;
- enable unrestricted network access;
- approve external side effects;
- change provider/data-egress rules;
- authorize package installation;
- authorize destructive Git operations.

Instruction provenance MUST be retained.

Instructions from a newly cloned or otherwise untrusted repository MUST be treated as untrusted project content until interpreted inside the normal policy boundary.

Conflicting instructions follow the authority hierarchy defined by Lumi policy; repository text never outranks organization/user policy.

## 26.9 File operations

A Project MUST support, subject to policy:

- read;
- list;
- search;
- create file;
- create directory;
- edit/patch file;
- move/rename;
- copy;
- delete;
- restore/revert when possible;
- checksum;
- diff;
- metadata/stat.

Edits SHOULD prefer structured patches or atomic replacement rather than fragile UI typing when direct filesystem APIs are available.

File writes MUST:

- revalidate the target path;
- remain within authorized roots;
- protect against symlink/junction escape;
- account for concurrent external modification;
- avoid silently overwriting changed content.

For an existing file, the runtime SHOULD compare the current content/version against the version used to prepare the edit.

Stale-write conflicts MUST be surfaced rather than silently clobbered.

## 26.10 Delete, overwrite, and recovery

Deletion and destructive overwrite MUST follow spec 08.

Inside a Project:

- reversible delete SHOULD be preferred;
- deleted files SHOULD retain enough metadata for restore where platform permits;
- destructive bulk changes require stronger policy/risk handling;
- mass deletion, recursive replacement, or destructive cleanup MUST NOT be inferred from broad natural-language goals without a bounded action plan.

The runtime SHOULD preserve a recoverable patch/diff boundary around agent changes even when Git is unavailable.

## 26.11 Search and indexing

Projects SHOULD provide local-first search across authorized roots.

Supported retrieval SHOULD include where practical:

- filename/path search;
- exact text search;
- code/symbol-aware search;
- semantic retrieval;
- Git-aware changed-file prioritization.

Indexes MUST:

- remain scoped to Project/tenant;
- preserve source paths;
- propagate deletion;
- honor ignored/sensitive paths;
- respect local-only policy;
- expose freshness/staleness.

Generated indexes are caches, not authoritative Project state.

The filesystem remains authoritative for current file content.

## 26.12 Ignore and sensitive-path policy

Project indexing and model context SHOULD honor:

- `.gitignore`;
- Lumi-specific ignore configuration;
- organization policy;
- explicit user excludes.

Ignore rules do not automatically equal security policy.

Sensitive-path policy MUST take precedence over convenience/search indexing.

Examples commonly excluded or restricted:

- secret files;
- private keys;
- credential stores;
- `.env` values;
- generated dependency trees;
- build output;
- large binary caches;
- database files;
- unrelated mounted directories.

A file ignored from search MAY still require separate explicit authorization before direct read.

## 26.13 Large files and binary files

The runtime MUST bound reads by file size and type.

Large or binary files SHOULD use:

- metadata inspection;
- partial/ranged reads;
- type-aware parsers;
- artifact tools;
- explicit user action where full ingestion is expensive/sensitive.

Do not send large binary content to a model merely because it is inside the Project.

## 26.14 Shell and code execution

Project shell execution follows spec 08.

Default shell behavior:

- cwd = selected task workspace/project root;
- explicit environment allowlist;
- no blanket inheritance of host secrets;
- bounded timeout/resources;
- policy-aware network;
- cancellation;
- output capture;
- provenance;
- declared isolation class.

Commands SHOULD be chosen from discovered project conventions when trustworthy and appropriate.

Examples:

- test;
- build;
- lint;
- format;
- typecheck;
- package-manager scripts;
- project-specific validation.

A command described in project content does not bypass shell policy.

## 26.15 Dependency and package changes

Adding or updating dependencies is a Project mutation with supply-chain implications.

Before applying a dependency change, the runtime SHOULD determine:

- requested package;
- ecosystem;
- version constraint;
- lockfile impact;
- provenance/registry;
- release age where policy requires;
- license where material;
- known advisory status where tooling exists.

Project-local dependency installation MAY be allowed by policy.

Global/system-wide package installation MUST require stronger authority.

Generated code MUST NOT silently execute dependency post-install scripts outside the permitted execution boundary.

## 26.16 Git discovery

When the Project contains a Git repository, Lumi SHOULD expose at minimum:

- repository root;
- current branch;
- HEAD;
- remotes metadata without secrets;
- clean/dirty status;
- staged/unstaged/untracked summary;
- diff;
- log/history;
- worktree/submodule state where material.

Git metadata is Project context.

It does not grant remote repository authority.

## 26.17 Git operations and risk classes

Git actions MUST be normalized by business effect.

Typical local read operations:

- status;
- diff;
- log;
- show;
- blame;
- branch listing.

Typical local-write operations:

- create/switch branch;
- stage;
- commit;
- create local tag;
- create worktree.

Typical external-write operations:

- push;
- create/update remote branch;
- open/update pull request through a connector;
- publish tag.

Destructive/high-risk operations include:

- hard reset that discards work;
- clean that deletes untracked files;
- force push;
- rewriting shared history;
- deleting remote branches/tags.

External/destructive actions MUST pass normal Lumi policy and approval rules.

Credentials for Git remotes MUST be resolved at the executor/connector boundary, not placed into model prompts.

## 26.18 Agent change set

A Task that mutates Project files SHOULD maintain an explicit change set containing:

- files created;
- files modified;
- files moved;
- files deleted;
- before/after checksums or equivalent version refs;
- patch/diff where practical;
- commands run;
- validations run;
- validation outcomes.

The user SHOULD be able to inspect the resulting change set without reading raw execution logs.

A Project MAY contain pre-existing user changes.

Lumi MUST distinguish pre-existing changes from agent-produced changes where technically possible.

## 26.19 Git cleanliness is not assumed

A dirty working tree MUST NOT automatically block all work.

The runtime MUST:

- detect existing changes;
- avoid claiming those changes as Lumi-generated;
- avoid destructive resets;
- avoid overwriting overlapping edits without conflict handling.

For risky or broad edits, Lumi SHOULD offer or automatically use an isolated branch/worktree when policy and repository state allow it.

## 26.20 Validation before completion

For code/project work, task completion SHOULD be based on relevant verification, not file mutation alone.

Depending on the Project, verification MAY include:

- tests;
- lint;
- typecheck;
- build;
- formatter/check mode;
- schema validation;
- generated artifact validation;
- deterministic project-specific checks.

A Task MUST report validation honestly:

- passed;
- failed;
- skipped;
- unavailable;
- ambiguous.

"Files edited" is not equivalent to "goal completed."

## 26.21 Project-scoped task history

A Project SHOULD expose durable task history including:

- task goal;
- status;
- created/completed time;
- affected files;
- artifacts;
- approvals;
- validations;
- evidence refs;
- relevant cost/time;
- unresolved items.

Task history MUST NOT be implemented as unlimited raw transcript retention by default.

## 26.22 Project memory

Project memory is a scope within spec 10 durable memory/retrieval.

Project-scoped durable memory MAY contain stable information such as:

- validated build/test commands;
- architecture conventions;
- durable project decisions;
- known environment requirements;
- recurring failure/recovery knowledge.

Project memory MUST retain provenance and MUST NOT silently promote arbitrary repository text, model guesses, or transient task output into trusted durable memory.

Project memory MUST be invalidatable when the underlying Project materially changes.

## 26.23 Project configuration

A Project MAY define Lumi-specific configuration for:

- allowed roots;
- task defaults;
- preferred validation commands;
- ignored paths;
- provider/data-egress constraints;
- shell/network constraints;
- artifact directories;
- project memory behavior.

Repository-local configuration MUST NOT be able to widen organization/user authority.

A local project config MAY narrow behavior.

Any authority-widening configuration requires an explicit higher-authority grant.

## 26.24 External filesystem changes

Users, IDEs, Git clients, build tools, and other agents may change the Project concurrently.

The runtime SHOULD detect filesystem changes through platform watchers or bounded revalidation.

Before applying a prepared mutation, Lumi MUST revalidate material preconditions.

After a user takeover or long pause, Lumi MUST re-observe relevant Project state before resuming mutation.

## 26.25 Multiple projects and switching

The desktop app SHOULD support multiple durable Projects and a recent-project list.

Each active Task MUST be bound to exactly one primary Project unless explicitly declared as a multi-project task.

Switching UI focus to another Project MUST NOT silently retarget an active Task.

Cross-project file operations require explicit multi-project scope and policy.

## 26.26 Multi-root projects

A Project MAY have multiple authorized roots for monorepos or related working directories.

Each root MUST have:

- stable root identifier;
- canonical local path;
- access mode;
- sensitivity/egress policy where needed.

The runtime MUST NOT infer sibling directories as authorized roots.

## 26.27 Clone and remote repository safety

Cloning a repository is a network + filesystem action.

Clone MUST specify:

- remote URL/connector identity;
- destination parent;
- resulting root;
- credential reference where needed;
- network policy.

Cloning or opening a repository does not authorize execution of repository code.

Post-checkout hooks, build scripts, install scripts, and repository-provided commands remain subject to shell/dependency policy.

## 26.28 Remote supervision

Remote clients MAY:

- view Project metadata safe for sync;
- create/steer/pause/cancel Tasks;
- inspect task progress;
- inspect approved change summaries/artifacts/evidence;
- approve/reject scoped actions.

Remote clients MUST NOT fabricate local paths.

A remote request referring to a local resource MUST use a Project/root/resource identifier understood by the owning ExecutionEnvironment.

Local filesystem truth remains on the ExecutionEnvironment unless policy explicitly allows synchronization.

## 26.29 UX requirements

Desktop Work mode MUST provide first-class Project actions:

- Open Folder;
- Open Recent Project;
- Clone Repository;
- New Task;
- Resume Task.

Within a Project, UI SHOULD expose:

- Files/search;
- Changes;
- Tasks/history;
- Artifacts;
- Evidence;
- validations;
- Git status/branch;
- shell/command activity at an appropriate abstraction level.

The primary interaction remains goal-oriented delegation.

Lumi is not required to become a full IDE.

Users SHOULD be able to inspect/edit/take over using their preferred external editor while Lumi remains state-aware.

## 26.30 Failure behavior

Project operations MUST fail clearly for at least:

- root missing;
- root moved/unmounted;
- permission revoked;
- path escapes root;
- symlink/junction escape;
- stale file version;
- conflicting concurrent modification;
- unsupported binary/large file path;
- shell policy denial;
- dependency policy denial;
- Git conflict;
- detached/unexpected Git state;
- credential failure;
- network failure;
- validation failure;
- insufficient disk space.

Failure MUST NOT cause silent scope widening.

## 26.31 Evidence and privacy

Routine file editing SHOULD use minimized evidence.

Do not continuously snapshot every file/screen merely because Project mode is active.

Evidence MAY include:

- normalized file change metadata;
- diffs;
- checksums;
- validation results;
- command summaries;
- Git refs;
- selected error output.

Sensitive file contents must follow evidence/redaction/retention policy.

## 26.32 Capability advertisement

An ExecutionEnvironment SHOULD advertise Project capabilities explicitly, for example:

```text
project_open_folder
project_recent
project_clone
project_multi_root
project_watch
files_read
files_write
files_patch
shell_host_bounded
git_read
git_local_write
git_remote_write
project_index_text
project_index_semantic
```

Clients MUST use capability negotiation rather than assume every runtime supports every Project function.

## 26.33 Minimum v1 Project contract

V1 Project support is conformant only if it can demonstrate end to end:

1. open a local folder as a durable Project;
2. restart Lumi and reopen the same Project identity;
3. discover key project metadata/instructions safely;
4. search/read files;
5. create and edit files;
6. rename/move and reversibly delete files;
7. run a bounded Project-local command;
8. inspect Git state when Git is present;
9. create a Project-bound Task;
10. persist/resume that Task;
11. show an inspectable Lumi change set;
12. validate the resulting work;
13. detect an external concurrent modification without clobbering it;
14. refuse path traversal/symlink escape;
15. keep unrelated filesystem/credentials outside Project authority.

## 26.34 Required tests

V1 MUST include deterministic tests for:

- open-folder project creation;
- durable project reopen after app/runtime restart;
- missing/moved-root handling;
- single-root path isolation;
- multi-root isolation;
- traversal refusal;
- symlink/junction escape refusal;
- hidden/sensitive path policy;
- project instruction cannot widen authority;
- read/search/create/edit/move/delete;
- reversible deletion;
- stale-write conflict;
- external concurrent edit;
- large/binary file bounds;
- shell cwd/env/network policy;
- package/dependency policy;
- Git status/diff on dirty tree;
- preservation of pre-existing user changes;
- local branch/commit behavior;
- remote push requires external-write authority;
- destructive Git action is blocked/approval-gated;
- task resume against correct Project/environment;
- remote client cannot substitute its own local path;
- project-memory provenance;
- validation pass/fail/skip reporting;
- capability downgrade hides/refuses unsupported operations.

## 26.35 Non-goals for v1

V1 does not require:

- a full text editor/IDE replacement;
- arbitrary whole-disk indexing;
- silent access to all repositories on a machine;
- automatic execution of repository-provided code;
- autonomous force-push/history rewrite;
- cloud mirroring of all project files;
- one Project spanning arbitrary machines without explicit synchronization;
- language-server parity for every programming language;
- perfect semantic indexing before basic project reliability works.

## 26.36 Product standard

The target experience is:

> A user opens a folder, gives Lumi a goal, and can trust Lumi to understand the project, make bounded changes, run the right validations, preserve existing work, show exactly what changed, and resume later without losing context or authority boundaries.

Folder-as-Project is a core Work-mode primitive.

It MUST be reliable enough that users can delegate real repository and document work without babysitting individual file operations.
