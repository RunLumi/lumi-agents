# 08 — Files, Shell & Artifacts v1

Status: Normative

## 8.1 Goal

Define safe local file, shell/code, and artifact behavior for Work mode and Workflow mode.

## 8.2 Workspace

Every task using local files or shell MUST operate inside an explicit workspace root unless policy grants broader access.

A workspace is an execution scope for a Task/Run. It is not automatically the same thing as a durable Project.

Workspace metadata MUST include:

- task_id;
- project_id when project-bound;
- local root;
- workspace kind;
- owner;
- created_at;
- retention;
- cleanup policy.

Canonical workspace kinds SHOULD include:

- PROJECT_ROOT;
- PROJECT_SUBDIR;
- ISOLATED_WORKTREE;
- TEMP_STAGING;
- SANDBOX_PROJECTION.

Project identity, authorized roots, discovery, Git semantics, project memory, and Folder-as-Project UX are defined by spec 26.

A task MUST NOT silently switch workspace kind or move from an isolated workspace into a live Project root.

## 8.3 File capabilities

Canonical file capabilities:

- files.read
- files.create
- files.edit
- files.move
- files.delete
- files.export
- files.share

Sensitive directories MAY be denied or approval-gated.

Project-mode file capabilities remain bounded by the Project's authorized roots from spec 26.

## 8.4 Path handling

Paths MUST be canonicalized before policy evaluation.

Traversal outside allowed roots MUST fail closed.

Symlink/junction behavior MUST be resolved safely.

A path being reachable from the host does not make it part of the task workspace or Project.

## 8.5 Destructive file actions

Delete/overwrite of existing user files SHOULD be reversible where platform/filesystem permits.

Permanent deletion MUST be DESTRUCTIVE risk.

Mass/recursive destructive changes SHOULD receive stronger review than isolated file mutations.

## 8.6 Shell execution

Shell/code execution MUST specify:

- workspace;
- executable/command;
- arguments;
- environment allowlist;
- network policy;
- timeout;
- resource limits;
- output capture;
- expected artifacts.

Do not inherit all host environment secrets by default.

When a Task is Project-bound, shell cwd SHOULD default to its declared Project workspace rather than an arbitrary process working directory.

## 8.7 Sandboxing

V1 SHOULD support isolated execution for untrusted/generated code.

Sandbox options MAY differ per platform.

The contract MUST expose whether execution is:

- HOST_BOUNDED
- SANDBOXED
- CONTAINERIZED
- REMOTE_SANDBOX

Workflow policy MAY require a minimum isolation class.

Repository/project content does not authorize lowering the required isolation class.

## 8.8 Network

Shell network egress MUST be policy-aware.

Untrusted generated code MUST NOT gain arbitrary network access merely because the host has it.

Project source code, scripts, hooks, or instructions MUST NOT widen network authority.

## 8.9 Package installation

Installing packages changes execution surface and supply-chain risk.

Package install SHOULD be:

- pinned;
- recorded;
- bounded to workspace/virtual environment where possible;
- subject to dependency age/provenance policy.

System-wide installation requires stronger approval.

Project-specific dependency-change requirements are expanded in spec 26.

## 8.10 Artifact types

V1 artifact types SHOULD include:

- text/markdown;
- PDF;
- DOCX;
- XLSX/CSV;
- PPTX;
- image;
- code repository;
- generic file bundle.

## 8.11 Artifact lifecycle

Artifact states:

- DRAFT
- VALIDATING
- READY_FOR_REVIEW
- APPROVED
- PUBLISHED
- REJECTED

Publication/share is separate from generation.

## 8.12 Artifact provenance

Artifact MUST record:

- producing task/run;
- project_id when project-bound;
- source refs;
- generator/tool/model refs;
- created_at;
- checksum when file-based;
- validation results;
- review state.

## 8.13 Validation

Artifact type SHOULD define validations.

Examples:

- DOCX opens successfully;
- XLSX formulas/references valid;
- PDF render succeeds;
- code tests/lint pass;
- CSV parse/schema valid.

Project task validation also follows spec 26 and MUST distinguish passed, failed, skipped, unavailable, and ambiguous validation.

## 8.14 Tests

V1 MUST test:

- path traversal;
- symlink escape;
- overwrite approval;
- shell timeout;
- cancellation;
- sandbox/no-network mode;
- generated artifact validation;
- publication separated from draft creation;
- Project-bound workspace cannot escape Project roots;
- isolated workspace cannot silently mutate the live Project root.

## 8.15 Project relationship

Spec 26 is normative for Folder-as-Project behavior.

The important separation is:

```text
Project = durable working context and authority scope
Workspace = concrete filesystem execution scope for one Task/Run
```

One Project may produce many task workspaces over time.

A workspace may be temporary and disposable while the Project remains durable.

File/shell implementation MUST expose enough identity to preserve this distinction across:

- resume;
- concurrent tasks;
- remote supervision;
- isolated worktrees;
- app restart;
- runtime upgrade/downgrade.

## 8.16 Document Workspace

[Spec 30](30-document-workspace-preview-edit.md) defines local previews and basic
edits for project files and generated artifacts. Files, Artifacts and Automation
Review Queue reference the same resource/version, not separate copied documents.

Opening a preview is read-only. Saving a UI or agent edit uses the trusted file
service: source-version conflict check, bounded staging, format/preservation
validation, authorized atomic publication and output verification. Unsupported
Office features cannot be silently discarded. A successful render or a valid ZIP
is not proof of business correctness or current formula results.

Document renderers receive no general filesystem, shell or secret access. Their
local resources, output copies, caches and drafts remain project/policy scoped.
Human edits create new versions and invalidate relevant prior artifact validation;
viewing an artifact never approves its publication.
