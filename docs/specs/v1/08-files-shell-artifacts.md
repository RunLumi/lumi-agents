# 08 — Files, Shell & Artifacts v1

Status: Normative

## 8.1 Goal

Define safe local file, shell/code, and artifact behavior for Work mode and Workflow mode.

## 8.2 Workspace

Every task using local files or shell MUST operate inside an explicit workspace root unless policy grants broader access.

Workspace metadata MUST include:

- task_id;
- local root;
- owner;
- created_at;
- retention;
- cleanup policy.

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

## 8.4 Path handling

Paths MUST be canonicalized before policy evaluation.

Traversal outside allowed roots MUST fail closed.

Symlink/junction behavior MUST be resolved safely.

## 8.5 Destructive file actions

Delete/overwrite of existing user files SHOULD be reversible where platform/filesystem permits.

Permanent deletion MUST be DESTRUCTIVE risk.

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

## 8.7 Sandboxing

V1 SHOULD support isolated execution for untrusted/generated code.

Sandbox options MAY differ per platform.

The contract MUST expose whether execution is:

- HOST_BOUNDED
- SANDBOXED
- CONTAINERIZED
- REMOTE_SANDBOX

Workflow policy MAY require a minimum isolation class.

## 8.8 Network

Shell network egress MUST be policy-aware.

Untrusted generated code MUST NOT gain arbitrary network access merely because the host has it.

## 8.9 Package installation

Installing packages changes execution surface and supply-chain risk.

Package install SHOULD be:

- pinned;
- recorded;
- bounded to workspace/virtual environment where possible;
- subject to dependency age/provenance policy.

System-wide installation requires stronger approval.

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

## 8.14 Tests

V1 MUST test:

- path traversal;
- symlink escape;
- overwrite approval;
- shell timeout;
- cancellation;
- sandbox/no-network mode;
- generated artifact validation;
- publication separated from draft creation.
