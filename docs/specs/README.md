# Lumi Agents Specifications

## Current normative version

**v1** is the current normative specification suite.

Start here:

- [v1 index](./v1/00-v1-index.md)
- [v1 definition of done](./v1/22-v1-definition-of-done.md)
- [v1 implementation order](./v1/25-v1-implementation-order.md)

The v1 suite is deliberately numbered so architecture, code, tests, and reviews can reference stable spec IDs.

Example:

> Implements Spec 04 approval binding and Spec 11 postcondition verification.

## Version policy

- `v0` files are historical design sketches and are superseded by v1.
- Breaking normative changes after v1 SHOULD create a new major spec version or explicit migration.
- Minor clarifications MAY update v1 when they do not change externally observable contract semantics.

## Relationship to other docs

- `AGENTS.md` — engineering constitution and decision rules.
- `docs/roadmap.md` — product/engineering sequencing and strategic gates.
- `docs/architecture.md` — high-level architecture.
- `docs/adr/` — architecture decisions and why.
- `docs/specs/v1/` — normative contracts and v1 acceptance criteria.
- `docs/evals/` — test/eval implementation guidance.

If a code change affects a normative contract, the relevant spec and tests MUST change in the same PR.
