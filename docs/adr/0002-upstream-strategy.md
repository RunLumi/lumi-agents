# ADR 0002: Upstream computer-use strategy

- Status: Accepted for MVP
- Date: 2026-09-18

## Decision

Do **not** fork a general computer-use project for the first release.

Use Cua Driver as the primary native desktop upstream behind a Lumi-owned adapter. Pin the exact release and verify checksums. The current candidate is `cua-driver-rs-v0.28.2`.

Use `open-codex-computer-use` as a reference and interoperability/behavior comparison, not as the production dependency initially.

## Evidence behind the choice

Cua Driver has a Rust cross-platform core, macOS/Windows platform crates, MCP/CLI/SDK surfaces, permission modes, and stable release artifacts for macOS and Windows. Its workspace declares MIT.

The open-codex-computer-use project is also MIT and useful. Its architecture documents a mature Swift/macOS path but describes the Windows and Linux runtimes as experimental, with Windows currently using a Go wrapper plus a PowerShell UI Automation bridge and lacking installer/onboarding/code signing. That makes it valuable reference code but a weaker default production dependency for Lumi's cross-platform MVP.

## Boundary

Lumi owns a small `NativeDesktopExecutor` contract. Cua-specific protocol details stay inside the adapter. No workflow definition may depend directly on Cua tool names.

## Fork trigger

Fork only if at least one of these becomes true:

- a paid customer blocker cannot be solved through the adapter and remains unresolved upstream across two stable releases;
- a security boundary requires code changes upstream will not accept;
- measured latency/reliability requires a material architectural change;
- upstream licensing changes incompatibly.

Until then, contribute fixes upstream where practical and keep our differentiation above the commodity input-control layer.
