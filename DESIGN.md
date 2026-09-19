# Lumi Design System

## 0. Brand Summary

**Product name:** Lumi
**Vietnamese sales name:** Lumi AI Workspace
**Product category:** AI operating workspace for SME teams and CEOs
**Core product wedge:** Daily Pulse → Operating Memory → CEO Morning Brief
**Primary URL for MVP:** `lumi.locuno.com`

Lumi turns clear SME updates into a calm, evidence-based CEO morning brief: an operating assistant
for stuck, late, risky, or decision-bound work, not a chatbot, task clone, or surveillance tool.

---

# 1. Design Philosophy

## 1.1 Core Positioning

> Calm clarity for running a company.

Lumi is a calm, serious, evidence-led civic editorial command surface: accountable and respectful
of human judgment, never flashy, toy-like, surveillance-oriented, crypto/cyberpunk, childish, or
templated.

## 1.2 Visual Direction

**Civic Editorial Intelligence** applies Civic Editorial Minimalism to an AI workspace: institutional
trust, editorial clarity, warm human texture, precise structure, high signal, and no spectacle,
AI gimmicks, or decorative noise. It must satisfy a CEO, team member, and top designer alike.

## 1.3 The Exceptional Standard

Lumi must be good enough that Silicon Valley comparisons stop mattering:

* **Immediate trust:** a CEO believes the morning signal before understanding the system.
* **Quiet authority:** composed, edited, inevitable, never decorated.
* **Operational warmth:** serious, not cold, for teams in Vietnam, Malaysia, Indonesia,
  Saudi Arabia, the Philippines, Latvia, and markets like them.
* **Native confidence:** web, iOS, Android, Mac, and Watch are one product expressed
  through each platform's best behavior, never a stretched viewport.
* **Unconventional restraint:** show fewer things in a better order with stronger evidence,
  not a visual gimmick.

Every screen must make a real operating meeting calmer and more decisive; otherwise it is not done.

## 1.4 The Product Metaphor

Lumi is a **lit operating desk**:

* Warm paper background: the desk surface.
* White cards: documents placed on the desk.
* Blue actions: signed authority.
* Amber and red rails: attention marks in the margin.
* Evidence chips: source notes attached to the claim.
* The clarity mark: Lumi found or verified something useful.

Never replace it with glass panels, neon depth, floating dashboards, or chatbot theatrics:
intelligence is work ordering, not visual effects.

## 1.5 Civic Material Intelligence

The visual language is **Civic Editorial Minimalism + Material Intelligence**: civic-service
seriousness, editorial hierarchy, and high-end native tactility without glassmorphism, aurora
gradients, AI glow, or signal-competing decoration. Every visible object needs a physical reason:

| Material cue | Purpose | Allowed expression |
|---|---|---|
| Paper | calm reading, trust, warmth | `Paper White` canvas, slight top light |
| Sheet | grouped evidence, decisions, records | white surface, 1px border, lit top edge |
| Ink | authority and hierarchy | Civic Navy / Ink text, not black |
| Mark | attention and provenance | rail, badge, source chip, restrained clarity mark |
| Lift | interactivity or overlay plane | navy-tinted token shadow only |
| Well | input / capture area | recessed inner shade, white fill |
| Seal | primary authority | Lumi Blue button, active state, selected state |

Material is never decorative: remove any shadow, border, tint, or rail that does not explain state,
hierarchy, affordance, or provenance.

The serious-SaaS standard: less chrome and more judgment; fewer panels and better-ranked
information; more evidence and less performance; tactile enough to trust, quiet enough for daily use.

---

# 2. Logo Interpretation

The logo is a geometric folded **L-form**: two planes divided by a precise diagonal channel, deep
institutional blue, strong negative space, and an architectural silhouette.

## 2.1 What the logo communicates

It communicates structure and an operating system (**L-form**); work aligned (**two planes**);
movement, handoff, and progress (**diagonal channel**); trust and calm intelligence (**blue**);
simplicity (**open space**); and precision rather than decoration (**sharp geometry**).

## 2.2 Logo usage principles

Use it with restraint.

Do:

* Give the mark generous whitespace on warm paper or white.
* Keep the four-point clarity mark separate and subtle for verified insight.
* Use L geometry to inspire layouts, corners, dividers, and section framing.

Do not:

* Add decorative gradients beyond the supplied primary app-icon treatment
* Add AI glow
* Slant or italicize the `umi` wordmark; its outlined letters are upright
* Separate the folded-L and `u` by more than one wordmark stem width; the canonical lockup uses
  approximately 0.8× stem width
* Add glassmorphism
* Add drop shadows
* Add animated sparkle effects
* Place the logo on noisy backgrounds
* Merge the separate clarity motif back into the folded-L logo

## 2.3 Logo clear space

Minimum clear space equals the width of the diagonal channel. Use the mark alone for small app
icons.

For product headers, use:

```text
[Logo mark] Lumi
```

For Vietnamese sales material, use:

```text
[Logo mark] Lumi AI Workspace
```

---

# 3. Brand Keywords

Design filters: clear, calm, reliable, precise, evidenced, human, operational, executive,
trustworthy, warm, focused, quietly intelligent. Never direct visually toward futuristic, magical,
viral, flashy, neon, robotic, cyber, gamified, trendy, or hype.

---

# 4. Core Design Principle

## 4.1 The Mass-Love Layer

Most users should instantly feel:

> “This is clear, useful, and trustworthy.”

Require readable type, obvious hierarchy, familiar layouts, no visual stress, hidden navigation, or
overdesigned interactions, and no cleverness that blocks understanding.

## 4.2 The Designer-Praise Layer

Top designers should notice:

> “This is restrained, consistent, and unusually well-edited.”

Require strong typographic rhythm, whitespace, intentional grids, subtle paper texture, and
logo-derived icons, never generic SaaS gradients, AI stock illustration, or a template feel.

## 4.3 The CEO Layer

CEOs should feel:

> “This helps me see what matters without micromanaging.”

Prioritize brief, risk, decision, and operating signal; avoid task-board clutter; rank and group
information with evidence so every section answers: “What should I know or decide?”

---

# 5. Color System

> **Light-only — no dark mode.** Warm paper and civic ink are the brand. Web, iOS,
> iPadOS, macOS, watchOS, and Android have no dark theme or `prefers-color-scheme`
> branch: web sets `color-scheme: light` and keeps Tailwind `dark:` inert with a
> never-applied `.dark`; SwiftUI forces `.preferredColorScheme(.light)` and
> `Info.plist` `UIUserInterfaceStyle = Light`; Android `LumiTheme` always resolves
> a light `ColorScheme` and `values-night` mirrors it. Never restore dark palette.
>
> **One source of truth.** The token table in §19 is canonical. Every platform
> mirror (`frontend/src/index.css`, `ios/Lumi/UI/Shared/LumiKit.swift`,
> `ios/LumiMac/Theme/LumiColors.swift`, `ios/LumiWatch/Theme/WatchColors.swift`,
> `android/.../theme/Color.kt`) must match it byte-for-byte — see the
> **Cross-Platform Token Parity** contract in §19.

## 5.1 Primary Palette

### Lumi Blue

```css
--color-lumi-blue: #006093;
```

Lumi Blue is deep cerulean, ocean ink, rebranded from cobalt `#0E3EB8` on 2026-07-09.
Cobalt had become generic AI/cloud/payments blue; cerulean keeps civic trust and reads as
operational intelligence, not an AI toy.

Use for:

* Logo
* Primary buttons
* Key links
* Active navigation
* Selected states
* Important chart lines
* Executive brief highlights

Personality:

* Trust
* Governance
* Reliability
* Operating clarity

---

### Civic Navy

```css
--color-civic-navy: #102A43;
```

Use for:

* Headlines
* Body text
* Navigation labels
* Dense UI text
* Executive brief titles

Personality:

* Serious
* Editorial
* Stable
* Professional

---

### Paper White

```css
--color-paper-white: #F4F0E8;
```

Use for:

* Main app background
* Landing page background
* Report surfaces
* Empty states

Do not default to pure white.
Warm paper makes the product feel more mature and less like a generic SaaS dashboard.
The paper is deliberately ~3 L\* below the white card surface (`#F4F0E8` vs `#FFFFFF`)
so cards read as **lit sheets resting on paper**, giving the `shadow-card`/elevation
system the tonal contrast it needs to register. Paper too close to white (the earlier
`#FAF8F4`) collapsed that depth into white-on-white — the exact generic-SaaS look §1.1
rejects.

---

### Surface White

```css
--color-surface-white: #FFFFFF;
```

Use for:

* Cards
* Modals
* Tables
* Input backgrounds

---

### Archive Gray

```css
--color-archive-gray: #E7EAF0;
```

Use for:

* Borders
* Dividers
* Table lines
* Quiet UI structure
* Disabled states

---

### Soft Slate

```css
--color-soft-slate: #5E6677;   /* AA ≥4.5:1 on paper */
```

Use for:

* Secondary text
* Timestamps
* Helper text
* Metadata

---

### Signal Amber

```css
--color-signal-amber: #F4A62A;
```

Use only for:

* Warnings
* Overdue flags
* Important notices
* Cash/payment risks
* Items needing attention

Limit amber to 5% of the interface.

---

### Risk Red

```css
--color-risk-red: #C2410C;
```

Use sparingly for:

* Critical risk
* Failed states
* Dangerous actions
* Data errors

Do not use red for normal urgency. Use amber first.

---

### Success Green

```css
--color-success-green: #1F7A4D;
```

Use for:

* Completed work
* Resolved risks
* Confirmed decisions
* Positive status

Avoid bright SaaS green.

---

## 5.2 Extended Palette

```css
:root {
  --color-lumi-blue: #006093;
  --color-lumi-blue-hover: #004F80;   /* blue-700 — perceptual hover step */
  --color-lumi-blue-active: #003E6A;  /* blue-800 — pressed */
  --color-lumi-blue-soft: #E4F3FC;

  --color-civic-navy: #102A43;
  --color-ink: #172033;
  --color-soft-slate: #5E6677;        /* AA ≥4.5:1 on paper (nudged from #667085) */

  --color-paper-white: #F4F0E8;
  --color-surface-white: #FFFFFF;
  --color-archive-gray: #E7EAF0;
  --color-border-subtle: #E7EAF0;
  --color-border: #D9DEE8;
  --color-border-strong: #C8D0DE;

  --color-signal-amber: #F4A62A;
  --color-amber-soft: #FFF4DC;

  --color-risk-red: #C2410C;
  --color-red-soft: #FFF1EC;

  --color-success-green: #1F7A4D;
  --color-green-soft: #E9F7EF;
}
```

---

## 5.3 Tonal Ramps & Wide-Gamut

Every brand hue is generated from a single **OKLCH ramp** (`50`→`950`), not
hand-picked per shade. OKLCH is perceptually uniform, so equal numeric steps
look like equal visual steps — hovers, pressed states, chart series, and
elevation tints all derive from the same ladder and never clash. Each brand
anchor is pinned at the exact ramp step that round-trips to its original sRGB
hex with zero drift (e.g. `lumi-blue = blue-600 = #006093`,
`hover = blue-700 = #004F80`, `active = blue-800 = #003E6A`). The blue ramp is
authored at hue ~235: the anchors round-trip exactly, and the deepest steps sit
slightly bluer after sRGB gamut mapping — expected, not drift. The full ramps
live in `frontend/src/index.css`; treat them as the generator, the §19 hexes as
the contract.

**Wide-gamut (Display-P3).** Because the ramps are authored in `oklch()`, modern
P3 displays — every recent iPhone, iPad, and Mac — render the brand at a fuller
chroma than sRGB can express, automatically, with **no hue shift**. The handful
of soft tints still pinned to sRGB hex are lifted on P3 via a single
`@media (color-gamut: p3)` block. sRGB displays keep the exact §19 values: this
is pure progressive enhancement, never a different design. This is a deliberate
edge most products skip — colour that is *more alive on better screens* without
ever being wrong on ordinary ones.

## 5.4 Color Temperament

Lumi color should feel expensive because it is controlled.

Rules:

* **Blue is authority.** Use Lumi Blue for the one action, active navigation,
  selected state, and trusted insight. Do not wash whole screens in blue.
* **Paper is material.** `Paper White` is not beige lifestyle branding; it is a
  calm reading surface. Pair it with white cards, navy text, and lit shadows so
  the app feels like documents on a desk.
* **Amber is attention.** Amber means "look here soon." It is the default for
  overdue, waiting, and business-risk states that are not catastrophic.
* **Red is rare.** Red means destructive, failed, blocked, critical, or legally
  dangerous. If everything is red, nothing is urgent.
* **Green is closure.** Use green for resolved, completed, paid, approved, and
  all-clear states. Do not use bright gamified green.
* **Gray is structure.** Archive Gray and Border provide document structure. They
  should never become a cold enterprise-gray theme.

Color budget per screen:

| Color role | Budget |
|---|---:|
| Paper / white / navy / slate | 85-95% |
| Lumi Blue | 3-8% |
| Amber / red / green combined | 1-6% |
| Decorative color | 0% |

Any new hue needs a product reason, a semantic name, token parity across
platforms, and contrast verification. "It looks nice" is not a reason.

## 5.5 Dark-ground status tints

The product is light-only (§5), but sales and print surfaces occasionally invert
one panel to Civic Navy to show the assistant's own output. Status colour still
has to survive that inversion: on `#102A43`, `risk-red` measures 2.83:1 and
`success-green` 2.75:1, both far under AA, so a status label rendered in them is
unreadable and the colour stops carrying meaning.

Three tints exist for that ground only. They are the **same semantics**, lifted
for legibility, never new meanings and never a second status palette:

| Token | Hex | On navy | Replaces |
|---|---|---:|---|
| `--color-amber-on-navy` | `#F4A62A` | 7.21:1 | `signal-amber` (unchanged; already passes) |
| `--color-red-on-navy` | `#FF9E7A` | 7.27:1 | `risk-red` |
| `--color-green-on-navy` | `#6FD39F` | 8.01:1 | `success-green` |

Rules:

* Use them **only** on a Civic Navy surface. On paper or white, the §5.2 hues
  remain the only correct choice.
* They are not a dark mode and do not license one. A navy panel is a single
  editorial inversion, not a theme.
* The saturated hue still carries the rail or dot; the tint carries the label
  text. Never invert that pairing.
* Any additional dark-ground tint needs the same measured contrast in this table
  before it ships.

---

# 6. Typography

## 6.1 Typeface — one brand family, two voices by weight

Type is **one family, Geist**, spoken in two voices by *weight and tracking* — a
tighter, heavier voice for the moments that carry trust, a lighter voice for dense
UI. The single family is the point: Geist reads as a *calm operating system from
the near future*, not enterprise documentation. It replaced the old IBM Plex Sans +
Inter pairing on 2026-07-09 (rationale: `docs/improvements/2026-07/week-28/geist-font-system.md`):
Plex signalled "IBM cloud / consulting deck" and Inter had become the
default-SaaS-template face — both too safe to define a new category. Geist is sharper, quieter, more "future infrastructure," and one coherent
family is cleaner than two.

| Role | Face | Voice | Weight · tracking |
|------|------|-------|-------------------|
| **Titles / display** (page titles, hero, big numbers) | **Geist** (`--font-display`, `.font-display`) | heavier, tighter — authoritative | 600–700 · `-0.018em` titles → `-0.055em` hero |
| **UI / body / labels** (everything else) | **Geist** (`--font-sans`) | lighter, quieter — disappears behind the work | 400–500 · 0 to `-0.01em` |
| **Numeric / tabular** (tables, metrics, badges) | Geist, `tabular-nums` | figures align in columns; counts don't jitter | — |
| **Mono** (code, evidence hashes, technical bits) | **Geist Mono** (`--font-mono`) | the mono of the same system | 400–500 |

**Geist is self-hosted** (`@fontsource/geist` + `@fontsource/geist-mono`,
`font-display: swap` — no render-blocking Google Fonts; TASTE.md §3.A). Critically,
**Geist covers Vietnamese** (the `vietnamese` subset ships in the static package), so
the primary market renders in the *brand* font, never a fallback. Rendering is tuned
globally: `font-optical-sizing: auto`, `text-rendering: optimizeLegibility`, and
`font-feature-settings: "ss01","cv01","tnum"` + `calt`/`kern`.

**Noto is the multilingual safety net, not the brand.** For scripts Geist does not
cover (Thai, Arabic, Devanagari, CJK, Korean), the stack *names* the matching Noto
family as a fallback — OS-provided, never self-hosted (self-hosting CJK would blow the
bundle budget, §performance). Noto solves missing glyphs; it never carries brand
personality.

Canonical stacks (mirrored on every platform — §19):

```
--font-sans / --font-display:
  Geist, "Noto Sans", "Noto Sans Thai", "Noto Sans Arabic", "Noto Sans Devanagari",
  "Noto Sans JP", "Noto Sans KR", "Noto Sans SC", "Noto Sans TC",
  ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
--font-mono:
  "Geist Mono", ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace
```

**Hard rules.** One brand family: **Geist** (+ Geist Mono for mono). Display vs body
is weight and tracking, *not* a second typeface. IBM Plex and Inter are removed
completely — do not reintroduce either, anywhere (web, iOS, Android, Mac, Watch,
slides, email, docs-site). No third face in core UI.

Avoid: Poppins · Montserrat · Futura-like startup fonts · overly rounded friendly
fonts · decorative serifs · thin weights below 400.

## 6.2 Type Scale

> The **canonical scale is fluid** — `clamp()`-based `--text-xs … --text-display`
> tokens that breathe between phone and desktop without breakpoints (defined in
> §19, paired with the `--tracking-*` tokens). The nominal px below are the
> desktop anchors of that scale.

```css
--font-sans: Geist, "Noto Sans", "Noto Sans Thai", "Noto Sans Arabic", "Noto Sans Devanagari", "Noto Sans JP", "Noto Sans KR", "Noto Sans SC", "Noto Sans TC", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;

--text-xs: 12px;
--text-sm: 14px;
--text-md: 16px;
--text-lg: 18px;
--text-xl: 22px;
--text-2xl: 28px;
--text-3xl: 36px;
--text-4xl: 48px;
--text-5xl: 64px;
```

## 6.3 Product UI Typography

### App page title

```css
font-family: var(--font-display);   /* Geist, display weight */
font-size: 28px;
line-height: 1.2;
font-weight: 650;
letter-spacing: -0.02em;
color: var(--color-civic-navy);     /* editorial ink — never pure black */
```

**Headings are Civic Navy ink, never black.** Page and section titles use
`--color-civic-navy` (`#102A43`) on every platform (web `h1`–`h3`, iOS
`LumiPageTitle`, Android `LumiPageTitle`/`LumiSectionHeader`): pure black looks cheap on
warm paper; navy looks editorial. Lumi Blue is for actions, links, and selected state;
body stays ink/navy and metadata soft-slate.

Use shared `<PageHeader>`, never a hand-rolled `<h1>`. It has an optional 11px, uppercase,
`0.14em`-tracked soft-slate editorial eyebrow (date, section, context) and one-line lede.
Today and CEO Brief use a locale-aware date eyebrow (`Intl.toLocaleDateString`, no i18n key)
to orient executives before the title; use it whenever a page has a natural where/when line.

### Section title

```css
font-size: 18px;
line-height: 1.35;
font-weight: 650;
```

### Body text

```css
font-size: 16px;
line-height: 1.6;
font-weight: 400;
```

### Metadata

```css
font-size: 13px;
line-height: 1.4;
font-weight: 450;
color: var(--color-soft-slate);
```

## 6.4 Marketing Typography

Marketing pages can be more editorial.

Hero headline:

```css
font-size: clamp(42px, 7vw, 88px);
line-height: 0.96;
font-weight: 700;
letter-spacing: -0.055em;
```

Hero body:

```css
font-size: clamp(18px, 2vw, 22px);
line-height: 1.55;
font-weight: 400;
```

Use large confident typography instead of hero illustrations.

## 6.5 Typography Taste Rules

Typography is product infrastructure: the cheapest way to feel world-class or generic.

Rules:

* **Geist display (600–700, tight tracking) is authoritative:** page/brief titles,
  hero moments, and large numbers should feel infrastructural, not like a consulting deck.
* **Geist body (400–500) operates:** labels, tables, forms, metadata, chat, and dense mobile cards
  disappear behind the work.
* **One family:** display and body are Geist, differentiated only by weight/tracking. Never add a
  second font; Noto is glyph fallback only.
* **Numbers align:** use `tabular-nums` for counts, money, dates, percentages, and jitter-sensitive tables.
* **Metadata stays readable:** owners, sources, dates, and confidence bear trust.
* **Test Vietnamese:** verify diacritics, long phrases, and natural line breaks on mobile and desktop.
* **No fake editorial flourish:** no random italics, mixed-family emphasis, poetic labels,
  oversized microcopy, or decorative uppercase.

Line-length rules:

| Content | Ideal width |
|---|---:|
| CEO brief prose | 56-72 characters |
| Form helper text | 40-64 characters |
| Table cell text | truncate after useful signal, reveal full text on detail |
| Mobile card body | 2-3 lines max before progressive disclosure |

If a line is too long, improve the composition or the copy. Do not shrink text
below the scale.

---

# 7. Layout System

## 7.1 Grid

Desktop:

```css
max-width: 1200px;
outer-margin: 80px;
grid-columns: 12;
gap: 24px;
```

Tablet:

```css
outer-margin: 40px;
gap: 20px;
```

Mobile:

```css
outer-margin: 20px;
gap: 16px;
```

### 7.1.1 Reading column — match width to the cognitive task

1200px is an outer bound; width follows the task. **Reading/deciding** surfaces (Today,
CEO Brief, single-record detail) use a focused `max-w-3xl` ~768px editorial column so
they read rather than scan. **Scanning/comparing** surfaces (CRM, lists/boards,
dashboards) use 1200px. Narrow for prose and decisions; wide for grids and tables.

## 7.2 Spacing Scale

```css
--space-1: 4px;
--space-2: 8px;
--space-3: 12px;
--space-4: 16px;
--space-5: 20px;
--space-6: 24px;
--space-8: 32px;
--space-10: 40px;
--space-12: 48px;
--space-16: 64px;
--space-20: 80px;
--space-24: 96px;
```

## 7.3 Whitespace Rule

Target composition:

```text
70% whitespace
30% content
```

This means edited, not empty. Avoid dense dashboards, excess widgets, competing cards,
small-label clutter, and multi-column executive pages.

## 7.4 One Message Per Section

Each page section answers one clear question: “What needs my decision?”, “What is overdue?”,
“Where is the team blocked?”, “Which customer is risky?”, or “What should I do today?” Split any
section that answers more than one.

## 7.5 Composition Grammar

Lumi uses three recurring compositions; choose by job.

### Briefing Column

Use for CEO Brief, Today, a single risk/decision/commitment, or any read-and-decide surface.

```text
Context
Primary summary
Ranked sections
Evidence / action
```

Rules:

* One focused column (`max-w-3xl`) unless comparison is required.
* The most important item appears above filters and secondary navigation.
* Each item shows claim, why it matters, owner, age/due date, and evidence.
* Actions sit near the claim they affect.

### Operating Table

Use for records that must be scanned, sorted, filtered, or compared.

```text
Header / filters
Table or compact list
Bulk or row actions
Detail sheet
```

Rules:

* Tables are for comparison, not decoration.
* Mobile becomes ranked cards with the same fields in priority order.
* Filters must reduce cognitive load. If a filter is rarely used, move it.
* Empty states should explain what creates records and what Lumi will do next.

### Capture Form

Use for Daily Pulse, settings, invitations, and any user-entered operating
signal.

```text
Short purpose
One question per block
Clear save / submit
Recovery state
```

Rules:

* Forms stay single-column.
* Labels are always visible.
* Helper text is short and human.
* The save state is explicit: saving, saved, failed, retry.

Do not mix these compositions casually. A CEO brief with dashboard widgets, a
table with editorial prose blocks, or a pulse form with a dashboard sidebar all
feel confused.

---

# 8. Shape and Geometry

## 8.0 Material Recipes

Every UI object must use one of these recipes. Do not invent one-off material.

| Recipe | Background | Border | Radius | Shadow | Use |
|---|---|---|---|---|---|
| Canvas | `--color-paper-white` + `--canvas-glow` | none | 0 | none | app background |
| Sheet | `--color-surface-white` | 1px `--color-border` | 8px | `--shadow-card` | cards, table shells, list groups |
| Raised sheet | `--color-surface-white` | 1px `--color-border-strong` | 8px | `--shadow-raised` | sticky bars, hovered interactive cards |
| Overlay | `--color-surface-white` | 1px `--color-border-strong` | 12px | `--shadow-overlay` | popovers, menus, tooltips |
| Modal sheet | `--color-surface-white` | 1px `--color-border-strong` | 16px | `--shadow-modal` | dialogs, command palette, bottom sheets |
| Recessed well | `--color-surface-white` | 1px `--color-border` | 8px | `--shadow-input` | inputs, textareas, search fields |
| Lit token | status soft token | 1px same-hue inset ring | 999px | `--shadow-lit` | badges, chips, tabs |
| Authority seal | `--color-lumi-blue` + `--gradient-primary` | 1px blue-active mix | 8px | `--shadow-button` | primary action |

Rules:

* A page may use many objects, but only these material recipes.
* A card inside a card is prohibited. Use sections, dividers, or nested rows.
* Overlays must visibly float above sheets. If the shadow tier does not make the
  plane obvious, the plane is wrong.
* Sheet surfaces must be white. Do not tint cards by domain unless the whole
  card is an alert or empty state.
* The background glow is part of the canvas only. Never attach glows to
  individual components.

## 8.1 Border Radius

Use modest radius. Radius communicates object class, not mood.

```css
--radius-xs: 2px;
--radius-sm: 4px;
--radius-md: 8px;
--radius-card: 8px;
--radius-control: 8px;
--radius-popover: 12px;
--radius-lg: 12px;
--radius-xl: 16px;
--radius-pill: 999px;
```

Radius roles:

| Radius | Use |
|---|---|
| 2px | tiny evidence markers, chart bar caps, progress segments |
| 4px | small thumbnails, compact icon containers, table row hover background |
| 8px | cards, buttons, inputs, list groups, mobile cards |
| 12px | popovers, dropdowns, date pickers, elevated menus |
| 16px | modals, command palette, bottom sheets, large empty states |
| 999px | badges, chips, avatar circles, segmented-control thumb only |

Avoid:

* giant pill buttons
* extreme roundness
* playful blobs
* organic shapes that conflict with the logo geometry

## 8.2 Borders

Use borders as material edges, not decoration.

```css
--border-hairline: 1px;
--border-focus: 2px;
--border-rail: 3px;
--color-border-subtle: #E7EAF0;
--color-border: #D9DEE8;
--color-border-strong: #C8D0DE;
```

Border roles:

| Border | Width | Color | Use |
|---|---:|---|---|
| Hairline | 1px | `--color-border` | cards, inputs, tables, dividers |
| Subtle divider | 1px | `--color-border-subtle` | section separators, table row dividers |
| Strong edge | 1px | `--color-border-strong` | overlays, active containers, sticky surfaces |
| Focus ring | 2px | blue mix | keyboard focus, never elevation |
| Severity rail | 3px | status hue | risk, overdue, danger zone, urgent decision |

Rules:

* Borders are always 1px except focus rings and severity rails.
* Do not use double borders unless the inner line is the lit top edge token.
* Do not put borders on all four sides of every row in a long list. Use one
  bottom divider or grouped sheets.
* Borders should feel archival, not boxy. If the UI looks like a spreadsheet,
  reduce borders and increase hierarchy.

## 8.3 Shadows — the elevation ramp

Shadows are **navy-tinted, never pure black** (`rgba(16, 42, 67, …)`) — a black
shadow on warm paper reads as dirt; a navy one reads as depth. Depth is a
4-tier ramp, not ad-hoc values. Each tier pairs an *ambient* shadow (close,
tight, holds the object to its surface) with a *key* shadow (cast, soft, sells
the height); both scale together so a surface reads as the same physical
material at every plane. Use the token, never a raw `box-shadow`.

| Token | Plane | Use |
|-------|-------|-----|
| `--elevation-1` | resting | cards, tables, list rows |
| `--elevation-2` | raised | sticky headers, hover-lifted cards, dock pods |
| `--elevation-3` | overlay | popovers, dropdowns, tooltips, the dock panel |
| `--elevation-4` | modal | dialogs, command palette, sheets (highest plane) |

Exact shadow recipes:

```css
--elevation-1:
  0 1px 2px rgba(16, 42, 67, 0.05);

--elevation-2:
  0 8px 24px -6px rgba(16, 42, 67, 0.10),
  0 2px 6px -2px rgba(16, 42, 67, 0.06);

--elevation-3:
  0 16px 40px -8px rgba(16, 42, 67, 0.14),
  0 4px 10px -4px rgba(16, 42, 67, 0.08);

--elevation-4:
  0 28px 60px -12px rgba(16, 42, 67, 0.20),
  0 8px 18px -6px rgba(16, 42, 67, 0.10);
```

Plane rules:

* Resting cards use `--shadow-card`, not a visible floating shadow.
* Hover lift is 1px vertical movement plus `--shadow-card-hover`, never a jump.
* Sticky headers use `--shadow-raised` only after they are stuck.
* Popovers and menus use `--shadow-overlay`.
* Dialogs and bottom sheets use `--shadow-modal`.
* Shadows do not communicate severity. Color rails and badges do.

### Lit-token coverage (one lighting language, every surface)

Lighting is not just elevation — each surface pairs a navy drop with an **inner
light-edge** so it reads as *lit from above*, never a flat sticker. This is the
single quality that makes the whole UI feel built by one hand. Every interactive
surface is on it (web; mobile mirrors it via platform-idiomatic shadows):

| Surface | Token | Lit treatment |
|---------|-------|---------------|
| Badges · chips · active nav pill · active tab | `shadow-lit` | soft tint + same-hue ring + lit edge (+ saturated dot/rail) |
| Cards · tables | `shadow-card` (`-hover`) | bright top edge + two-layer navy drop |
| Sticky bars (stuck header) · dock pods | `shadow-raised` | lit edge + the elevation-2 drop (8/24) — the raised plane: lifted off the page but a tier *below* the overlay panel that opens above a pod |
| Primary button · toggle (on) | `shadow-button` + `--gradient-primary` | top-down sheen + inner highlight + tight drop |
| Inputs · textarea · toggle track | `shadow-input` | recessed inner top shade (the inverse — a well) |
| Selects · popovers · dropdowns · toasts · chart tooltip | `shadow-overlay` | lifted lit sheet (elevation-3) + hairline ring |
| Dialogs · command palette · sheets | `shadow-modal` | the highest plane (elevation-4) — a deeper navy cast so a modal floats visibly clear of mere overlays |
| Skeletons | `.lumi-shimmer` | a soft light sweeps across while loading |
| Alerts · status banners | `shadow-lit` + accent rail | soft tint + same-hue ring + 3px saturated rail |
| Charts | gradient fills | trend areas fade to a soft wash; bars get lit, rounded tops |

The colour half is equally unified: badges, toasts, alerts, the chart tooltip, and
the mobile banner all draw their status colours from the **same lit-token quads**
(§10.4) — `info / success / warning / danger`. Nothing in the product invents its
own status palette.

In components, use the **shadcn/Tailwind utilities** — never a raw `box-shadow`,
an arbitrary `shadow-[…]`, or a stock `shadow-sm/md/lg/xl/2xl` (those are the
inconsistency this ramp replaces). The guard
`node frontend/scripts/check-design-tokens.mjs` rejects them.

```html
<div class="shadow-elevation-1">  <!-- resting card -->
<div class="shadow-elevation-3">  <!-- floating popover / dropdown -->
```

```css
/* raw CSS (rare) reads the token directly */
box-shadow: var(--elevation-1);
```

Focus rings are a separate affordance (accessibility, not depth): use
`shadow-ring` / `shadow-ring-destructive`, not an elevation tier (§16.5).

Still forbidden: glassmorphism blur stacks, generic floating-SaaS card shadows,
thick offset neo-brutalist shadows, and **coloured glows** (the navy tint is the
only colour a Lumi shadow ever carries).

---

# 9. Iconography

## 9.1 Icon Principle

Icons should feel like they belong to the logo.

Use:

* Simple line icons
* Geometric construction
* 1.75px or 2px stroke
* Square or angled details
* Occasional four-point star motif

Avoid:

* Cartoon icons
* 3D icons
* AI sparkle overload
* Gradient icons
* Emoji-like icons
* Random icon packs with inconsistent stroke

## 9.2 Core Icon Motifs

Supporting system motifs:

### Star mark

Meaning:

* verified
* clarified
* important insight
* Lumi found this

Use sparingly.

### L-corner

Meaning:

* structure
* framework
* workspace
* task/project container

### Annotation dot

Meaning:

* evidence
* source
* comment
* detail

### Line bracket

Meaning:

* grouping
* section
* report category

## 9.3 Icon Examples

* Daily Pulse: circle + small star
* CEO Brief: document + star
* Commitment: check line + deadline dot
* Risk: amber marker + bracket
* Decision: forked path + star
* Project: L-frame + stacked cards
* Recurring: loop arrow + small dot
* Evidence: document + annotation dot
* Chat: speech bubble + L-corner

---

# 10. UI Components

## 10.1 Buttons

### Primary Button — a lit, pressable object

Use for the single main action. The fill is lumi-blue, but **lit from above**:
a near-invisible top-down sheen (`--gradient-primary`: a lighter crown → brand →
pressed-tone floor) + `--shadow-button` (an inner top highlight + a tight navy
drop). The result is a tactile object you want to press, not a flat rectangle.

```css
background-color: var(--color-lumi-blue);
background-image: var(--gradient-primary);   /* the only allowed gradient */
box-shadow: var(--shadow-button);            /* inner highlight + navy drop */
color: white;
border: 1px solid color-mix(in oklch, var(--color-lumi-blue-active) 35%, transparent);
border-radius: var(--radius-control);
height: 40px;
min-width: 96px;
padding: 0 16px;
font-weight: 550;
font-size: 14px;
letter-spacing: 0;
```

- **Hover:** drop the sheen, deepen to `--color-lumi-blue-hover` (solid).
- **Active:** settle to `--color-lumi-blue-active`, remove the drop, nudge down 1px
  — the press feels physical.
- **Focus-visible:** 2px blue focus ring, 2px offset, never hidden by the shadow.
- **Disabled:** no gradient, no shadow, `opacity: 0.55`, cursor default.

> This single, near-imperceptible sheen is the **one** gradient in the system. It
> is structural lighting, not decoration — it must never be visible as a "gradient,"
> only as the button looking *lit*. Coloured/decorative gradients remain banned.

Button size contract:

| Size | Height | Padding | Font | Use |
|---|---:|---:|---:|---|
| `sm` | 32px | 12px | 13px / 550 | table rows, dense toolbars |
| `md` | 40px | 16px | 14px / 550 | default product actions |
| `lg` | 48px | 20px | 15px / 600 | hero, mobile primary actions |
| `icon-sm` | 32px square | 0 | icon 16px | dense toolbars |
| `icon-md` | 40px square | 0 | icon 18px | default icon actions |

Button hierarchy:

* One filled primary button per action cluster.
* Secondary buttons can sit beside the primary, but never compete in fill.
* Tertiary buttons are inline actions, not page-level CTAs.
* Destructive actions never share the same visual weight as the safe primary
  action unless the entire dialog is a destructive confirmation.

### Secondary Button

```css
background: transparent;
color: var(--color-civic-navy);
border: 1px solid var(--color-border);
border-radius: var(--radius-control);
height: 40px;
padding: 0 16px;
box-shadow: var(--shadow-lit);
```

### Tertiary Button

Text-only.

```css
color: var(--color-lumi-blue);
font-weight: 550;
height: 32px;
padding: 0 4px;
```

### Destructive Button

Use rarely.

```css
background: transparent;
color: var(--color-risk-red);
border: 1px solid var(--color-risk-red);
border-radius: var(--radius-control);
height: 40px;
padding: 0 16px;
```

---

## 10.2 Cards — lit documents

Cards are **lit documents on warm paper**, never glass or flat boxes: white surface,
1px border, and `shadow-card` lit drop (bright top edge plus close and soft navy-tinted
shadows). Never black paper shadows: they read as dirt, while navy reads as depth.

```css
background: var(--color-surface-white);
border: 1px solid var(--color-border);
border-radius: var(--radius-card);
padding: 20px;
box-shadow: var(--shadow-card);     /* lit token — see §19 */
```

Clickable rows and board cards lift 1px with `shadow-card-hover`; all others rest.

Card density contract:

| Card type | Padding | Radius | Border | Shadow | Use |
|---|---:|---:|---|---|---|
| Compact row card | 12-16px | 8px | 1px | `--shadow-card` | mobile list rows, task rows |
| Standard card | 20px | 8px | 1px | `--shadow-card` | brief item, risk, decision |
| Reading card | 24px | 8px | 1px | `--shadow-card` | CEO summary, evidence detail |
| Empty-state card | 24-32px | 16px | 1px | `--shadow-card` | all-clear, no data yet |
| Overlay card | 16-20px | 12px | 1px strong | `--shadow-overlay` | popover content |

Card anatomy:

```text
top row:    status / section label          owner or timestamp
title:      strong claim or record name
body:       why it matters, one short paragraph
metadata:   evidence, source, due date, confidence
action:     one contextual action, placed after the claim
```

Spacing inside a standard card:

```css
gap-title-body: 8px;
gap-body-meta: 12px;
gap-meta-action: 16px;
```

Clickable card state:

```css
transition: transform var(--motion-fast) var(--ease-out),
            box-shadow var(--motion-fast) var(--ease-out),
            border-color var(--motion-fast) var(--ease-out);
hover: transform translateY(-1px);
hover: box-shadow var(--shadow-card-hover);
active: transform translateY(0);
```

Use cards for brief, task, risk, decision, project, and pulse summaries. Avoid nested cards,
glass/blurred translucency, colored or heavy gradients (only the near-invisible primary-button
sheen in §10.1), untinted black shadows, and over-coloring.

---

## 10.3 Inputs

Inputs should be calm and highly readable.

```css
height: 40px;
border: 1px solid var(--color-border);
border-radius: var(--radius-control);
background: white;
padding: 0 12px;
font-size: 15px;
line-height: 1.4;
box-shadow: var(--shadow-input);
```

Focus:

```css
border-color: var(--color-lumi-blue);
box-shadow:
  var(--shadow-input),
  0 0 0 3px color-mix(in oklch, var(--color-lumi-blue) 14%, transparent);
```

Textarea for daily pulse:

```css
min-height: 140px;
line-height: 1.6;
padding: 12px;
```

Input state contract:

| State | Border | Shadow | Text |
|---|---|---|---|
| Rest | `--color-border` | `--shadow-input` | `--color-ink` |
| Hover | `--color-border-strong` | `--shadow-input` | unchanged |
| Focus | `--color-lumi-blue` + blue halo | input + focus ring | unchanged |
| Error | `--color-risk-red` + red halo | input + destructive ring | error text below |
| Disabled | `--color-border-subtle` | none | `--color-soft-slate` |

Placeholder text must meet contrast expectations for helper text. Never use
placeholder text as the label.

---

## 10.4 Badges — the "lit token" system

> **The anti-slop rule.** Never use a flat pastel text pill. Lumi badges are layered,
> scannable *lit tokens*, not one flat fill.

**Anatomy.** Every status badge is a **quad**, not one color:

| Layer | Role | Token |
|-------|------|-------|
| **surface** | the chip body — a soft, desaturated tint | `--badge-{status}-surface` |
| **ring** | a 1px *inset* hairline in the **same hue at low alpha** — identity, not a generic grey border | `--badge-{status}-ring` (`color-mix`, tracks the brand hue automatically) |
| **dot** | a 5px **saturated** disc the eye locks onto first — instant scannability | `--badge-{status}-dot` |
| **text** | AA-compliant strong ink of the hue | `--badge-{status}-text` |
| **lit edge** | a 1px inner top highlight so the chip reads *lit from above*, not flat | `shadow-lit` |

Five families (`neutral · info · success · warning · danger`) define the quad in
`index.css`; components reference token classes, never inline values.

**Geometry.** `rounded-full`, `h-5`, `px-2`, `gap-1.5`, dot `5px`; text `11px / 560 /
tracking-0.01em / tabular-nums`: small, dense, engineered, never shouty.

**States (every badge earns them).**
- **Resting:** surface + inset ring + lit edge + (status only) dot.
- **Hover** (interactive badges — filters, links, removable tags): `brightness 0.985`
  + a 1px lift (`-translate-y-px`). Calm, tactile, never a colour flash.
- **Active:** settle back down (`translate-y-0`, `brightness 0.95`).
- **Focus-visible:** 2px `--ring` halo at 2px offset — keyboard parity with buttons.

**Variant → family map** (the only allowed status variants):

| Variant | Family | Domain meaning |
|---------|--------|----------------|
| `status-open` | neutral | open / todo / draft / queued |
| `status-in-progress` | info (blue) | active / in review / medium |
| `status-done` | success (green) | done / approved / resolved / paid |
| `status-overdue` · `status-amber` | warning (amber) | overdue / high / waiting |
| `status-critical` | danger (red) | blocked / failed / rejected / critical |

Generic counts/categories/tags use dotless `outline` / `secondary` lit-neutral chips
(`bg-card`/`bg-secondary` + inset `ring-border` + `shadow-lit`) so tags never pose as status;
`default` is solid brand.

**Do / Don't.**
- ✅ One dot, one hue, per status. ✅ The ring is the status hue, never plain grey.
- ❌ No flat pastel fill with no ring or dot. ❌ No bright red unless truly critical.
- ❌ Never hand-roll a `statusColors` map on a page.

**Implementation.** `<StatusBadge status={x} />` (`@/components/page`) maps domain
status/severity/priority to a variant and automatically enables its status dot. Shadcn
`<Badge dot interactive>` carries styling; §19 `index.css` owns token quads.

## 10.5 Mobile Primitives

Mobile UI must express the same shadcn-style component contract as the web app.
The implementation language differs, but the primitives do not.

| Web primitive | Mobile KMP primitive | Native Android wrapper | Use for |
|---|---|---|---|
| `PageHeader` | `LumiPageTitle` | `LumiPageTitle` | Page title + optional description |
| `SectionHeader` | `LumiSectionHeader` | `LumiSectionHeader` | One-message section dividers |
| `Card` | `LumiCard` | `LumiCard` | Brief, task, risk, decision, pulse summaries |
| `Button` default | `LumiPrimaryButton` | `LumiButton` | One primary screen action |
| `Button` link/ghost | `LumiInlineAction` | `LumiTextButton` / `LumiInlineAction` | Inline row actions |
| `Input` / `Textarea` | `LumiMultilineField` / field primitive | `LumiFormField` | Forms and Pulse answers |
| `Badge` / `StatusBadge` | `LumiBadge` | `LumiBadge` | Status, counts, severity |
| `EmptyState` | `LumiEmptyState` | `LumiEmptyState` | Purposeful empty and all-clear states |
| Directory row | `LumiDirectoryRow` | `LumiRow` | Workspace rows and settings rows |

Mobile source of truth:

* Shared cross-platform screens and reusable business UI live in
  `android/shared/src/commonMain/kotlin/com/locuno/lumi/shared/ui/`.
* Android-only adapters live in `android/core/designsystem/`, but must mirror the
  same names, token values, shapes, and states.
* iOS must consume the shared KMP screens when available through
  `ios/Lumi/KMP/SharedScreenBridge.swift`; SwiftUI-only fallbacks are temporary
  adapters, not a second product design system.

Mobile token mapping:

```text
LumiBlue        #006093   primary buttons, active nav, key actions
CivicNavy      #102A43   headings and primary text
PaperWhite     #F4F0E8   app background
SurfaceWhite   #FFFFFF   cards, sheets, inputs
Border         #D9DEE8   borders and dividers
SoftSlate      #5E6677   metadata and helper text
SignalAmber    #F4A62A   warnings only
RiskRed        #C2410C   destructive/error only
SuccessGreen   #1F7A4D   resolved/all-clear states
```

Mobile deviations from web must be functional, not aesthetic: 48dp/44pt touch
targets, bottom navigation, native text scaling, store billing, and platform back
behavior. Color, typography hierarchy, card borders, badge tones, and empty/error
states must remain visibly Lumi.

Use badges for status and risk — always the **lit-token quad** of §10.4 (surface +
same-hue inset ring + saturated dot + AA text + lit edge), never a flat fill. The
canonical quads (web `index.css §19`, iOS `LumiBadge`, Android `LumiBadge`):

| Family | surface | dot / ring hue | text |
|--------|---------|----------------|------|
| neutral (open) | `#F2F4F7` | slate | `#344054` |
| info (in progress) | `lumi-blue-soft` | `lumi-blue` | `lumi-blue` |
| success (done) | `green-soft` | `success-green` | `success-green` |
| warning (overdue) | `amber-soft` | `signal-amber` | `#9A5B00` (AA) |
| danger (critical) | `red-soft` | `risk-red` | `risk-red` |

Do not use bright red unless critical. The dot is the saturated hue; the ring is
the same hue at low alpha (`color-mix`), never plain grey.

**Implementation:** never hand-roll a `statusColors` map on a page. Use the shared
`StatusBadge` primitive (`@/components/page`), which maps any domain status / severity
/ priority to the variant **and** enables the dot for status families — one source of
truth. `<StatusBadge status={x} />`, or pass `label` / `variant` to override.

### Mobile excellence rules

Mobile is a first-class executive surface, not a companion view.

Rules:

* **One-handed operation:** reach primary actions on common phones; use bottom bars/sheets when central.
* **Native touch targets:** ≥44pt iOS, ≥48dp Android unless the primitive is larger.
* **Cards over squeezed tables:** tables that cannot retain title, owner, status, due/age, and action become cards.
* **Evidence stays visible:** source/confidence need a compact row or disclosure, never an unlabeled tiny icon.
* **Design network states:** loading, retry, stale, offline, and denied must match the happy path.
* **Honor keyboard/safe areas:** submit stays uncovered and active fields visible.
* **Resist locale pressure:** Vietnamese fits navigation, cards, buttons, and empties without shrinking type.

Mobile polish test:

> Can a CEO read a brief in a car, approve a decision, and trust its source without
> pinching, hunting, or wondering whether the app is loading?

If not, redesign mobile before adding features.

---

## 10.5 Tables

Tables should be functional, not spreadsheet-like.

Use for:

* Tasks
* Commitments
* Risks
* Decisions
* Projects

Rules:

* Row height minimum 52px
* Strong title column
* Subtle metadata
* Sticky header if table is long
* Use badges for status
* Avoid excessive grid lines

Table border:

```css
border-bottom: 1px solid var(--color-border);
```

Header:

```css
font-size: 12px;
font-weight: 650;
text-transform: uppercase;
letter-spacing: 0.06em;
color: var(--color-soft-slate);
```

---

## 10.6 Page Primitives

Every surface uses shared editorial primitives (`frontend/src/components/page/`), never
hand-rolled headers or raw `<h1>`s, to keep hierarchy and rhythm identical.

### PageHeader

Canonical title block: optional uppercase **eyebrow**, 28px/650/civic-navy **title**
(§6.3), optional one-line **lede**, and right-aligned **actions**.

```text
TODAY'S OPERATING BRIEF · 12 Jun        ← eyebrow (optional)
Good morning                            ← title                 [ Generate ]  ← actions
Lumi has reviewed yesterday's updates…  ← lede (optional, ~1 sentence)
```

**Toolbar exception.** Board/Timeline/Workflow/Tasks pair title, back button, and view
switcher, so use exported **`PAGE_TITLE_CLASS`** rather than full `<PageHeader>`. It is the
same 28px/650/civic-navy single source of truth; never hand-tune an inline `<h1>`.

### SectionHeader

Long-page divider: uppercase **label**, optional **count** chip, and edge-running hairline;
it enforces one message per section (§7.4).

```text
NEEDS YOUR DECISION  ③ ─────────────────────────────────────
```

### EmptyState

Authored empty state, never blank: icon, title, one-line population explanation, optional action.
`tone="success"` is the reassuring green-soft/success-green Risks/Brief all-clear variant.

> The CEO Brief page is the reference implementation of all three.

## 10.7 Severity rails — colour as triage

List and digest items use a **3px left status-hue rail** for pre-reading triage. Only
attention levels get one; calm items stay rail-less, applying the §8.3 vocabulary to rows.

| Level | Rail | Used for |
|-------|------|----------|
| Danger | `border-l-4 border-l-[var(--color-risk-red)]` | high/critical risks, overdue commitments, the Settings danger zone |
| Attention | `border-l-4 border-l-[var(--color-signal-amber)]` | high-priority tasks, urgent decisions |
| Calm | *(no rail)* | everything routine |

Pair rails with **ranking** (§4.3): railed items lead so a digest reads in priority order.
Today is the ranked-and-railed reference; Risks, Decisions, and Settings danger use the same
lit-token quad hues (§10.4), never per-surface severity colors.

---

# 11. Product Surfaces

Every Lumi surface must know which operating job it serves:

| Surface job | Design answer |
|---|---|
| See clearly | summary first, ranked items, evidence nearby |
| Decide fast | one primary action, options visible, owner and deadline clear |
| Trust the signal | source, timestamp, confidence, and provenance close to the claim |
| Capture signal | short form, visible labels, calm save state |
| Chase and verify | status, owner, next action, and follow-up age visible |

Design from the operating question inward, then reveal the data needed to answer it.

## 11.1 CEO Brief Page

This most important surface is a morning briefing document, calm executive memo, and
prioritized decision surface — never a dashboard.

Page structure:

```text
Header:
  Good morning / Brief date / Generate brief button

Executive Summary:
  2–4 sentence summary

Sections:
  1. Needs CEO Decision
  2. Customer / Revenue Risk
  3. Cash / Receivables
  4. Overdue Commitments
  5. Team Blockers
  6. Follow-up Opportunities
  7. Suggested Questions
```

Each brief item shows title, why it matters, suggested CEO action, owner, due date,
source/evidence, and confidence when low.

### Brief item layout

```text
┌──────────────────────────────────────────────┐
│ Decision needed                              │
│ Approve 5% discount for ABC before 11:00     │
│                                              │
│ Why it matters: 120M VND deal may stall.     │
│ Suggested action: approve or hold price.     │
│                                              │
│ Owner: Lan · Source: Sales pulse · 08/06     │
└──────────────────────────────────────────────┘
```

Use the four-point star only for Lumi-generated insight, not every item.

---

## 11.2 Daily Pulse Page

This page must be extremely simple: one question per block, completed in 60–90 seconds.
Avoid many required fields, dense forms, project-management jargon, long instructions, and excess dropdowns.

Default questions:

```text
1. Hôm nay bạn hoàn thành việc gì quan trọng?
2. Bạn đang kẹt ở đâu?
3. Bạn đã hứa gì với khách / team / nhà cung cấp?
4. Việc nào cần CEO hoặc quản lý quyết?
5. Có rủi ro nào về khách, doanh thu, công nợ, đơn hàng hoặc deadline không?
6. Ngày mai việc quan trọng nhất của bạn là gì?
```

Visual style: large textarea, minimal labels, warm paper, clear submit, and “Mất khoảng 60 giây.”

---

## 11.3 Tasks Page

Tasks must work without making Lumi a task-manager clone. Show title, owner, due date,
priority, status, project, and business impact when present; add customer, money, source type,
and linked commitment/risk.

---

## 11.4 Projects Page

Project UI is clean and operational: project, owner, team, status, due date, open/overdue tasks,
and linked risks. No complex Gantt or chart features in MVP.

---

## 11.5 Recurring Tasks Page

Recurring tasks are operating routines. Use routine/cadence, Every Monday, Daily, Monthly, and
Next run; never expose technical RRULE language.

Example:

```text
Check receivables
Every Monday · Finance · Next run: 10/06
```

---

## 11.6 Commitments Page

Commitments are stronger than tasks. Show promise, owner, stakeholder/customer, due date/time,
business impact, risk, evidence, and status. Use “Who promised what?”, “What is overdue?”, and
“Which promises affect customers or money?”

---

## 11.7 Risks Page

Risks are sober, not scary. Show title, severity, likelihood, owner, related customer/project/task,
money when available, evidence, and suggested action. Use amber unless critical, never red-heavy UI.

---

## 11.8 Decisions Page

Decision requests are an executive queue: decision, requester, needed-by date, options,
recommendation when available, business impact, evidence, and status. Answer: “What is waiting on leadership?”

---

## 11.9 Chat with Lumi

Chat is an operating query layer, not a ChatGPT clone.

Examples:

```text
Hôm nay việc nào cần tôi quyết?
Ai đang bị kẹt nhiều nhất?
Khách nào có rủi ro?
Task nào quá hạn?
Tóm tắt project ABC.
```

Chat answers are short, source-linked, action-oriented, clear about missing data, and
never speculative without saying so.

## 11.10 AI Insight States

AI-generated items look accountable, never magical.

Every AI insight, risk, commitment, and decision request has a visible state:

| State | Visual treatment | Copy behavior |
|---|---|---|
| Source-backed | normal card, evidence chip, timestamp | confident, concrete |
| Low confidence | neutral or amber soft badge, confirm action | cautious, asks for review |
| Missing evidence | empty/evidence-needed state, no strong claim | explains what data is missing |
| Conflicting evidence | comparison layout or warning rail | names the conflict plainly |
| Stale evidence | age visible, refresh or request update | avoids current-tense claims |
| Confirmed by human | success badge or resolved state | records owner and time |

Never make AI look more certain than its evidence; an honest, useful low-confidence state
is more premium than a polished hallucination.

---

# 12. Data Visualization

Use charts sparingly; Lumi is not a dashboard.

Allowed charts:

* Small weekly pulse completion trend
* Overdue tasks by team
* Risks by type
* Decisions waiting by age
* Commitments resolved vs overdue

Style:

* Minimal axes
* No rainbow palettes
* No 3D charts
* No decorative gradients
* Use Lumi Blue, Civic Navy, Archive Gray, Signal Amber

Default chart colors:

```css
--chart-primary: #006093;
--chart-secondary: #102A43;
--chart-muted: #CBD3DF;
--chart-warning: #F4A62A;
--chart-risk: #C2410C;
--chart-success: #1F7A4D;
```

---

# 13. Illustration and Imagery

## 13.1 Illustration Style

Use:

* Documents
* Briefing sheets
* Operating ledgers
* Checklists
* Evidence markers
* Thin annotation circles
* Structured work surfaces
* Simple geometric diagrams
* Paper textures
* Editorial layouts

Avoid:

* Robots
* Neural networks
* Holograms
* AI glow
* 3D mascots
* Floating dashboards
* Generic people illustrations
* Hyper-polished startup illustration packs

## 13.2 Photography

If using photography:

Use:

* Real work environments
* Natural light
* Editorial composition
* Real notebooks, papers, laptops
* Human hands reviewing documents
* CEO/founder desk scenes

Avoid:

* Stock call-center teams
* Overly smiling office people
* Fake diversity collage
* Futuristic blue-light rooms
* Corporate handshake photos

---

# 14. Motion Design

Motion should be calm and functional.

Use motion for:

* Loading brief generation
* Showing AI extraction progress
* Expanding evidence
* Revealing brief sections
* Completing tasks

Motion duration:

```css
--motion-fast: 120ms;
--motion-normal: 180ms;
--motion-slow: 260ms;
```

Easing:

```css
cubic-bezier(0.2, 0.8, 0.2, 1)
```

Avoid:

* Bouncy motion
* Sparkle animation
* AI typing theatrics
* Excessive loading animations
* Looping decorative motion

## 14.1 Layout stability

The calmest motion is the motion that never happens. The app scrolls at the
document level, so the page-level scrollbar appears on long routes and vanishes on
short ones — and a scrollbar that takes layout space drags the whole column sideways
by its width as you navigate. Reserve the gutter once so it can't:

```css
html {
  scrollbar-gutter: stable;
}
```

`stable` reserves the inline-end edge only, so it tracks LTR→RTL automatically, and
is inert under overlay scrollbars (macOS) — a fix where the scrollbar takes space, a
no-op where it doesn't. No content jump between routes; CLS stays at zero from this
source.

When Lumi generates a brief, use language:

```text
Lumi is reviewing yesterday’s updates...
Finding commitments...
Checking overdue items...
Preparing the CEO brief...
```

No magical animation needed.

---

# 15. Voice and Copy

## 15.1 Brand Voice

Lumi speaks like:

* A precise operating assistant
* A calm chief of staff
* A trustworthy analyst
* A clear editor

Lumi does not speak like:

* A motivational coach
* A hype AI tool
* A playful chatbot
* A corporate consultant using jargon

## 15.2 Copy Rules

Use:

* Short sentences
* Specific nouns
* Action verbs
* Clear ownership
* Evidence-based claims

Avoid:

* “Unlock productivity”
* “Supercharge your team”
* “10x your workflow”
* “AI-powered magic”
* “Revolutionize operations”
* “Seamlessly leverage synergy”

## 15.3 Preferred Language

Good:

```text
3 decisions are waiting for CEO review.
5 commitments are overdue.
ABC has not responded to a 120M VND quote for 6 days.
Finance needs to confirm the payment date today.
```

Bad:

```text
Your team has several exciting opportunities to improve operational excellence.
```

## 15.4 Vietnamese Copy Tone

Use:

```text
rõ
ngắn
thẳng
không quá Tây
không quá corporate
không dùng jargon AI
```

Example:

```text
Hôm nay có 3 việc cần CEO quyết.
```

Not:

```text
Lumi đã tối ưu hóa năng suất bằng trí tuệ nhân tạo để nâng cao hiệu quả vận hành.
```

---

# 16. Accessibility

Lumi must feel premium because it is usable.

## 16.1 Contrast

* Body text must meet WCAG AA
* Primary blue on white must pass
* Avoid pale gray text
* Do not use color alone to communicate risk

**Verified + guarded.** Every shipped text pair clears AA (≥4.5:1), measured across all
three surface tones (paper / card / muted+accent), the filled controls, and the badge
quads: body text (soft-slate/paper 5.07, civic-navy/paper 12.88, lumi-blue/paper 7.70,
muted-foreground on the muted surface ~4.8 — the tightest), the **primary button**
(white on lumi-blue) and **danger button** (white on risk-red), the accent/count-chip
(lumi-blue on lumi-blue-soft), and all five badge-quad text colours. This is enforced by
`src/test/contrast.test.ts` (17 pairs), which fails CI the moment a colour decision drops
a real text pair below AA, so the warm-paper depth (and any future tuning) can never
silently regress contrast. (Hairline `border-base` is intentionally subtle — cards/
inputs are identified by surface tone + lit shadow + the focus ring, §16.5, not the
border alone, so 1.4.11 non-text contrast is met by those affordances.)

## 16.2 Font Size

Minimum body text:

```css
16px
```

Minimum metadata:

```css
12px
```

Daily pulse textareas should use:

```css
16px or 17px
```

## 16.3 Click Targets

Minimum tap/click target:

```css
40px height
```

Prefer:

```css
44px
```

## 16.4 Keyboard Navigation

All app controls must be reachable by keyboard:

* buttons
* links
* form fields
* modals
* dropdowns
* chat input

## 16.5 Focus States

Focus states must be visible — and the brand blue is sourced from the token layer
(`--lumi-blue`), never a hardcoded literal, so the focus ring and the text-selection
highlight track one colour:

```css
:focus-visible {
  outline: 2px solid color-mix(in oklch, var(--color-lumi-blue) 45%, transparent);
  outline-offset: 2px;
}
```

**Touch feedback.** On touch devices the browser's default tap highlight is an opaque
grey box that reads as unfinished. Replace it with a faint branded tint rather than
removing it outright — feedback stays, but in the product's blue:

```css
body {
  -webkit-tap-highlight-color: color-mix(in oklch, var(--color-lumi-blue) 12%, transparent);
}
```

`:active` / `:focus-visible` still carry the considered states on top.

---

# 17. Landing Page Direction

## 17.1 Landing Page Strategy

Avoid a typical AI-startup landing page.

Avoid:

* Big 3D orb
* Gradient AI brain
* Floating cards everywhere
* Cartoon team workflow
* Fake dashboard overload

Use:

* Strong typography
* Warm paper background
* Product screenshots
* Short CEO-focused copy
* Evidence-based sections
* Calm structure

## 17.2 Hero

Hero copy:

```text
Run the company with a clearer morning brief.

Team updates, tasks, projects, and recurring work —
turned into a daily operating brief for the CEO.
```

Vietnamese version:

```text
Mỗi sáng, CEO nắm rõ công ty đang kẹt ở đâu.

Team cập nhật việc hằng ngày.
Lumi tổng hợp thành brief điều hành:
việc trễ, khách rủi ro, tiền kẹt,
và quyết định đang chờ.
```

CTA:

```text
Start a 14-day pilot
See example brief
```

## 17.3 Landing Page Structure

```text
1. Hero
2. Example Morning Brief
3. How Lumi Works
4. What Lumi Tracks
5. Who It Is For
6. Pilot Offer
7. FAQ
8. Final CTA
```

## 17.4 Example Section

Show a real-looking brief instead of abstract illustration.

```text
Today’s Lumi Brief

3 decisions need CEO review.
5 commitments are overdue.
2 customer risks need follow-up.
1 payment date needs confirmation.
```

This is more convincing than any AI illustration.

---

# 18. App Layout

## 18.1 Default App Layout

```text
┌──────────────────────────────────────────┐
│ Sidebar  │ Top bar                       │
│          ├───────────────────────────────┤
│          │ Page content                  │
│          │                               │
└──────────────────────────────────────────┘
```

Sidebar should be calm and structured.

Navigation groups:

```text
Work
- Today
- Projects
- Tasks
- Recurring

Lumi
- Daily Pulse
- CEO Brief
- Commitments
- Risks
- Decisions
- Chat

Admin
- Teams
- Settings
```

## 18.2 CEO Home

Default CEO route:

```text
/brief
```

CEO should not land on task lists first.

## 18.3 Member Home

Default member route:

```text
/today
```

Member should see:

* Today’s tasks
* Daily pulse reminder
* Assigned commitments
* Overdue items

---

# 19. Design Tokens

Use this base token file.

```css
:root {
  /* Brand — anchored to the OKLCH blue ramp (§5.3) */
  --color-lumi-blue: #006093;          /* blue-600 */
  --color-lumi-blue-hover: #004F80;    /* blue-700 */
  --color-lumi-blue-active: #003E6A;   /* blue-800 */
  --color-lumi-blue-soft: #E4F3FC;

  /* Text */
  --color-civic-navy: #102A43;
  --color-ink: #172033;
  --color-soft-slate: #5E6677;         /* AA ≥4.5:1 on paper */

  /* Surfaces */
  --color-paper-white: #F4F0E8;
  --color-surface-white: #FFFFFF;
  --color-archive-gray: #E7EAF0;
  --color-border-subtle: #E7EAF0;
  --color-border: #D9DEE8;
  --color-border-strong: #C8D0DE;

  /* States */
  --color-signal-amber: #F4A62A;
  --color-amber-soft: #FFF4DC;
  --color-risk-red: #C2410C;
  --color-red-soft: #FFF1EC;
  --color-success-green: #1F7A4D;
  --color-green-soft: #E9F7EF;

  /* Dark-ground status tints (§5.5). Web/print sales surfaces only — these are
     deliberately OUTSIDE the §19.1 platform-parity contract: no native shell has
     a navy panel, so mirroring them would add dead tokens to every platform. */
  --color-amber-on-navy: #F4A62A;
  --color-red-on-navy: #FF9E7A;
  --color-green-on-navy: #6FD39F;

  /* Typography — Geist (one brand family, display = weight/tracking; §6.1).
     Noto families are a named glyph fallback (OS-provided), never self-hosted. */
  --font-sans: Geist, "Noto Sans", "Noto Sans Thai", "Noto Sans Arabic", "Noto Sans Devanagari", "Noto Sans JP", "Noto Sans KR", "Noto Sans SC", "Noto Sans TC", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --font-display: var(--font-sans);
  --font-mono: "Geist Mono", ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;

  /* Fluid type scale — clamp(min, preferred, max); ~1.2 minor third (§6.2) */
  --text-xs: clamp(0.75rem, 0.73rem + 0.10vw, 0.8125rem);
  --text-sm: clamp(0.8125rem, 0.79rem + 0.12vw, 0.875rem);
  --text-base: clamp(0.9375rem, 0.91rem + 0.16vw, 1rem);
  --text-lg: clamp(1.0625rem, 1.01rem + 0.28vw, 1.1875rem);
  --text-xl: clamp(1.25rem, 1.16rem + 0.45vw, 1.5rem);
  --text-2xl: clamp(1.5rem, 1.34rem + 0.80vw, 2rem);
  --text-3xl: clamp(1.875rem, 1.60rem + 1.35vw, 2.75rem);
  --text-display: clamp(2.5rem, 1.90rem + 3.00vw, 4.5rem);

  /* Letter-spacing — tighten as type grows */
  --tracking-tight: -0.02em;
  --tracking-snug: -0.01em;
  --tracking-normal: 0em;
  --tracking-wide: 0.08em;

  /* Radius */
  --radius-xs: 2px;
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-card: 8px;
  --radius-control: 8px;
  --radius-popover: 12px;
  --radius-lg: 12px;
  --radius-xl: 16px;
  --radius-pill: 999px;

  /* Borders */
  --border-hairline: 1px;
  --border-focus: 2px;
  --border-rail: 3px;

  /* Spacing */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;
  --space-16: 64px;
  --space-20: 80px;

  /* Elevation — navy-tinted 4-tier ramp (§8.3) */
  --elevation-1: 0 1px 2px rgba(16, 42, 67, 0.05);
  --elevation-2: 0 8px 24px -6px rgba(16, 42, 67, 0.10), 0 2px 6px -2px rgba(16, 42, 67, 0.06);
  --elevation-3: 0 16px 40px -8px rgba(16, 42, 67, 0.14), 0 4px 10px -4px rgba(16, 42, 67, 0.08);
  --elevation-4: 0 28px 60px -12px rgba(16, 42, 67, 0.20), 0 8px 18px -6px rgba(16, 42, 67, 0.10);

  /* Lit tokens — lighting, not just elevation (§8.3 / §10.1 / §10.2). Each pairs
     an inner top highlight (lit from above) with a navy drop:
       shadow-lit     — badges, chips, active nav pills, active tabs
       shadow-card    — cards (+ shadow-card-hover for interactive cards)
       shadow-raised  — sticky bars and hovered raised sheets
       shadow-button  — the primary CTA
       shadow-input   — the recessed inverse, for form fields
       shadow-overlay — floating panels (selects, popovers, dropdowns, the dock)
       shadow-modal   — dialogs, command palette, sheets */
  --shadow-lit: inset 0 1px 0 0 color-mix(in oklch, white 70%, transparent), inset 0 -1px 1px 0 color-mix(in oklch, var(--color-civic-navy) 5%, transparent);
  --shadow-card: inset 0 1px 0 0 color-mix(in oklch, white 60%, transparent), 0 1px 2px -1px color-mix(in oklch, var(--color-civic-navy) 8%, transparent), 0 8px 20px -10px color-mix(in oklch, var(--color-civic-navy) 10%, transparent);
  --shadow-card-hover: inset 0 1px 0 0 color-mix(in oklch, white 68%, transparent), 0 2px 6px -2px color-mix(in oklch, var(--color-civic-navy) 10%, transparent), 0 12px 28px -14px color-mix(in oklch, var(--color-civic-navy) 14%, transparent);
  --shadow-raised: inset 0 1px 0 0 color-mix(in oklch, white 72%, transparent), 0 8px 24px -6px color-mix(in oklch, var(--color-civic-navy) 10%, transparent), 0 2px 6px -2px color-mix(in oklch, var(--color-civic-navy) 6%, transparent);
  --shadow-button: inset 0 1px 0 0 color-mix(in oklch, white 22%, transparent), 0 1px 2px 0 color-mix(in oklch, var(--color-civic-navy) 22%, transparent);
  --shadow-input: inset 0 1px 1px 0 color-mix(in oklch, var(--color-civic-navy) 4%, transparent);
  --shadow-overlay: inset 0 1px 0 0 color-mix(in oklch, white 75%, transparent), 0 16px 40px -8px color-mix(in oklch, var(--color-civic-navy) 14%, transparent), 0 4px 10px -4px color-mix(in oklch, var(--color-civic-navy) 8%, transparent);
  --shadow-modal: inset 0 1px 0 0 color-mix(in oklch, white 80%, transparent), 0 28px 60px -12px color-mix(in oklch, var(--color-civic-navy) 20%, transparent), 0 8px 18px -6px color-mix(in oklch, var(--color-civic-navy) 10%, transparent);

  /* The one allowed gradient — the primary-button sheen (§10.1) */
  --gradient-primary: linear-gradient(180deg, color-mix(in oklch, white 10%, var(--color-lumi-blue)) 0%, var(--color-lumi-blue) 52%, var(--color-lumi-blue-hover) 100%);

  /* Canvas atmosphere — a fixed, very soft top-down light under the paper grain so
     the background reads as a lit surface in a calm room, never a dead sheet (§1.2). */
  --canvas-glow: radial-gradient(115% 50% at 50% -6%, color-mix(in oklch, white 60%, transparent) 0%, transparent 58%);

  /* Motion */
  --motion-fast: 120ms;
  --motion-normal: 180ms;
  --motion-slow: 260ms;
  --ease-out: cubic-bezier(0.2, 0.8, 0.2, 1);
}
```

> Web also defines the full OKLCH ramps (`--blue-50…950`, etc.) and a
> `@media (color-gamut: p3)` enrichment block — see §5.3 and
> `frontend/src/index.css`. Those are the *generator*; the hexes above are the
> *contract* every platform mirrors.

## 19.1 Cross-Platform Token Parity (contract)

The values above are canonical. Each native platform keeps a hand-mirrored copy;
they must match this table exactly — a drift here is a defect (it is how Mac/Watch
once shipped `#0B3397`/`#667085` after web had moved on). When a token changes,
it changes in **every** column in the same PR.

| Token | Web (`index.css`) | iOS / Mac / Watch (`Color`) | Android (`Color.kt`) |
|-------|-------------------|-----------------------------|----------------------|
| lumi-blue | `#006093` | `lumiBlue` | `LumiBlue` |
| lumi-blue-hover | `#004F80` | `lumiBlueHover` | `LumiBlueHover` |
| lumi-blue-active | `#003E6A` | `lumiBlueActive` | `LumiBlueActive` |
| lumi-blue-soft | `#E4F3FC` | `lumiBlueSoft` | `LumiBlueSoft` |
| civic-navy | `#102A43` | `civicNavy` | `CivicNavy` |
| ink | `#172033` | `ink` | `Ink` |
| soft-slate | `#5E6677` | `softSlate` | `SoftSlate` |
| paper-white | `#F4F0E8` | `paperWhite` | `PaperWhite` |
| surface-white | `#FFFFFF` | `surfaceWhite` | `SurfaceWhite` |
| archive-gray | `#E7EAF0` | `archiveGray` | `ArchiveGray` |
| border-subtle | `#E7EAF0` | `borderSubtle` | `BorderSubtle` |
| border | `#D9DEE8` | `border` | `Border` |
| border-strong | `#C8D0DE` | `borderStrong` | `BorderStrong` |
| signal-amber | `#F4A62A` | `signalAmber` | `SignalAmber` |
| amber-soft | `#FFF4DC` | `warningBanner` | `AmberSoft` |
| risk-red | `#C2410C` | `riskRed` | `RiskRed` |
| red-soft | `#FFF1EC` | — | `RedSoft` |
| success-green | `#1F7A4D` | `successGreen` | `SuccessGreen` |
| green-soft | `#E9F7EF` | — | `GreenSoft` |

**Scope of this contract.** It binds the product palette above. The §5.5
dark-ground tints (`amber/red/green-on-navy`) are intentionally absent: they exist
only for web and print sales surfaces that invert one panel to navy, a surface no
native shell has. Adding them to the native mirrors would ship dead tokens. If a
native surface ever needs a navy ground, add the row here in that PR.

Material shape tokens must also mirror by role, even when native platforms express
them as `CGFloat`, `Dp`, or component defaults:

| Role | Web token | iOS / Mac / Watch | Android |
|---|---|---|---|
| tiny radius | `--radius-xs: 2px` | `radiusXS = 2` | `RadiusXS = 2.dp` |
| small radius | `--radius-sm: 4px` | `radiusSM = 4` | `RadiusSM = 4.dp` |
| card/control radius | `--radius-card/control: 8px` | `radiusCard = 8` | `RadiusCard = 8.dp` |
| popover radius | `--radius-popover: 12px` | `radiusPopover = 12` | `RadiusPopover = 12.dp` |
| modal radius | `--radius-xl: 16px` | `radiusXL = 16` | `RadiusXL = 16.dp` |
| hairline border | `--border-hairline: 1px` | `1 / displayScale` | `1.dp` or platform hairline |
| focus ring | `--border-focus: 2px` | `2` | `2.dp` |
| severity rail | `--border-rail: 3px` | `3` | `3.dp` |

**No dark column** — by design (§5). Native shells force light
(`UIUserInterfaceStyle = Light` / `.preferredColorScheme(.light)` /
light-only `LumiTheme`); web sets `color-scheme: light` and keeps the `dark:`
variant inert (pinned to a never-applied `.dark` class).

---

# 20. Tailwind Theme Guidance

> **Banned: stock Tailwind palette classes.** In `frontend/src`, never use
> `gray/indigo/slate/blue-NNN` (or raw `green/amber/red-NNN` for status). They make
> the product look like a generic SaaS template instead of wearing the Lumi system.
> Map every color to a semantic token — `bg-card`, `text-foreground`,
> `text-muted-foreground`, `border-border`, `text-primary`, and the `*-soft` status
> tokens (§5.2 / §19). This is merge-blocking (CLAUDE.md Rule 4); the guard
> `node frontend/scripts/check-design-tokens.mjs` enforces it. Full mapping table:
> frontend/AGENTS.md → "Color Tokens — Never Hardcode Colors."

Example Tailwind extension:

```js
export default {
  theme: {
    extend: {
      colors: {
        lumi: {
          blue: "#006093",
          hover: "#004F80",
          soft: "#E4F3FC"
        },
        civic: {
          navy: "#102A43"
        },
        paper: {
          white: "#F4F0E8"
        },
        archive: {
          gray: "#E7EAF0"
        },
        signal: {
          amber: "#F4A62A"
        },
        risk: {
          red: "#C2410C"
        },
        success: {
          green: "#1F7A4D"
        }
      },
      borderRadius: {
        sm: "4px",
        md: "8px",
        lg: "12px",
        xl: "16px"
      },
      fontFamily: {
        sans: ["Geist", "Noto Sans", "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ["Geist Mono", "ui-monospace", "SFMono-Regular", "monospace"]
      }
    }
  }
};
```

---

# 21. Anti-Patterns

Do not use AI glow, neon or purple-blue startup gradients, glassmorphism, floating 3D dashboards,
robot illustrations, brain icons, circuit-board patterns, excessive sparkles, cartoon mascots, huge
pill buttons, generic SaaS avatars, fake stock-office photography, template landing-page waves,
dashboard overload, red-heavy fear UI, or surveillance language.

Do not say:

```text
AI-powered productivity revolution
10x your team
Supercharge your workflow
Command your company with AI
Automate your CEO
```

Say:

```text
Know what needs attention.
See what is stuck.
Turn daily updates into a morning brief.
Keep work clear without more meetings.
```

---

# 22. Unique Unconventional Edge

The unconventional move is restraint. Most AI products signal intelligence with glow, gradients,
magic, animation, chatbot bubbles, or fake dashboards; Lumi signals it with edited information,
source-backed insights, strong typography, calm surfaces, evidence trails, clear next actions, and
human-readable operating memory.

The product should feel like:

> A serious organization has reviewed the company’s operating signals and prepared a brief.

Not:

> A chatbot generated a summary.

That distinction is the brand.

---

# 23. Example Product Copy

## Homepage hero

```text
A clearer morning brief for running the company.

Lumi turns daily team updates, tasks, projects, and recurring work
into a focused operating brief for the CEO.
```

## Vietnamese hero

Use the canonical Vietnamese hero in [§17.2](#172-hero).

## Product description

```text
Lumi is an AI operating workspace for SME teams.
Team members update daily pulse, tasks, projects, and recurring work.
Lumi turns that information into commitments, risks, decisions,
and a morning brief for leadership.
```

## Empty state

```text
No brief yet.

Ask your team to submit today’s pulse.
Lumi will use those updates to prepare your first operating brief.
```

## Error state

```text
Lumi does not have enough information yet.

Try adding a daily pulse, task, or commitment.
```

## Low confidence state

```text
Lumi found this item, but the source is incomplete.
Please confirm before acting.
```

---

# 24. Design QA Checklist

Before shipping any page, check:

```text
[ ] Is the page calm?
[ ] Is the main action obvious?
[ ] Is there one clear message per section?
[ ] Is the text readable at 16px+?
[ ] Is the layout using enough whitespace?
[ ] Are colors restrained?
[ ] Are warnings not over-red?
[ ] Are Lumi-generated insights source-backed?
[ ] Is the logo used respectfully?
[ ] Does this feel like a serious operating assistant, not a toy?
[ ] Would a CEO understand the page in 5 seconds?
[ ] Would a designer see restraint and consistency?
[ ] Does the first viewport answer the user's role-specific question?
[ ] Are important items ranked before they are decorated?
[ ] Is every AI claim visibly source-backed or clearly marked low confidence?
[ ] Are amber and red used only for real attention or danger?
[ ] Does the screen work in Vietnamese without awkward wrapping?
[ ] Does mobile have a purpose-built layout, not a squeezed desktop layout?
[ ] Are loading, empty, error, stale, permission, and low-confidence states designed?
[ ] Could this screen be used in a real operating meeting without losing trust?
[ ] Does every visible object use one of the §8.0 material recipes?
[ ] Are card/control/popover/modal radii role-correct?
[ ] Are borders limited to 1px hairlines, 2px focus rings, and 3px severity rails?
[ ] Are shadows using `--shadow-*` tokens, never raw or black shadows?
[ ] Is the only product gradient the primary-button lighting sheen?
[ ] Does any lifted surface have a real reason: interaction, sticky state, or overlay?
```

---

# 25. Final Design Statement

Lumi’s design should be:

> Calm enough for daily use, serious enough for executive decisions, warm enough for teams, and precise enough to trust.

The logo supplies institutional blue, geometric structure, and disciplined negative space. Do not
decorate it; build the whole product around what it says:

> Structure. Clarity. Guidance. Trust.
