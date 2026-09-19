# ADR 0011 — Durable Project domain and root-bounded authority

Status: Accepted

Date: 2026-09-19

## Context

Spec 26 (`docs/specs/v1/26-project-workspace-folder-as-project.md`) and P0
issue #50 define Folder-as-Project as the core Work-mode primitive: a user
opens a local folder and delegates work against it across durable tasks.
Until now no `Project` concept existed in code; only task-scoped spec 08
workspaces. Lumi needed a place for durable Project identity and the
root-filesystem authority boundary without building a project-management
framework.

## Decision

1. **New narrow crate `lumi-project`.** The Project registry is a genuinely
   separate responsibility from task/run execution state (`lumi-state`) and
   from per-task workspaces (`lumi-workspaces`, spec 08). The crate owns:
   the `ProjectRecord` (all §26.3 fields), the durable `ProjectStore`,
   root health/relink, and project-scope path resolution. It composes
   `lumi-workspaces` primitives; it does not duplicate them.

2. **Registry persistence follows the state-store conventions.** One JSON
   document (`projects.json`), atomic temp-file + rename writes, an
   advisory transaction lock that a crash leaves in place for operator
   recovery, generation-based compare-before-save, and unknown schema
   versions fail closed. `update_record(record, expected_generation)`
   exposes the compare-and-swap contract for deliberate registry edits.

3. **Identity is proven on disk, not assumed from paths.** Open Folder
   writes `.lumi/project.json` carrying the `project_id`. Reopening a
   folder whose marker matches reopens the same project identity.
   A folder whose marker is missing/foreign (moved, replaced) surfaces
   `Moved` health or `PathClaimedByUnprovenProject`; relinking is
   deliberate and refused while a root is still healthy. A missing or
   moved root is never silently recreated (§26.4).

4. **Authorized roots are the only filesystem boundary.** Path resolution
   delegates to the spec 08 fail-closed resolver (re-anchor absolutes,
   refuse `..` and symlink escapes). Multi-root projects require explicit
   root registration; siblings are never inferred. Resolution routes an
   existing file to the root that contains it and falls back to the
   primary root for create targets.

5. **Protocol additions are additive.** `ProjectId`, `EnvironmentId`,
   `WorkspaceKind`, and `ProjectTaskBinding` join `lumi-protocol`;
   `Task.project_binding` and workspace-metadata `project_id` /
   `workspace_kind` fields are optional with serde defaults, so existing
   persisted state deserializes unchanged (spec 20 compatibility).
   `None` workspace kind means a legacy task-depot workspace and is never
   reinterpreted as project scope.

6. **Capability snapshots advertise only what exists.** A fresh project
   advertises `project_open_folder`, `project_recent`, `files_read`,
   `files_write`, `shell_host_bounded` (plus `git_read` when a `.git`
   directory is present). Git/clone/semantic capabilities appear only
   when their implementations ship, keeping negotiation honest.

## Consequences

- Task binding to projects is durable and survives restart; resume must
  restore the same project/workspace relationship (later PRs wire the
  orchestrator and desktop).
- Project instruction files (AGENTS.md/README.md) are recorded with
  SHA-256 provenance at open time. They remain observations; nothing in
  this crate grants them authority.
- The `.lumi/` directory is written into opened project folders. This
  matches the product convention for project-local artifacts and is the
  identity anchor; documentation must tell users not to commit secrets
  there (it contains only the project id).
- Desktop surfaces (Open Folder, Recent Projects, Project home) bind to
  `lumi-project` in a later PR; the crate ships with its own acceptance
  tests for spec 26 §26.34 items 1–5 first.

## Alternatives considered

- **Store projects in `PersistedState`.** Rejected: project lifecycle
  (open/close across restarts, multi-project) is independent of task
  journal state; merging them couples two write patterns and forces
  project writes through the task-state lock.
- **Project as a `Workspace` variant only.** Rejected: spec 26.6 makes
  durable Projects and task workspaces distinct; identity, authorized
  roots, and health/relink have no natural home in spec 08 metadata.
- **UUID-only identity without an on-disk marker.** Rejected: path
  equality cannot prove "same resource" after a folder is replaced at
  the same path; the marker makes §26.3's "when the runtime can prove
  it is the same resource" decidable.
