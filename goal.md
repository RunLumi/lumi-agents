/goal

# MISSION

Rebuild the Lumi desktop frontend (`apps/desktop`) as a **React + TypeScript +
Tailwind + shadcn/ui** application with a modern **liquid-glass material
system**, restoring every existing view at feature parity, while keeping the
Lumi Design System (`DESIGN.md`, `ICON.md`) as the single source of visual
truth.

The target result:

> Same product. Same information architecture. Same behavior. Same trust
> surface. Rendered by a typed, component-based React app whose chrome
> surfaces use a disciplined liquid-glass material family, with every token,
> recipe, glyph, and interaction still governed by DESIGN.md and ICON.md.

Do not implement a demo.

Do not redesign the product's information architecture.

Do not fork the design system into two sources of truth.

Do not regress the Spec 26 backend slice; runtime work on #50 (dogfooding,
model planning) continues separately.

Deliver a complete, verified conversion: every current screen, state, and
keyboard path rebuilt on React, the vanilla implementation retired, CI green.

---

# 1. DECISION RECORD

Record this block in the PR description (AGENTS.md feature test).

* **Outcome:** Lumi desktop contributors and future feature velocity. The
  vanilla single-file frontend (`app.js`, ~1,800 lines, 58 event listeners)
  is at the inflection point where manual DOM sync becomes the maintenance
  bottleneck as Work-mode views multiply.
* **Evidence:** Observed structure (18 sections, hand-rolled routing/state/
  render in one file); approved decision 2026-09-20: React chosen over
  Svelte/Solid for long-horizon stability discipline and ecosystem depth;
  Tauri's local webview neutralizes React's usual costs (bundle size,
  hydration). shadcn/ui chosen because its components are copied, auditable
  MIT source (no runtime dependency sprawl) on Tailwind, which DESIGN.md §20
  already anticipates.
* **Smallest solution:** Lift-and-shift conversion at parity. No new product
  features, no IA changes, no Rust behavior changes. The only additions are
  the frontend toolchain, typed IPC bindings, and the glass material family
  (one deliberate extension of §8.0).
* **Proof:** Per-view parity checklist against `docs/screens/*.jpg` and the
  behavior inventory; typed coverage of every Tauri command/event; CI green
  including new frontend checks; refreshed screenshots.
* **Cost and exit:** Adds npm toolchain + lockfile to a cargo-only repo, new
  CI checks, and backdrop-filter GPU cost bounded by rules in §5 below.
  Rollback: revert the cutover PR; vanilla `src/` is deleted only in that PR
  and remains recoverable in git history. Stop/continue threshold: if typed
  IPC bindings or glass rendering cannot meet the constraints on either
  WebKit or WebView2, stop and reduce (fall back to solid Sheet materials)
  rather than widen the CSP or ship degraded contrast.

---

# 2. SOURCE OF TRUTH

Before changing code, read the current repository state on `main`.

Read at minimum:

* `AGENTS.md` (feature test, dependency policy, release gates)
* `DESIGN.md` — especially:
  * §5 Color System, §6 Typography, §7 Layout System
  * §8.0 Material Recipes, §8.1 Radius, §8.3 Shadows
  * §9 Iconography, §10 UI Components, §11 Product Surfaces
  * §14 Motion Design, §16 Accessibility
  * §18 App Layout, §19 Design Tokens, §19.1 Cross-Platform Token Parity
  * §20 Tailwind Theme Guidance, §21 Anti-Patterns, §24 Design QA Checklist
* `ICON.md` — the Lumi Glyph System (utility / product / marks, one 20×20
  construction)
* `docs/screens/*.jpg` — current shipped UI; this is the **parity baseline**
* `docs/specs/v1/26-project-workspace-folder-as-project.md`
* `docs/specs/v1/23-user-experience-handoff.md`
* `apps/desktop/src-tauri/src/` — the real command/event surface
* `apps/desktop/src-tauri/capabilities/default.json`
* `apps/desktop/ui/src/` — the packaged frontend and behavior contract
* `apps/desktop/ui/src/ipc/mock.ts` — the devmock contract to preserve
* `README.md` — core checks

Do not assume screenshots or docs are aspirational. They describe what ships.

---

# 3. PARITY IS THE CONTRACT

`docs/screens/*.jpg` define the target:

```text
home.jpg
project-home.jpg
project-tasks.jpg
project-files.jpg
project-changes.jpg
project-git.jpg
project-artifacts.jpg
project-evidence.jpg
project-approvals.jpg
project-settings.jpg
```

The screenshots govern **information architecture, layout, content, and
behavior per screen** — not final pixels.

Final pixels are governed by DESIGN.md plus the glass material family
defined in §5 below.

For every view, port from `app.js` before restyling:

* all states: loading skeleton, empty, error, partial, permission-denied
* all interactions: routing, collapsible sidebar, command palette, dialogs,
  context menus, hover/focus/active treatments
* all keyboard behavior: tab order, shortcuts, palette invocation, escape
  handling, focus traps and returns
* all Tauri IPC calls and their error handling
* borderless window chrome: drag regions, traffic-light spacing, resize
  behavior (`decorations: false` stays)

"Empty means empty." Runtime-derived state only. The only fixture data in
the codebase is the devmock transport (§8).

---

# 4. TARGET STACK

Locked decisions. Do not substitute without recording why in the PR.

```text
Framework        React 19 + TypeScript (strict)
Bundler          Vite
Styling          Tailwind CSS v4, theme built from DESIGN.md §19 tokens
Components       shadcn/ui (Radix primitives, copied source via CLI)
Routing          internal state routing ported as-is (no router library
                 unless a real need emerges; keep the current route model)
IPC              @tauri-apps/api from npm; typed bindings via tauri-specta
Package manager  npm, package-lock.json committed
Location         apps/desktop/ui/  (sibling of src-tauri/)
Node             pinned via engines + CI matrix
```

`tauri.conf.json` changes:

```json
"build": {
  "frontendDist": "../ui/dist",
  "devUrl": "http://localhost:1420",
  "beforeDevCommand": "npm --prefix ../ui run dev",
  "beforeBuildCommand": "npm --prefix ../ui run build"
}
```

Vite: port 1420, `strictPort`, `clearScreen: false`, outDir `dist`.

`withGlobalTauri` may be dropped once nothing references `window.__TAURI__`
outside the devmock transport.

Keep the CSP as strict as today. Do not add remote sources. Tailwind output
and Vite assets are self-hosted; `style-src 'unsafe-inline'` already covers
what inline styles need. If anything else appears to need CSP widening,
stop and redesign instead.

Replace `withGlobalTauri`-era globals with `@tauri-apps/api` imports.

---

# 5. LIQUID GLASS MATERIAL SYSTEM

Goal: modern translucent "liquid glass" chrome — frosted, layered, alive —
without becoming a one-off soup of blur effects.

## 5.1 Amend DESIGN.md §8.0, not the components

Add ONE new material family to the §8.0 recipe table. Every glass surface
uses one of these recipes; inventing per-component materials is prohibited
(§21 Anti-Patterns applies). Suggested family:

```text
Glass chrome   translucent --color-surface-white + --glass-blur
               1px --glass-border, radius follows §8.1,
               --shadow-card        → sidebar, side panels
Glass overlay  same base, higher opacity + --glass-blur-strong
               --shadow-overlay     → popovers, menus, tooltips
Glass modal    same base, strongest tier
               --shadow-modal       → dialogs, command palette
```

Content surfaces (cards, tables, inputs, lists) **stay on the existing
opaque Sheet/Recessed-well recipes**. Glass is for chrome that floats above
content, not for content itself. The canvas glow remains canvas-only.

DESIGN.md now defines this family directly (amended 2026-09-20): the §8.0
glass recipe rows (Glass chrome / Glass overlay / Glass modal), the §19
`--glass-*` tokens, and the §21 scoping that lifts the glassmorphism ban
for floating desktop chrome only. Consume those definitions; do not
re-derive values in code.

If implementation forces a recipe or token change — a contrast failure, a
WebView2 performance limit, a fallback gap — update DESIGN.md in the same
PR. The doc stays the contract; code follows it.

## 5.2 Rendering rules

* `backdrop-filter: blur() saturate()` only; no SVG filters, no
  canvas compositing tricks.
* Blur budget: cap concurrent blurred surfaces per screen (target ≤ 3
  visible layers). Never nest backdrop-filter inside backdrop-filter.
* Specular top edge: one subtle 1px light gradient border/inset highlight
  per glass surface. No glow on components (§8.0 rule stands).
* Motion respects §14; glass surfaces may ease opacity/transform on
  enter/exit. No continuous animation.

## 5.3 Degradation and accessibility — non-negotiable

* `@media (prefers-reduced-transparency)`: glass recipes collapse to the
  existing opaque recipes. Same for any environment where backdrop-filter
  is unavailable — detect and fall back, never ship transparent-without-blur.
* `@media (prefers-reduced-motion)`: §14 rules apply.
* Text on glass must still pass §16.1 contrast. Verify every glass
  surface at both window backgrounds (empty state and dense content
  underneath). If contrast fails, raise opacity until it passes.
* Focus rings on glass surfaces must stay visible (§16.5).

## 5.4 Performance and platforms

* Verify on macOS (WKWebView) and Windows (WebView2); both are release
  targets and render backdrop-filter differently. Measure scroll and
  palette-open latency at the minimum window size (800×600) and the
  default (1200×800).
* If WebView2 performance is unacceptable, reduce blur radius before
  removing glass; document the measured decision.

---

# 6. DESIGN TOKEN BRIDGE

Build the Tailwind theme mechanically from DESIGN.md §19:

* One mapping, reviewed once: every `--*` token → Tailwind theme key,
  including light values only for now unless §19 defines dark tokens.
* §19.1 parity contract holds: a token that exists must resolve to the
  same value Tailwind serves and plain CSS serves.
* No hardcoded colors, radii, shadows, spacing, or font sizes in
  components. Everything resolves through tokens. `grep`-able rule: no
  raw hex in `ui/src` outside the token definition file.
* shadcn/ui theming variables are aliased to Lumi tokens (§20 guidance),
  so copied shadcn components inherit the design system instead of
  importing a foreign palette.

# 7. GLYPH SYSTEM PORT

Port `ICON.md` and the `UTILITY_GLYPHS` / product / marks path data from
`app.js` into React components unchanged:

* One `<Glyph name />` API (utility + product) and `<Mark name />`,
  built on a shared 20-grid, `currentColor`, `aria-hidden` by default with
  accessible labels at the call site.
* Path data is copied verbatim from `app.js` — this is a port, not a
  redraw. Geometry, stroke widths, and optical sizes stay identical.
* Brand marks continue to load from `src/assets/brand/` (move under
  `ui/src/assets/brand/`).
* SVGs must keep intrinsic sizes (known prior IPC/SVG pitfall: sizeless
  SVGs collapse layouts).

# 8. TYPED IPC AND THE MOCK TRANSPORT

* Inventory every Tauri command and event the frontend uses, from
  `src-tauri/src/` — not from memory. The inventory lives in the PR.
* Add `tauri-specta` (or, if it proves incompatible, a reviewed codegen
  alternative) so command names and payload types are generated from Rust
  and consumed in TypeScript. Renaming a command or changing a payload
  must become a compile error on both sides.
* Rust changes are limited to binding registration/annotations. No
  command behavior changes. `cargo clippy` and `cargo test` stay green.
* One `ui/src/ipc/` layer wraps all invokes. Views never call
  `invoke()` directly.
* Preserve the devmock: `?devmock` query param loads a mock transport
  implementing the same typed interface with fixture data (`ui/src/ipc/mock.ts`).
  Never wired in the packaged app: no query parameter, no fixture.
* Preserve known IPC pitfalls (from prior work): Rust methods that return
  nothing are not serialized (compute client-side); async views need
  skeleton + route guard so stale async results never render.

# 9. MIGRATION ORDER

Port section-by-section from `app.js` (its section headers map to
components). In this order:

```text
1.  App shell: window chrome, drag regions, collapsible sidebar, routing
2.  Projects home (recent projects, open folder)
3.  Project shell + home tab
4.  Tasks
5.  Files
6.  Changes
7.  Git
8.  Artifacts
9.  Evidence
10. Approvals
11. Settings
12. Command palette
```

Per-section discipline:

* Port behavior first, restyle with glass second. Never both blind.
* Each view is a pure function of the state store; port the central state
  block into typed stores, keep render derivations pure.
* The section is done when its parity checklist passes (§3) and its
  screenshots match structure (glass styling may differ).

# 10. DO NOT

* Do not change information architecture, navigation model, or add features.
* Do not introduce sample/demo data into production paths.
* Do not weaken accessibility to achieve the glass look.
* Do not widen the CSP, add remote fonts, or load any runtime asset from
  the network.
* Do not invent materials outside the amended §8.0 table.
* Do not hardcode design values outside the token definition.
* Do not copy shadcn demo styling wholesale — components are theming hosts
  for Lumi tokens, not the visual source of truth.
* Do not add a component library beyond shadcn/Radix without the
  AGENTS.md dependency test.
* The React cutover is complete; do not reintroduce the retired vanilla preview
  surface under `apps/desktop/src/`.
* Do not touch workflow/policy/runtime crates beyond IPC binding
  registration.

# 11. DEPENDENCY POLICY

Per AGENTS.md: exact need, alternatives, maintenance, provenance/license,
security exposure, replaceable boundary — reviewed before each addition.

```text
react / react-dom        MIT
vite, @vitejs/*          MIT
tailwindcss              MIT
shadcn/ui components     MIT, copied source, reviewed per component
@radix-ui/*              MIT, transitive via shadcn — keep the set minimal
@tauri-apps/api          MIT (matches Tauri)
tauri-specta             review license + maintenance before adopting
```

Lock everything (`package-lock.json`). Record licenses in
`THIRD_PARTY_NOTICES.md` where required. No post-install scripts beyond
known-safe tooling; review every dependency's install scripts.

---

# 12. CI AND QUALITY GATES

Existing gates must stay green on every PR:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Add frontend gates and wire them into CI:

```sh
npm ci && npm run typecheck
npm run lint
npm run test        # component/logic tests for stores, IPC layer, routing
npm run build      # must succeed with production flags
```

* Vite production build must be what `frontendDist` serves. No dev-mode
  React in packaged builds.
* CI workflow additions pin Actions by commit SHA (existing policy).
* Record platform limits honestly (e.g., WKWebView glass verification
  requires macOS; CI can cover build/typecheck only).

---

# 13. PR STRATEGY

Coherent vertical PRs; each leaves `main` coherent.

```text
PR 1  Foundation: apps/desktop/ui toolchain, token bridge, glass recipes
      + DESIGN.md amendment, glyph port, typed IPC + mock transport,
      CI wiring. Completed before the React cutover.

PR 2  Cutover: all views migrated per §9, frontendDist switched,
      vanilla src/ and dev-preview.html/.js removed, screenshots
      refreshed (docs/screens convention: jpg via scripts/png-to-jpg.sh,
      old set moved to archived/), README/docs updated.

PR 3+ Polish: measured performance fixes, QA checklist findings,
      Windows WebView2 adjustments.
```

The migration branch is `feat/react-shadcn-rebuild`. PR 2 is one
atomic cutover: half-React half-vanilla never ships.

---

# 14. DEFINITION OF DONE

* [ ] Every view in §9 ported with all states, interactions, and keyboard
      behavior; parity checklist per view recorded in the PR.
* [ ] Glass implementation matches the DESIGN.md §8.0 recipes and §19
      `--glass-*` tokens exactly (any adjustment reflected in DESIGN.md in
      the same PR); degraded modes verified per §8.0/§24.
* [ ] Degradation verified: reduced-transparency, reduced-motion, no-
      backdrop-filter fallback, contrast at minimum window size.
* [ ] Every Tauri command/event typed; views use the `ipc/` layer only.
* [ ] devmock transport works via `?devmock`; packaged app contains no
      fixture data.
* [ ] Frontend gates (typecheck, lint, test, build) in CI and green.
* [ ] All existing CI checks green on the exact PR head.
* [ ] Verified on macOS; Windows WebView2 result recorded honestly
      (verified or explicitly listed as unverified with reason).
* [x] `apps/desktop/src/` vanilla code removed in the cutover PR.
* [ ] Screenshots refreshed per repo convention; old set archived.
* [ ] No sample data in production UI; empty means empty.
* [ ] Dependency review recorded; licenses/notice files updated.
* [ ] Rollback path stated: revert the cutover PR restores the previous release
  artifact; the retired preview source is not reintroduced.

---

# 15. WORK AUTONOMOUSLY

Proceed long-horizon. Inspect, implement, test, screenshot, verify, iterate.

You are authorized to: create branches; implement; add the toolchain;
update DESIGN.md/ICON.md where this mission says the system genuinely
changes; write tests and fixtures (devmock only); open PRs; fix CI; merge
when gates are green and repo policy permits.

A blocker is valid only when work requires an external input (e.g., OS
signing, a design decision genuinely outside this mission's scope). Record
it in the BLOCKER format and continue unblocked work.

Stop and reduce scope if: glass cannot meet contrast or performance rules
on a release platform, typed bindings cannot cover the command surface, or
a parity gap would require a behavior change to resolve. Reduce to the
opaque recipes / manual types / narrower scope — never ship the violation.

---

# NORTH STAR

Make the conversion boring:

> Launch the app and it is unmistakably the same Lumi — same jobs, same
> trust, same speed — now rendered by a codebase where every pixel traces
> to a token, every call traces to a type, and the chrome floats like
> glass without costing a millisecond.

**Same product. Better bones. Glass where it floats, paper where it works.**
