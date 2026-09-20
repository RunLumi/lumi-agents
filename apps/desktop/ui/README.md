# Lumi desktop UI (React + TypeScript + Tailwind v4)

React 19 + TypeScript (strict) + Tailwind CSS v4 + Vite frontend for the
Lumi desktop app. PR 1 of the conversion (goal.md, 2026-09-20): toolchain,
token bridge, glass recipes, glyph port, typed IPC + mock transport, CI.
Views are ported at parity in the PR 2 cutover; the vanilla `apps/desktop/src`
still ships until then.

## Commands

```sh
npm ci          # exact lockfile install (no scripts run)
npm run dev     # Vite dev server on 1420 (strictPort)
npm run build   # tsc + vite production build → dist/
npm run typecheck
npm run lint
npm run test    # node --test over src/tests/*.test.ts
```

## Design system contract

- Every color, font, radius, and shadow resolves from DESIGN.md §19 tokens
  via `src/styles/index.css` (`@theme` + material recipes). No raw hex in
  components.
- Liquid-glass family: `glass-chrome` / `glass-overlay` / `glass-modal`
  utilities (§8.0, amended 2026-09-20). Glass is floating chrome only;
  content surfaces stay opaque. `prefers-reduced-transparency` and missing
  `backdrop-filter` collapse glass to its opaque twin (§5.3).
- Glyphs: `<Glyph name />` (utility + product) and `<Mark name />` from
  `src/components/glyphs/` — path data ported verbatim from the vanilla
  implementation (ICON.md §7).

## Typed IPC

`src/ipc/` wraps every Tauri command; views never call `invoke()` directly.
`COMMAND_NAMES` in `commands.ts` is cross-checked against
`src-tauri/src/lib.rs generate_handler!` by `src/tests/commandSurface.test.ts`
— renaming or removing a command fails CI on both sides.

Decision record: `tauri-specta` was reviewed and deferred (release-candidate
stability + proc-macro in the release binary); the reviewed alternative is a
hand-maintained surface list enforced by CI. Revisit when tauri-specta hits a
stable major.

## Dependency review (AGENTS.md policy)

| Package | License | Note |
| --- | --- | --- |
| react / react-dom | MIT | UI runtime |
| @tauri-apps/api | MIT (Apache-2.0 dual) | IPC only |
| vite / @vitejs/plugin-react | MIT | build-time |
| tailwindcss / @tailwindcss/vite | MIT | build-time |
| typescript / typescript-eslint / eslint | Apache-2.0 / MIT | build-time |
| @types/node / @types/react / @types/react-dom | MIT | build-time |

The environment's npm policy (`--before 2026-09-13 --ignore-scripts
--save-exact`) is honored: exact versions, no install scripts run.
`npm audit`: 0 vulnerabilities at lock time (vitest 5 deferred — its fix
line postdates the policy window; the @vitest/mocker advisory path
(`vi.mock`) is unused by our tests).

## Dev preview

`?devmock` on the built/dev app selects the mock transport (same typed
interface, fixture data). The vanilla `dev-preview.html` harness remains
until the PR 2 cutover.
