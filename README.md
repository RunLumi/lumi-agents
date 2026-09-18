# Lumi Agents

Lumi Agents is RunLumi's local execution layer for reliable, policy-controlled browser and desktop automation on employee computers.

The goal is **not** another generic mouse-and-keyboard agent. The goal is to automate repeatable cost-center workflows inside the software customers already use, while RunLumi owns permissions, approvals, evidence, exceptions, and workflow economics.

## Architecture thesis

Use the least fragile execution surface that can complete the task:

1. **Connector / API**
2. **Browser DOM / accessibility**
3. **Native semantic automation** through macOS Accessibility / Windows UI Automation
4. **Deterministic app adapter**
5. **Vision + coordinates** only as a fallback

The model plans and handles ambiguity. Deterministic code performs actions whenever possible.

## Fastest credible path

For the first customer pilots:

- use **Playwright** for browser-native workflows;
- integrate **Cua Driver** behind a Lumi-owned adapter for native desktop control;
- keep Cua as a pinned, replaceable upstream rather than forking it;
- own the policy engine, approval gates, workflow state, audit/evidence, retries, exception routing, evals, and customer-specific workflow packs;
- run locally on the employee machine for credentials and app access, with cloud orchestration only where policy allows it.

The current upstream candidate is Cua Driver **0.28.2**. It is MIT-licensed and ships macOS and Windows builds. We do not depend on Cua's optional OmniParser/Ultralytics paths.

See `docs/adr/` for decisions and `docs/ROADMAP.md` for the release plan.

## Repository layout

- `crates/lumi-protocol` — stable execution/risk vocabulary.
- `crates/lumi-policy` — local policy decisions and approval requirements.
- `crates/lumi-runtime` — local runtime entry point and future executor adapters.
- `docs/adr` — architecture decision records.
- `docs/evals` — workflow reliability and ROI evaluation contract.
- `docs/workflows` — reusable workflow-pack template.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The Rust toolchain is pinned in `rust-toolchain.toml`.

## Security invariants

- No external side effect without an explicit policy decision.
- Destructive actions require human approval in the default policy.
- Any non-read-only vision action requires approval by default.
- Secrets never belong in prompts, trajectories, screenshots, or logs.
- Upstream binaries must be version-pinned and checksum-verified before shipping.
- Unattended execution needs a local kill switch and revocable server-side lease.

See `SECURITY.md` and `docs/adr/0004-security-boundary.md`.

## Licensing

This repository is currently **proprietary**. Using MIT dependencies does not require RunLumi to license its own product under MIT.

Permissive third-party code may be used when its notices and obligations are preserved. AGPL/GPL dependencies are not approved for the distributed runtime without explicit review. If we later open-source a clean runtime layer, Apache-2.0 is the default candidate because it adds an explicit patent grant; decide that per package rather than licensing the whole product prematurely.

See `LICENSE`, `THIRD_PARTY_NOTICES.md`, and `docs/adr/0003-licensing.md`.
