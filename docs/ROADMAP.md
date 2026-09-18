# Release roadmap

The fastest path is a **headless local runtime first**, not a polished desktop shell. Add Tauri when onboarding, permissions, updates, and enterprise deployment require a UI.

## 0-30 days: falsification + internal alpha

Build:

- Lumi protocol + local policy gate;
- Playwright browser executor;
- Cua Driver 0.28.2 adapter over a pinned local binary/MCP boundary;
- deterministic macOS and Windows fixture apps;
- trajectory/evidence schema;
- three real RunLumi workflow packs from target cost-center work.

Release target:

- one-command developer install;
- signed/notarized packaging spike on macOS and signed installer spike on Windows;
- local kill switch;
- no unattended external side effects.

Success threshold: at least 3 workflows, each run 30 times on its target OS with >= 90% end-to-end task success, zero unauthorized side effects, and a documented failure taxonomy. If native desktop automation is not materially improving customer workflows versus browser/API-only execution, stop expanding the desktop layer.

## 30-90 days: customer pilot runtime

Add:

- Tauri 2 shell only for permission onboarding, status, approvals, logs, and update UX;
- Keychain/Credential Manager integration;
- signed updater;
- capability manifests and admin policy;
- redacted evidence bundle;
- exception queue and human handoff;
- cross-platform smoke suite on every release;
- per-workflow cost, intervention, latency, and ROI metrics.

Pilot gate: >= 95% success on stable, well-specified workflow steps; < 10% human intervention on the automated portion; no P0/P1 security event; rollback tested; every side effect attributable to a workflow/action/approval.

## 3-6 months: production hardening

- MDM-friendly deployment;
- device identity and revocation;
- policy signing;
- SBOM and dependency provenance;
- resumable workflows and idempotency keys;
- replayable fixtures for top customer apps;
- background execution where OS/app semantics support it;
- model-provider abstraction and deterministic fallback paths.

## 6-18 months: defensible platform

Own the layers generic computer-use agents will not naturally commoditize:

- workflow economics and cost-center ROI;
- reusable vertical workflow packs;
- customer policy/permission model;
- approvals and exception routing;
- business context and connector graph;
- deterministic app adapters for high-value systems;
- evidence/audit/replay;
- fleet deployment and device trust;
- workflow evals, regression corpus, and reliability data.

Do not spend moat budget on cursor movement, screenshot plumbing, or generic model prompting unless customer evidence shows upstreams cannot meet the requirement.
