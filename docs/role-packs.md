# Role Packs v0

Role Pack v0 is the smallest composition layer above Workflow Pack. A Role
Pack names one bounded operational responsibility, one work queue and work-item
identity, and exact versions of the workflow packs that can process that queue.
It does not create a second executor or bypass local policy, approvals,
postcondition verification, audit, or evidence.

The Rust contract lives in `crates/lumi-packs/src/role.rs`. A role manifest is
loaded with `lumi_workflows::load_role_pack(role_path, packs_dir)`, which reads
the exact referenced `pack_id` and `version`, validates the authority ceiling,
and returns a `ResolvedRolePack`. `ResolvedRolePack::prepare_work_item(...)`
accepts a `RoleWorkItem { work_item_id, workflow_id, inputs }`, validates the
role queue schema, stable identity, exact composed workflow, payload types, and
unknown fields, then delegates workflow input validation and action construction
to the existing `lumi_packs::prepare_run`. It checks every materialized action
against the role ceiling and returns an intersected `effective_budget` plus the
steps that remain approval-gated. The `ResolvedRolePack::execute_step(...)`
bridge enforces those role-required approvals before calling the existing
orchestrator; the orchestrator remains the local policy, journal, verification,
and audit gate. Role preparation never executes an action.

The v0 manifest fields are:

- `schema_name: "lumi.role_pack"`, `schema_version: 0`, `role_id`, semantic
  `version`, `owner`, `supervisor`, and `job_to_be_done`;
- `scope` and explicit `non_scope` boundaries;
- one `queue` with a stable `work_item_id_field` and named triggers;
- a schema-defined `work_item_schema`;
- exact `workflow_packs` references;
- an `authority` ceiling containing exact capabilities and risk classes,
  approval-required consequential risks, and finite action/retry/vision/write
  and model-cost maxima.

The role manifest and its nested queue/authority/work-item schema objects use
the strict v0 envelope; unknown role keys are rejected before admission.

Validation fails closed when a referenced pack is missing or has the wrong
version, when the role adds a capability or risk class, when a consequential
step is not approval-bound, or when any role budget exceeds a composed pack's
declared ceiling. The effective runtime budget is the minimum of the role and
pack limits. This makes the ceiling an executable preparation check rather than
metadata that a runner may ignore.

Role admission also computes canonical content digests over the role and every
referenced workflow manifest. A prepared run is refused if the same role or
pack version later has different content. Pack runtime ranges are accepted only
in the repository's narrow `>=major.minor[.patch]` form with an optional `<`
upper bound; unsupported or incompatible ranges fail admission against the
compiled runtime version.

Role v0 also refuses a composed step with declared preconditions until the
existing pack runner evaluates them; silently carrying an unenforced
precondition would make the role contract weaker than its manifest.

`roles/finance-operations-associate/role.json` is a provisional composition
example for Finance Operations. It composes invoice reconciliation and expense
audit preparation. The expense pack contains an `EXTERNAL_WRITE` step, so the
role requires local policy approval for that risk class and explicitly excludes
autonomous verdict approval, payments, transfers, tax filings, and binding
financial commitments. The manifest is a composition fixture; it is not live
customer evidence or a capacity-replacement certification.

Role Packs do not claim maturity from fixture runs. Live scorecards and
provenance-aware maturity gates belong in `lumi-evals`: fixture and synthetic
runs can prove contracts, while sustained capacity claims require representative
production evidence, measured human baseline and residual work, explicit
deployment/window identity, and independent postcondition evidence.
