# Contributing

The first priority is a small, auditable runtime that can ship customer workflows safely.

Before opening a change:

- keep the execution hierarchy intact: API > browser semantic > native semantic > app adapter > vision fallback;
- do not add a dependency without checking its license, provenance, maintenance, and whether the capability already exists upstream;
- do not add an autonomous side effect without a policy rule and tests;
- add or update a deterministic fixture for behavior that can regress;
- keep model-provider code outside the OS execution boundary;
- never commit secrets, customer data, production screenshots, or raw employee trajectories.

Architecture changes belong in a short ADR under `docs/adr/`. Customer-specific logic should become a reusable workflow pack rather than leaking into the core runtime.
