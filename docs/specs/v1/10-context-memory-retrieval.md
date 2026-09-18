# 10 — Context, Memory & Retrieval v1

Status: Normative

## 10.1 Goal

Separate transient task context, durable memory, workflow state, evidence, and retrieval indexes to avoid accidental retention and trust confusion.

## 10.2 Context classes

V1 defines:

1. WORKING_CONTEXT
2. DURABLE_MEMORY
3. WORKFLOW_STATE
4. EVIDENCE_HISTORY
5. RETRIEVAL_INDEX

These classes MUST NOT be treated as interchangeable.

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

## 10.4 Durable memory

Durable memory MUST have:

- memory_id;
- tenant/user owner;
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

## 10.7 Retrieval index

Indexes MAY store embeddings/metadata for search.

Index MUST maintain:

- tenant boundary;
- source refs;
- deletion propagation;
- sensitivity labels;
- model/provider provenance where required.

## 10.8 Retrieval trust

Retrieved content is data, not authority.

Prompt injection rules apply to retrieved text.

## 10.9 Provenance

Any retrieved/memorized fact used in consequential decision SHOULD retain source/provenance when feasible.

## 10.10 Sensitive data

Memory system MUST support "do not persist" classes.

Secrets MUST NOT be stored as generic memory.

## 10.11 User control

Employee/user experience SHOULD allow appropriate:

- inspect;
- delete;
- disable;
- retention control;

subject to organization/audit obligations.

## 10.12 Context compaction

Compaction MUST preserve:

- task objective;
- trusted instructions;
- policy-relevant constraints;
- unresolved actions;
- verified results;
- recovery-critical state.

Do not preserve unnecessary raw sensitive text just because token budget allows it.

## 10.13 Cross-task memory

Cross-task memory access MUST be tenant/user scoped and policy-controlled.

## 10.14 Tests

V1 MUST test:

- tenant isolation;
- delete propagation;
- no audit-to-memory implicit promotion;
- local-only data does not leave device during retrieval;
- prompt injection in retrieved content;
- compaction preserves safety/recovery state.
