# Lumi Desktop App

Employee-facing desktop shell built with [Tauri v2](https://v2.tauri.app).
A viewport onto the orchestrator's durable state — the UI does not own
policy, credentials, or executor state.

## Features (spec 15)

- **Dashboard**: task progress with trust-labeled steps (§23.9)
- **Approvals**: business-effect cards with approve/reject (§23.4)
- **Exceptions**: what blocked, safe choices, evidence (§23.5)
- **Evidence viewer**: trust-labeled action history (§23.9)
- **Emergency stop**: one button stops everything (§23.10)
- **Permissions**: OS permission status and onboarding prompts

## Prerequisites

- Rust 1.98+ (the workspace toolchain)
- Node.js 22+ (for the Playwright worker, not the frontend build)
- Tauri CLI: `cargo install tauri-cli --version "^2"`

## Development

```bash
cd apps/desktop
cargo tauri dev
```

The Rust backend compiles from `src-tauri/`. The packaged frontend is the
React/TypeScript app in `ui/`; Tauri builds it into `ui/dist` before packaging.

## Production build

```bash
cargo tauri build
```

Requires signing credentials for distribution (spec 21). The
`lumi-certification` crate gates the release on all checks passing.

## Architecture

The Tauri IPC commands in `src-tauri/src/lib.rs` are thin wrappers that
delegate to the orchestrator/policy/audit crates. The UI is a control
surface, not an authority (§23.17).

Trust language (§23.9): the evidence viewer and task progress show
Planned / Attempted / Executed / Verified / Ambiguous / Failed /
Cancelled as DISTINCT states. "Attempted" is never rendered as "done".
