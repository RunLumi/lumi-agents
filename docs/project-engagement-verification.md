# Project engagement verification

Review date: 2026-09-21. Feature PR: [#117](https://github.com/RunLumi/lumi-agents/pull/117); merged as `624e895bcec02b390d428dbc0381f248d2469fa1`.

This is a partial implementation of the broader browser/computer engagement
proposal. The multiline composer, capability discovery, scoped public-page reader,
durable project automations and bounded native adapter are implemented. Full
authenticated Chrome and live native-computer canaries are not. The repository's customer-unattended and V1
NO-GO status remains unchanged.

## Inspected results

| Check | Actual result | Source/revision |
|---|---|---|
| Frontend TypeScript, ESLint, tests and production build | Passed; 95 tests, zero failures | [UI verification run](https://github.com/RunLumi/lumi-agents/actions/runs/35524542816), job 106114309770, tested source `ca5077b14dcb839178e4f163e632cc43be5cc5a9` |
| Network boundaries, real Chromium reader and project UI behavior | Five tests passed, zero failures | Same UI verification run and tested source |
| Selected desktop/browser Rust clippy and tests | Passed; 51 tests passed and one intentionally ignored subprocess helper | [Refinement run](https://github.com/RunLumi/lumi-agents/actions/runs/35524304008), job 106113683393; integrated source `9643aea21cd3b63ecb54546669ef8aade94415bf` |
| Full Rust formatting/clippy/test matrix | Passed on Ubuntu, macOS and Windows | [Core CI run](https://github.com/RunLumi/lumi-agents/actions/runs/35524544999), revision `384a099522c5ec15eb72aef553aceb4fe38b59c2`; subsequent commit changes frontend/translations/tests only |
| Standalone Tauri desktop compilation and clippy | Passed on macOS and Windows | Same core CI run |
| Root and desktop Rust dependency policies | Passed | Same core CI run |
| Browser worker dependency audit | Zero reported vulnerabilities for the pinned worker graph | UI verification run; Playwright `1.63.0`, matching committed package lock |

The UI verification job created and then tested the integrated `ca5077b` source,
so its trigger SHA differs from the code under test. Its temporary integration
workflow was removed in that commit. Permanent checks remain in
`.github/workflows/project-engagement.yml` and the normal repository workflows.
Do not treat older failed UI checks on pre-fix SHAs as the result for `ca5077b`;
do not label a later uninspected CI run as passed either.

The post-merge native adapter slice was verified locally against the official
Cua Driver 0.28.2 macOS arm64 release. The downloaded asset matched the
published SHA-256 `818ddefa0fa8ba2ec9cba837c7aa634a4b064221c748752cf49c5b08e2c94e8c`,
the app was accepted by macOS Gatekeeper with a stapled notarization ticket,
and the standalone binary reported `cua-driver 0.28.2`. Its modern MCP
`server/discover` and `tools/list` protocol responses were inspected, and the
Lumi adapter passed 26 native/contract tests plus 29 desktop unit tests, 3
connection tests and 6 runner tests. No OS permissions were granted and no
native app was mutated during this probe.

## What these tests establish

The real browser canary opens a controlled page through Chromium, observes its
text and links, checks that page scripts do not execute, and refuses unsupported
interactive operations and destinations. Network tests exercise actual local
fixture sockets and public-address/origin admission rules.

The project UI test uses Chromium against the real React UI with an explicitly
installed fixture IPC transport. It verifies multiline Enter behavior, no
accidental submission, project identity, draft preservation across navigation,
Cmd/Ctrl-style submission, unavailable Chrome selection, schedule seeding,
timezone persistence and pause controls. It is not evidence of a live model
completing a business task through Tauri.

Rust tests cover schedule/timezone behavior, duplicate request identity, local
store ownership, interrupted occurrence handling, blocked-schedule visibility,
revision conflicts and browser-handle lifecycle behavior. A passing compilation
matrix is not signed macOS/Windows release certification.

## Remaining blockers and warnings

- The contribution-origin check reports missing matching DCO sign-offs for PR
  commits. [Inspected licensing run](https://github.com/RunLumi/lumi-agents/actions/runs/35524306743),
  job 106113690662, explicitly requires genuine authorized-contributor sign-offs.
  No sign-offs were synthesized, and no contribution/licensing gate was weakened.
  The PR remains draft and unmerged for maintainer review.
- Frontend `npm ci` reports two moderate-severity dependency advisories and
  deprecated transitive packages. The worker audit's zero count does not cover
  the frontend graph. The production build also warns about large bundles.
  These warnings have not been fixed or represented as a clean security audit.
- Authenticated Chrome attachment is now wired through an explicit selected
  PID/window and project origin ceiling, with read/navigate-only task support.
A live authenticated Chrome canary, browser mutations, typing, uploads and
downloads remain unvalidated. Native computer execution is wired through the
verified Cua adapter for interactive tasks, but no live app canary or Windows
executable pin has been validated.

On 2026-09-21, a read-only canary used the official checksum-verified Cua
Driver 0.28.2 temporary binary to enumerate host windows. Chrome was running,
but no eligible visible Chrome window was available: all reported Chrome
surfaces were off-screen or had degenerate bounds. No existing profile was
attached, no page content was read, no permission was granted, and the
temporary driver was stopped and removed. A normal user can complete the
canary after opening/selecting a visible Chrome window in Project Settings.
- Provider recovery is now opt-in through the existing OS-backed broker on macOS
  and Windows; Linux remains session-only. No cloud runner or OS background
  service is implemented here.
- Attachments, live takeover, complete approval interaction, global cross-project
  automation aggregation, and event/webhook triggers remain outside this change.
- No live provider plus browser plus verified file workflow, authenticated
  customer canary, sustained unattended deployment, or signed installer was
  validated in this work.

See [project engagement](project-engagement.md) for the implemented user journey,
permission boundaries, local execution requirements and crash/recovery behavior.
