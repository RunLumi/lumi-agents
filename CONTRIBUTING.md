# Contributing

Read `AGENTS.md` before making non-trivial changes.

The repository is intentionally opinionated: reliability, bounded authority, verified outcomes, and reusable workflow assets outrank architecture fashion.

## Contribution principles

- Preserve execution hierarchy: API > browser semantic > native semantic > app adapter > vision fallback.
- Keep provider-specific behavior behind adapters.
- Keep policy/approval authority in the trusted local core.
- Add deterministic fixtures for behavior that can regress.
- Add eval coverage for workflow-facing changes.
- Do not add a side effect without risk class, policy behavior, evidence, and verification.
- Do not commit secrets, customer data, production screenshots, or raw employee trajectories.
- Prefer a small adapter over a new framework.
- Prefer upstream contribution over permanent forks when practical.
- Turn customer-specific logic into reusable workflow-pack assets.

## Architecture changes

Create or update an ADR under `docs/adr/` when changing:

- trust boundaries;
- public protocol;
- provider abstraction;
- execution hierarchy;
- licensing;
- persistence/state model;
- approval semantics;
- evidence semantics;
- updater/distribution model.

## Licensing

Lumi Agents core is Apache-2.0.

By submitting a contribution for inclusion, you agree that it is licensed under Apache-2.0 unless a separate written agreement applies.

Dependencies and copied code retain their own licenses and notices.

Do not introduce AGPL/GPL/runtime-sensitive copyleft, non-commercial, research-only, unknown, or field-of-use restricted code without explicit review.

## Definition of a good PR

A good PR is small enough to understand, includes the right tests/evals, leaves durable documentation, and reduces uncertainty about real user behavior.

A large PR that "adds an agent framework" without measured need is not progress.
