# ADR 0010: Role composition and market-proof gates

- Status: Accepted for experimental v0; no live certification granted
- Date: 2026-09-19
- Complements: ADR 0006 (Work/Workflow modes), 0008 (durable intent), 0009 (authority ceilings)

## Decision

A Role Pack composes exact versions of Workflow Packs into one bounded queue,
with an owner, supervisor, scope, work-item schema and additional capability,
risk, approval and budget ceilings. It does not replace the orchestrator or
grant a capability. Local policy still authorizes every normalized action.

Keep composition in existing `lumi-packs` and `lumi-workflows` crates. Keep
measurement in `lumi-evals`. Reject unsupported preconditions rather than
silently ignoring them in the role path. Enforce stricter role approvals inside
the same execution gate, including when tenant policy would otherwise allow an
action. No second approval-consumption path is introduced.

The example Finance Operations role composes existing synthetic packs; its
external audit-verdict write remains approval-bound. The initial market
experiment is narrower read/report work and does not certify that complete
two-pack role. Ads, HR and Legal Ops remain candidates if access and customer
evidence favor them.

## Evidence and alternatives

The main audit found useful pack preparation but fixture-only effect/economic
evidence. Adding a new agent organization or scheduler framework would not
establish operational ownership. Reuse the current action, policy, state,
verification and audit contracts and test their failure boundaries first.

The existing pinned T3/Codex/Cua reference reviews reinforce environment-owned
authority, durable intent and exact effect evidence. The 2026-09-19 primary
T3 recheck distinguishes persisted command receipts from transient test
milestones; neither substitutes for a Lumi business postcondition.

## Consequences and deletion test

Role scorecards are assessments of supplied measurements. Fixture/staging labels
cannot prove production capacity; imported assertions cannot mint trusted
runtime evidence. Live canary, sustained production role and three independent
deployments remain separate gates.

No new provider, executor or external dependency is required. The change does
not broaden native platform support or certify a customer system. If live
operation does not reduce fully loaded human effort, or the third deployment
remains bespoke, narrow the role or remove this layer rather than expand it.
