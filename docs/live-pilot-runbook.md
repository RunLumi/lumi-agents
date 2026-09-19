# Live-pilot runbook

Status: preparation only. No target tenant, customer credentials, measured
baseline or live run evidence has been supplied. Track [#40](https://github.com/RunLumi/lumi-agents/issues/40).

## Admission: one responsibility

The supervisor records the role/workflow versions, exact application and tenant,
account identity, environment (staging or production), approved operations,
allowed destinations, local credential references, data/retention policy,
exception owner, deadline and budget. Never store credential values in this
record. Start with read/report/draft work; money movement, ad spending changes,
legal commitments and personnel judgments are outside the initial boundary.

A sample export is useful for input and reconciliation testing. It does not
prove authenticated connector behavior. A fake site, fixture or synthetic
customer is not a real-system canary. A representative staging system can satisfy
a workflow canary gate but cannot establish production capacity replacement.

## Baseline before automation

1. Define one work unit and the received/eligible/exception boundary before
   selecting a sample. Keep stable source IDs and prevent replay from becoming
   another completed unit.
2. Time 5–10 initial manual cases to calibrate instrumentation. This small sample
   is a starting estimate, not sufficient evidence of the whole queue. Continue
   matched baseline measurement across the representative case mix.
3. Record active human minutes separately from elapsed cycle time. Separate
   routine processing, approval, expected exception, unexpected rescue,
   supervisor review and engineering/support. Avoid simultaneous timer overlap.
4. Obtain the buyer's loaded labor cost and include all deployment/support costs.
   Missing values remain unknown. Do not use the example packs' numbers as ROI.
5. Freeze the eligible cohort and inclusion rules. Include duplicates, missing
   inputs, wrong accounts, stale sessions, currency/date edges and known mismatches.
   Record all received cases; do not drop hard cases to improve the success rate.

## First real execution

Verify read-only authentication and tenant/account identity. Confirm the actual
runtime/OS/app/API versions and the live capability/permission state. Verify that
cancel reaches the same runtime that would execute an action. Run the expected
policy denial and unavailable-verifier tests before enabling any side effect.

Prepare a normalized action through the Workflow/Role Pack path. Record intent
before consequential execution, authorize locally, obtain scoped approval when
required, execute, re-observe external state and verify postconditions. Approval
is permission, not execution evidence. A positive HTTP response alone is not a
verified outcome.

For reconciliation, verify the complete expected input set, identities,
deterministic matching result, exceptions and saved artifact checksum. A report
file existing does not establish financial correctness. The verifier must not
read a pre-seeded expected answer as proof that the executor produced it.

## Measure and decide

- Exploration below 30 runs is useful but does not pass the Alpha gate.
- Alpha: >=30 representative runs, >=90% verified, zero unauthorized effects.
- Canary: >=100 representative real/staging runs, >=95% verified, <5% unexpected
  rescue, zero unauthorized effects, measured baseline/residual work and costs.
- Every ambiguous effect is quarantined; it is excluded from verified success
  and cannot be retried until independently resolved.
- After about 100 live runs, rescue >15%, unverifiable outcomes, material ambiguity
  or negative economics requires diagnosis and narrowing before more scope.
- Four consecutive production weeks and mature-role thresholds are separate
  from the canary. Three independent deployments are a separate reuse gate.

Persist per-unit source/time/version identity, verification evidence references,
failure category, cycle time, human-time categories and cost. Reconcile totals to
the intake queue and runtime/audit records. An imported scorecard is an assessment
of supplied evidence, never a signature or an authority token.

Support must be charged once. Distinguish ordinary residual operator work from
engineering/support costs, while including both in total human attention. Keep
cost per verified unit inclusive of failed/ambiguous attempts and preserve losses.

## Stop, recover, and learn

The supervisor stops admission and invokes local cancellation for an unexpected
effect, wrong account, privacy incident or uncontrolled retry. Preserve minimal
redacted evidence and inspect external state. Do not delete journals or reissue a
new action ID to escape an ambiguous record. Restore only after state integrity,
policy, credentials and target identity are revalidated.

For each significant escaped failure, record:

- incident ID, deployment and exact workflow/runtime/app versions;
- intended action, authority, what actually happened and independent evidence;
- affected layer (model, policy, execution, verification, external state or spec);
- containment, ambiguity/side-effect disposition and remaining customer impact;
- the smallest redacted reproducible input and expected safe outcome;
- regression test path, failing-before/passing-after result and fix PR;
- reviewer and canary re-entry decision.

Customer content remains in the approved evidence store. The public regression
corpus contains minimized/redacted cases only, under the deployment's permission
and retention policy. Nonreproducible incidents remain visible with an owner;
they must not disappear from reliability or support costs.

## External blocker record

For missing access/signing/customer input record: BLOCKER; why it matters;
smallest required external input; prepared implementation/tests/runbook; and safe
work that can continue. No open access request is answered by elapsed time or a
passing fixture. The current missing input is one named target environment,
process owner, approved workflow and representative queue/baseline.

## Assess an imported measurement ledger

The repository includes an intentionally synthetic input for checking the local
assessment path:

```sh
cargo run --locked -p lumi-evals --bin lumi-role-scorecard -- \
  docs/evals/fixtures/role-scorecard.synthetic.json
```

For collected measurements, pass the approved local JSON ledger path instead.
The command reads at most 16 MiB and writes a report to stdout. It performs no
network access or business side effect. Exit 0 means a structurally valid
assessment was produced, not that a canary passed. Exit 2 means invalid input or
an output error. Inspect validation errors and unknown measurements before using
the report.

The CLI intentionally has no trusted evidence resolver. Even a production-shaped
import cannot certify a role. A trusted runtime integration must separately
verify source hashes, tenant/deployment identity, coverage and reviewer evidence.
