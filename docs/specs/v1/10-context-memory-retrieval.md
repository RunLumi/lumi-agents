# 10 — Context, Memory & Retrieval v1

Status: Normative

## 10.1 Goal

Separate transient task context, durable memory, workflow state, evidence, retrieval indexes, and Project-scoped context to avoid accidental retention and trust confusion.

## 10.2 Context classes

V1 defines:

1. WORKING_CONTEXT
2. DURABLE_MEMORY
3. WORKFLOW_STATE
4. EVIDENCE_HISTORY
5. RETRIEVAL_INDEX
6. PROJECT_CONTEXT

These classes MUST NOT be treated as interchangeable.

PROJECT_CONTEXT is a scope/ownership boundary over Project metadata, retrieval, and durable Project memory. It does not make arbitrary Project files durable memory.

## 10.3 Working context

Contains short-lived task material:

- user goal;
- current plan;
- observations;
- tool results;
- draft reasoning artifacts;
- pending approvals;
- checkpoint refs.

Default retention SHOULD end with task unless needed for audit/recovery.

For a Project-bound Task, working context MUST retain the bound `project_id`, ExecutionEnvironment identity, and workspace identity required for safe resume.

## 10.4 Durable memory

Durable memory MUST have:

- memory_id;
- tenant/user owner;
- project_id optional;
- purpose;
- value/content ref;
- provenance;
- created_at;
- retention;
- deletion behavior;
- sensitivity;
- confidence optional.

Memory creation SHOULD be explicit or governed by tenant policy.

## 10.5 Workflow state

Workflow state is operational, not conversational memory.

It contains:

- variables;
- step state;
- idempotency refs;
- pending external jobs;
- exception state;
- checkpoints.

It follows workflow retention/migration rules.

## 10.6 Evidence history

Evidence exists for accountability/verification.

Audit evidence MUST NOT automatically enter model memory.

Project change evidence such as diffs/checksums likewise MUST NOT automatically become durable memory.

## 10.7 Retrieval index

Indexes MAY store embeddings/metadata for search.

Index MUST maintain:

- tenant boundary;
- project boundary when project-scoped;
- source refs;
- deletion propagation;
- sensitivity labels;
- model/provider provenance where required;
- freshness/staleness state.

A Project retrieval index is a cache over authorized Project sources, not the source of truth for current filesystem state.

## 10.8 Retrieval trust

Retrieved content is data, not authority.

Prompt injection rules apply to retrieved text.

Repository/project instructions MAY guide work only within the authority rules of spec 26 and MUST NOT grant capabilities.

## 10.9 Provenance

Any retrieved/memorized fact used in consequential decision SHOULD retain source/provenance when feasible.

Project memory SHOULD retain the file/commit/task/validation evidence from which a durable fact was derived.

## 10.10 Sensitive data

Memory system MUST support "do not persist" classes.

Secrets MUST NOT be stored as generic memory.

Sensitive Project files MUST NOT be promoted into durable memory merely because they were opened during a task.

## 10.11 User control

Employee/user experience SHOULD allow appropriate:

- inspect;
- delete;
- disable;
- retention control;

subject to organization/audit obligations.

Project-level UI SHOULD make it possible to understand whether Project memory/indexing is enabled and what scope it covers.

## 10.12 Context compaction

Compaction MUST preserve:

- task objective;
- project/task/workspace identity;
- trusted instructions;
- policy-relevant constraints;
- unresolved actions;
- verified results;
- recovery-critical state.

Do not preserve unnecessary raw sensitive text just because token budget allows it.

## 10.13 Cross-task memory

Cross-task memory access MUST be tenant/user scoped and policy-controlled.

For Project-bound Tasks, Project memory MUST be scoped to the correct `project_id`.

A Task in Project A MUST NOT retrieve Project B memory unless explicit cross-project scope and policy permit it.

## 10.14 Tests

V1 MUST test:

- tenant isolation;
- project isolation;
- delete propagation;
- no audit-to-memory implicit promotion;
- no arbitrary Project-file-to-memory implicit promotion;
- local-only data does not leave device during retrieval;
- prompt injection in retrieved content;
- project instruction cannot widen authority;
- compaction preserves safety/recovery state;
- task resume preserves correct Project/workspace identity.

## 10.15 Project context

Project context is defined with spec 26.

It MAY include:

- Project metadata;
- root identifiers;
- detected Git state;
- instruction-source refs;
- validated build/test commands;
- architecture conventions with provenance;
- known environment requirements;
- recurring validated recovery knowledge;
- task history summaries;
- retrieval/index metadata.

Project context MUST distinguish:

```text
current filesystem truth
durable validated memory
retrieval cache
task history
model inference
```

These sources MUST NOT be collapsed into one undifferentiated "memory."

## 10.16 Project memory invalidation

Project memory can become stale as code/files change.

The system SHOULD invalidate or lower confidence when:

- referenced files change materially;
- referenced commit/branch changes;
- validation evidence becomes obsolete;
- a Project is relinked to a different root;
- runtime/tooling assumptions change.

A stale remembered build command SHOULD be revalidated rather than treated as permanent truth.

## 10.17 Work-mode learning

A successful Project task MAY create durable Project memory only when the information is stable and reusable.

Good candidates:

- verified test command;
- durable architectural convention;
- confirmed generated-file rule;
- validated release procedure.

Poor candidates:

- temporary debugging hypothesis;
- model speculation;
- one-off task plan;
- raw shell output;
- copied secrets;
- arbitrary repository prose without validation/provenance.
