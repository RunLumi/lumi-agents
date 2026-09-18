# Instructions for coding agents

Optimize for a fast, safe customer release, not framework elegance.

## Invariants

- Do not bypass `lumi-policy`.
- Do not turn vision/coordinate actions into the default path.
- Do not execute external side effects in tests.
- Do not read or transmit secrets except through an explicit, reviewed secret interface.
- Do not add AGPL/GPL/unknown-license code to the distributed runtime.
- Do not vendor upstream code when an adapter to a pinned release is sufficient.
- Do not couple the runtime protocol to a single model provider.
- Do not make customer-specific selectors or business rules part of the generic runtime.

## Preferred implementation order

1. deterministic fixture;
2. protocol and policy contract;
3. adapter implementation;
4. integration test;
5. observability/evidence;
6. UI polish.

For upstream computer-use, prefer a thin adapter to a pinned Cua Driver release. Fork only when a measured customer blocker cannot be solved upstream or through the adapter boundary.
