# Lumi Agents

**A trusted execution operating system for real computer work.**

Lumi Agents is RunLumi's local-first runtime for agentic work across APIs, browsers, files, shells, and native desktop applications.

It is designed for two jobs on one core:

- **Work mode**: substantial general knowledge work, research, files, browser, code, artifacts, and long-running tasks.
- **Workflow mode**: hardened repeatable business automation with policy, verification, evidence, exceptions, and measurable economics.

The goal is not "an AI that can click."

The goal is to make delegation reliable.

## Product thesis

Models reason.

Lumi governs.

Lumi chooses the safest execution surface.

Lumi acts.

Lumi verifies the real outcome.

Lumi records evidence.

Humans handle judgment, relationships, ambiguity, and consequential approvals.

## Execution hierarchy

Prefer the least fragile surface that can complete the work:

1. connector/API;
2. browser semantic automation;
3. native semantic automation;
4. deterministic app adapter;
5. vision/coordinates.

Additional controlled surfaces include local files, sandboxed shell/code execution, and artifact generation.

## Why this architecture

Computer-use primitives will commoditize.

The durable layers are:

- workflow semantics;
- provider-neutral orchestration;
- business context;
- local policy;
- approvals;
- secrets isolation;
- verification;
- evidence/replay;
- reusable skills and workflow packs;
- device/deployment trust;
- evals and reliability data;
- workflow economics.

## Provider-neutral by design

The core runtime must not depend on one model vendor.

Planned adapters include:

- OpenAI;
- Anthropic;
- Gemini;
- Azure OpenAI;
- AWS Bedrock;
- OpenRouter;
- xAI/Grok after contract validation;
- generic OpenAI-compatible endpoints;
- local Ollama/vLLM-style deployments.

Routing is based on tenant data policy, required capability, measured reliability, latency, and **cost per verified successful workflow**.

## Runtime stack

Current foundation:

- Rust core;
- local policy gate;
- Cua Driver behind a Lumi-owned native adapter;
- Playwright as the planned browser-semantic engine;
- cross-platform CI on macOS, Windows, and Linux core;
- dependency/license checks.

Target architecture adds:

- durable task state;
- model router;
- audit/evidence ledger;
- secrets broker;
- postcondition verifier;
- workflow-pack SDK;
- Tauri desktop shell;
- signed updater;
- provider and executor adapters.

## Security invariants

- Local policy is authoritative.
- Models and content cannot grant themselves authority.
- External/destructive actions require explicit policy or approval.
- Secrets are resolved at the executor boundary, not placed in prompts.
- Non-read-only vision actions are conservative by default.
- Workflows verify actual postconditions before reporting success.
- Unattended execution needs a local kill switch and revocable lease.
- Prompt-injected website/document/email instructions are untrusted data.
- Evidence is minimized and must not become continuous employee surveillance.

See:

- `SECURITY.md`
- `docs/security-model.md`
- `docs/adr/0004-security-boundary.md`

## Roadmap

Read `docs/roadmap.md`.

The first milestone is intentionally narrow:

> Three economically valuable workflows, each run repeatedly with verified outcomes, zero unauthorized side effects, and measured before/after economics.

Do not broaden the platform until that works.

## Repository map

Current:

```text
crates/
  lumi-protocol/
  lumi-policy/
  lumi-runtime/

docs/
  roadmap.md
  architecture.md
  security-model.md
  model-providers.md
  workflow-packs.md
  evals/
  adr/
```

Target structure evolves only when real code needs it. Empty framework sprawl is explicitly discouraged.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The Rust toolchain is pinned in `rust-toolchain.toml`.

## Licensing

Lumi Agents is licensed under **Apache-2.0**.

Third-party dependencies retain their own licenses and notices.

See:

- `LICENSE`
- `NOTICE`
- `THIRD_PARTY_NOTICES.md`
- `docs/adr/0003-licensing.md`

The commercial RunLumi control plane, managed fleet, premium workflow packs, hosted analytics, vertical IP, and deployment services may remain proprietary outside this open-core repository.

## Quality bar

We are not trying to ship the most features.

We are trying to make delegating real work feel boringly reliable.

That means fewer surprises, bounded authority, excellent artifacts, graceful recovery, provider freedom, inspectable evidence, and measurable value.
