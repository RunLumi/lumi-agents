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

Lumi Agents community code is Apache-2.0. See [LICENSING.md](LICENSING.md) for
developer rights, private extensions and the separate commercial boundary.

By submitting a contribution for inclusion, you agree that it is licensed under Apache-2.0 unless a separate written agreement applies.

Dependencies and copied code retain their own licenses and notices.

Do not introduce AGPL/GPL/runtime-sensitive copyleft, non-commercial, research-only, unknown, or field-of-use restricted code without explicit review.

## Developer Certificate of Origin

For PRs whose base already contains this policy, sign each new non-merge commit
with `git commit -s`, after personally confirming [DCO 1.1](DCO). This records a
`Signed-off-by: Your Name <your-email>` trailer. Use an identity you are authorized
to certify; a GitHub noreply email is acceptable. Each named co-author must have
an accompanying genuine sign-off. Preserve existing notices and attribution.

You retain copyright. Apache-2.0 supplies the license grant; the DCO supplies a
provenance declaration. There is no mandatory copyright assignment or bespoke
CLA for ordinary contributions. No separate contributor registration, payment
or corporate account is required beyond the collaboration platform's own access
requirements to open issues, propose ideas or submit a patch.

The DCO workflow uses the **base branch's checker**, not a checker replaced in a
PR, and validates new commit trailers. It is a syntax/identity consistency check,
not proof of ownership or cryptographic signature verification. The adoption PR
is explicitly a policy bootstrap: no past contributor is represented as having
signed, and no historical commit is rewritten. Once the policy is on the base,
older open branches may need their actual contributors' sign-offs before merge.

AI tools and maintainers must never fabricate sign-offs for someone else. Review
AI-assisted code for provenance, third-party obligations and security before you
certify it. Resolve employer/client permission first when their rights are involved.
Disclose copied source and its exact origin. A bot-generated commit is not exempt
from human provenance review; preserve authorship and obtain proper certification
rather than adding an invented bot or human attestation. A maintainer can explain
how to prepare a properly attributed, genuinely certified replacement commit.

Uncertain rights, substantial imported projects and proposed license exceptions
need the [rights review](docs/licensing/commercial-and-rights-review.md). Do not post
private contracts or confidential identity documents in a public PR.

A verified GitHub signature is not a DCO; a DCO is not a copyright assignment.
A maintainer must preserve genuine trailers when merging and must not synthesize
new personal certifications. Repository required-check rules are configured
separately from the workflow and must not be assumed to exist.

## Definition of a good PR

A good PR is small enough to understand, includes the right tests/evals, leaves durable documentation, and reduces uncertainty about real user behavior.

A large PR that "adds an agent framework" without measured need is not progress.
