# Roadmap: verified digital labor

Updated 2026-09-19. Implementation status lives in [readiness](v1-readiness.md),
not this roadmap. The [execution plan](plan.md) and GitHub issues are the active
work graph. Normative contracts remain in [specs/v1](specs/v1/00-v1-index.md).

## Product outcome

Lumi should own bounded recurring operational responsibilities across the
software a company already uses. The economic unit is verified useful work
completed with fewer human minutes and lower fully loaded cost.

The strongest objection is that mature automation products and frontier-model
platforms may already solve enough of the work. Lumi must earn differentiation
through verified outcomes, low exception/support burden and repeatable
integration. Code volume and feature parity do not answer that objection.

See [product thesis](product-thesis.md) and the provisional
[lighthouse decision](lighthouse-role.md). Finance Ops leads provisionally;
Facebook Ads Ops, HR Ops and Legal Ops are explicit candidate families from the
owner. Existing packs are implementation leverage, not proof of demand.

## Invariant

```text
Models propose → policy authorizes → executors act
→ verifiers determine outcome → evidence records → economics measures value
```

Prefer API/connector, browser semantics, native semantics, deterministic app
adapters, then vision. A new role never widens authority. Remote supervision
never acquires the environment's local credentials or policy authority.

Preserve cancellation, durable intent before side effects, scoped approvals,
idempotency, evidence minimization and recovery from ambiguity. Correct refusal
and escalation are useful outcomes; they are not completed routine work units.

## Work, Workflow and Role

Work mode handles novel work, exceptions and discovery of repeatable sequences.
Workflow Packs harden those sequences with input/output contracts, policy,
postconditions, evidence, failure behavior and evaluations. A minimal Role Pack
composes workflows into bounded operational ownership and a measurable queue.

```text
Work-mode collaboration → repeatable routine identified
→ deterministic steps and exception boundary extracted
→ policy + postconditions + evals → certified Workflow Packs
→ one Role Pack → live operation → failures become regressions
→ lower human attention per 100 work units
```

Workflow Pack remains the execution unit. Role Pack is not a replacement
orchestrator, a new source of permission or an employment-decision system.

## Evidence gates

| Gate | Required proof | Investment unlocked |
|---|---|---|
| A: real workflow | Real or representative staging system, >=100 representative runs, measured baseline/residual effort, verified outputs, exceptions and runtime economics | Harden one responsibility |
| B: lighthouse role | Multiple workflows sustain a bounded queue with substantial measured human-time displacement | Improve role throughput and exception handling |
| C: repeatability | Same role in >=3 independent deployments, measured customization/support and shared logic | Broader distribution |
| D: scale | Repeatable paid value survives deployment and support costs | Role catalog, ecosystem and broader control plane |

For an early canary retain >=95% verified completion, <5% unexpected rescue and
zero unauthorized side effects. After approximately 100 representative runs,
rescue >15%, material ambiguity or poor fully loaded economics triggers diagnosis
and narrowing before more scope.

Internal mature-role targets, not customer guarantees:

- >=99% verified completion on its certified routine matrix;
- zero unauthorized effects and no material safety/privacy/compliance regression;
- >=80% baseline routine human minutes removed;
- >=80% eligible routine units completed without unexpected rescue;
- exceptions correctly escalated;
- operating cost <=30% displaced loaded labor value after support;
- at least four consecutive measured production weeks.

Staging and fixtures cannot establish production capacity replacement.

## Near-term work

1. Repair durable persistence/replay defects and reconcile readiness (#43).
2. Select one queue and obtain actual access and baseline measurements (#40).
3. Implement only the Role Pack and scorecard needed by that responsibility (#41).
4. Bridge the smallest real API/browser path; require native only where necessary.
5. Show actual workload, approvals, exceptions, evidence, human effort and costs
   in the desktop operations console (#10). Chat is a steering tool.
6. Prepare artifact-bound signing, updater/rollback and customer onboarding (#11).
7. Run, measure, turn escaped failures into regression cases, and reduce touches.
8. Measure independent deployment reuse (#42): target >=70% shared logic and a
   third deployment under one engineering day; a tenth under four hours remains
   a later hypothesis.

Customer configuration belongs in configuration; reusable adapters and vertical
rules belong in versioned packs. If the third deployment is still largely bespoke,
narrow or redesign the product rather than expanding the catalog.

## Defer until justified

Do not prioritize generic assistant polish, dozens of providers, marketplace,
large multi-agent organizations, broad native platform support or deep fleet
features before live economics. Provider neutrality, local-only routing,
capability contracts and replaceable engines remain required boundaries.

Customer/system access, signing authority, baseline measurement, elapsed
production operation and three independent deployments are explicit dependencies.
Prepare all safe unblocked work; do not label synthetic evidence as their solution.

## Scoreboard

Track verified units; baseline and residual human minutes; approvals, exceptions,
rescue, review and support separately; routine coverage; wrong targets and
ambiguity; cycle time and SLA; runtime/support/deployment cost; signed net value;
reuse and deployment hours; and incidents converted to regressions.

The decisive question is whether a rational manager would pay again next month
for this bounded responsibility. A green test suite is necessary engineering
proof and cannot answer that commercial question.
