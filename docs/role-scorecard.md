# Role scorecard and live assessment

`lumi-evals::role` is the evidence layer for a bounded Role Pack. It answers
whether a measured cohort meets internal canary or mature-role criteria. It is
an assessment report, not a certification token, policy decision, permission,
or deployment command. Workflow execution, approvals, audit storage, and
release certification remain owned by their existing components.

## Cohort identity

Every `RoleScorecard` has one immutable identity:

```text
tenant_id + role_id + deployment_id + role_version + runtime_version
```

Every work-unit and evidence record must carry the same values. The validator
rejects mixed tenants, roles, deployments, role versions, and runtime
versions. A work unit also carries a unique `work_unit_id`, a `window_id`, and
an exact observation timestamp.

Windows are half-open intervals, `[start_unix_seconds, end_unix_seconds)`. They
must have positive, non-overflowing duration, unique IDs, and no overlap or
gap. A timestamp at the end of a window belongs to the next window. Mature
evidence counts only the longest contiguous run of exact seven-day windows
where each window has eligible routine work and trusted source coverage; four
labels that are not four consecutive measured weeks cannot pass.

## Evidence provenance and trust

`EvidenceProvenance` is explicit:

| Provenance | Use | Live gate |
| --- | --- | --- |
| `Fixture` | deterministic engineering scenarios | never eligible |
| `Synthetic` | generated economics or cohorts | never eligible |
| `Staging` | customer-representative controlled system | canary only |
| `Production` | real customer operation | canary and mature |

`EvidenceTrust::ImportedAssertion` records a claim supplied by an importer.
It is useful context but cannot satisfy a live gate. `RuntimeVerified` can only
be attached by application code after an evidence verifier has checked the
reference, evidence digest, tenant/deployment identity, audit digest, and
reviewer proof.
Deserializing an imported `runtime_verified` label fails closed. Callers must
use `RoleScorecard::assess_with_verifier` and provide that verifier; the plain
`assess` method treats every source as imported.

The verifier callback is an observation check. It cannot authorize an action or
change policy. `RepresentativeCoverage` must point to an evidence reference
in the same scorecard. A true coverage claim without a runtime-verified,
reviewer-backed reference is ineligible for a live gate.

## Measurements

Each `WorkUnitMeasurement` represents one received and attempted unit. The
record keeps `eligible`, `completed`, and `verified` separate. Verified implies
completed and eligibility. Runtime cost is recorded per attempted unit, so
failed, ambiguous, and ineligible attempts still contribute to total cost.

Human time is split into disjoint categories:

```text
residual operator time = expected approval + expected exception
                       + unexpected rescue + review
support attention is reported separately
```

When all operator-time categories are supplied, the validator requires that
equality. Residual minutes may exceed baseline minutes: the scorecard records
that as negative displacement and charges the extra measured labor against the
economics result instead of hiding a costly failure.
Support minutes remain a separate attention category because support dollars
are already included in the window operating-cost ledger; this prevents
subtracting the same labor twice. Missing values remain unknown. Expected approvals are not unexpected rescue. Exception count,
escalation count, correctly escalated exceptions, and incorrectly handled
exceptions are kept separately; mature assessment requires every eligible
unit to have these measurements and rejects incorrect handling. Rescue-rate
denominators use eligible units with rescue, while the all-attempt
`unexpected_rescue_events` total remains available for supervision attention.

The scorecard also carries optional SLA and reuse measurements:

```text
sla_met per eligible unit
deployment_hours
customer_specific_code_percent
shared_role_logic_percent
```

These are reported with the cohort and are unknown when absent. They do not
create authority or hide customer-specific effort.

Routine coverage is reported explicitly as eligible routine units divided by
all received units. Exception-unit rate uses eligible routine units as its
matching denominator and counts eligible units with at least one exception;
raw exception counts remain separate.

Optional per-unit cycle times produce observed before/after averages, signed
minutes reduced per unit, and a signed reduction rate. Missing cycle times stay
`null`; the scorecard never substitutes an SLA or a guessed cycle time. The
SLA rate is likewise `null` when eligible-unit SLA observations are incomplete.

Human attention is also reported across all received units, including
ineligible work: residual human minutes plus support minutes, normalized per
100 received units. Routine displacement metrics continue to use the eligible
routine denominator, so out-of-scope work cannot disappear from supervision
economics.

## Exact economics

`RoleMetrics::total_attempted_cost_micro_usd` is the sum of every attempted
unit's runtime cost plus each window's support and deployment costs. A missing
unit cost or missing window cost keeps the total unknown. The
`CostPerVerifiedUnit` report preserves both the full attempted-cost numerator
and the verified-unit denominator; an executor success or an ambiguous result
is never added to the denominator.

When baseline minutes, residual minutes, and loaded labor cost are measured,
the report derives signed displaced labor value. `SignedRatio` and
`displaced_human_minutes` preserve extra human work as a negative numerator;
`signed_net_loss_micro_usd` is:

```text
attempted runtime + support + deployment cost - displaced labor value
```

Positive means loss; negative means net value created. No value is invented for
an empty or zero-denominator cohort. A non-positive displaced labor value is a
known mature-gate failure, not an unknown measurement.

`signed_net_value_created_micro_usd` is the sign-inverted loss alias. When
runtime, support, deployment, labor value, and the observed window cohort are
known, `payback_hypothesis_observed_windows` divides one-time deployment cost
by positive recurring net value. It is expressed in the same observed cohort
window units, so it does not silently extrapolate to calendar months or years;
unknown or non-positive recurring net value yields `null`.

## Gate criteria

Canary and mature assessments are separate `GateResult` values.

Customer canary requires all of:

- staging or production provenance;
- representative coverage with a runtime-verified reviewer proof;
- at least 100 eligible representative units;
- at least 95% verified completion;
- measured baseline and residual human minutes;
- complete runtime, support, and deployment cost measurements for all attempts;
- unexpected rescue strictly below 5% of eligible units;
- zero unauthorized side effects;
- no measured safety or privacy regression.

The mature internal target requires all of the canary safety conditions plus:

- production provenance;
- at least four consecutive exact seven-day windows;
- at least 99% verified completion on the eligible routine denominator;
- at least 80% baseline human minutes removed;
- at least 80% eligible units without unexpected rescue;
- total attempted operating cost at most 30% of displaced labor value;
- complete, correct exception escalation;
- zero wrong-target effects and ambiguous states;
- zero safety and privacy regressions.

The percentage comparisons are exact cross-multiplied comparisons, so 99% and
80% pass their inclusive mature thresholds while the canary rescue boundary is
strictly below 5%.

The report uses `Pass`, `Fail`, and `Unknown`. Missing
measurements produce `Unknown`; malformed identities, duplicate units, duplicate
references, mixed versions, out-of-window timestamps, and inconsistent
categories are validation errors. A fixture, synthetic cohort, staging cohort,
short window, imported assertion, or self-reported `Production` field can
never produce `HardenedDigitalRole`.

If a live verifier is unavailable or cannot prove a referenced digest, the
result remains `Unknown` until that evidence is resolved.

## Example integration boundary

An application should keep the scorecard input and evidence verifier separate:

```rust,no_run
use lumi_evals::RoleScorecard;

let report = scorecard.assess_with_verifier(|evidence, identity| {
    audit_store.verify_digest(
        &evidence.reference,
        &identity.tenant_id,
        &identity.deployment_id,
        &evidence.reviewer_proof,
    )
});

// Consume report.canary/report.mature as review evidence. A passing report
// does not authorize a side effect or change the deployment's policy.
```

The scorecard has no database, network client, credential access, or executor
integration. Store raw evidence and verifier decisions in the caller's audit
system with the retention and redaction policy appropriate to the tenant.
