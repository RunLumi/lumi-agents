# 16 — Evals, Observability & Economics v1

Status: Normative

## 16.1 Goal

Define how Lumi measures reliability, safety, latency, cost, and workflow economics.

## 16.2 Eval layers

V1 evals MUST cover:

1. unit/contract tests;
2. deterministic executor fixtures;
3. workflow scenario tests;
4. provider/model matrix;
5. OS/app/browser matrix;
6. adversarial/security tests;
7. recovery/resume tests;
8. customer workflow outcome metrics.

## 16.3 Core reliability metrics

Track at minimum:

- verified_completion_rate;
- unexpected_human_rescue_rate;
- expected_approval_rate;
- unauthorized_side_effect_count;
- action_count;
- unnecessary_action_rate;
- retry_count;
- resume_count;
- verifier_failure_rate;
- ambiguous_state_rate;
- wrong_target_rate;
- latency;
- variable_runtime_cost.

## 16.4 Execution metrics

Track by tier:

- connector;
- browser semantic;
- native semantic;
- app adapter;
- vision;
- shell/files/artifacts.

High fallback/vision rate in mature workflow SHOULD trigger redesign review.

## 16.5 Provider metrics

Per provider/model/workflow family track:

- request success;
- structured output validity;
- tool-call validity;
- verified workflow success;
- latency;
- retry;
- context overflow;
- cost;
- human rescue.

## 16.6 Failure taxonomy

Metrics MUST use canonical failure categories from spec 18.

Do not group every failure into "model error".

## 16.7 Workflow economics

Recommended fields:

- runs_per_month;
- baseline_manual_minutes_per_run;
- residual_human_minutes_per_run;
- loaded_labor_cost_per_hour;
- runtime_variable_cost_per_run;
- support_cost_monthly;
- implementation_cost;
- cycle_time_before;
- cycle_time_after.

Derived:

```text
gross_labor_value_monthly =
  runs_per_month
  * (baseline_manual_minutes_per_run - residual_human_minutes_per_run)
  / 60
  * loaded_labor_cost_per_hour

net_value_monthly =
  gross_labor_value_monthly
  - runtime_variable_cost_monthly
  - support_cost_monthly

simple_payback_months =
  implementation_cost / max(net_value_monthly, epsilon)
```

These are decision metrics, not guaranteed marketing claims.

## 16.8 North-star metric

Primary technical-economic metric:

```text
cost_per_verified_successful_workflow
```

This MUST use verified success denominator.

## 16.9 SLOs

Alpha:
- >=30 repeated runs/workflow;
- >=90% verified completion;
- 0 unauthorized side effects.

Customer canary:
- >=100 representative runs;
- >=95% verified completion;
- <5% unexpected rescue on hardened routine path;
- 0 policy bypass.

Hardened narrow workflow:
- target >=99% verified completion on certified matrix.

## 16.10 Expected approvals

Human approval intentionally required by policy MUST NOT count as unexpected rescue.

## 16.11 Regression corpus

Every reproducible escaped production failure SHOULD become a regression scenario.

Do not remove difficult scenarios to improve headline score.

## 16.12 Observability

Each run SHOULD expose:

- trace/task ID;
- state transitions;
- model/provider selections;
- executor selections;
- policy decisions;
- approvals;
- verifier results;
- timings;
- budgets;
- failures.

Sensitive data MUST respect redaction/retention.

## 16.13 Dashboards

Internal dashboards SHOULD prioritize:

- workflow reliability;
- policy/security failures;
- top failure classes;
- provider regressions;
- app/OS regressions;
- intervention trends;
- economics;
- reuse/deployment hours.

## 16.14 Alerting

Alert on:

- unauthorized side effect;
- policy bypass;
- cross-tenant anomaly;
- update signature failure;
- verifier regression;
- sharp workflow success drop;
- cost runaway;
- high ambiguity rate.

## 16.15 Tests

V1 MUST prove metric calculations use verified completion and that expected approvals are distinguished from rescue.
