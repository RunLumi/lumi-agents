/* Token bridge integrity (DESIGN.md §19): the desktop token layer must
   preserve the canonical palette, shape, type, and liquid-glass family. */
import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const css = readFileSync(new URL("../styles/lumi.css", import.meta.url), "utf8");
const themeCss = readFileSync(new URL("../styles/index.css", import.meta.url), "utf8");

const REQUIRED_TOKENS = [
  "--color-lumi-blue: #006093",
  "--color-lumi-blue-soft: #E4F3FC",
  "--color-civic-navy: #102A43",
  "--color-paper-white: #F4F0E8",
  "--color-signal-amber",
  "--color-risk-red: #C2410C",
  "--color-success-green: #1F7A4D",
  "--border-focus: 2px",
  "--border-rail: 3px",
  "--text-display: clamp(2.5rem, 1.90rem + 3.00vw, 4.5rem)",
  "--glass-tint",
  "--glass-tint-strong",
  "--glass-blur: 18px",
  "--glass-blur-strong: 32px",
  "--shadow-elevation-1",
  "--shadow-modal",
];

for (const token of REQUIRED_TOKENS) {
  test(`token present: ${token}`, () => {
    assert(css.includes(token), `missing token: ${token}`);
  });
}

test("token present: Geist served locally (§6.1)", () => {
  assert(
    themeCss.includes("@fontsource-variable/geist") &&
      css.includes('"Geist Variable"') &&
      css.includes('"Geist Mono Variable"'),
    "Geist must be bundled locally and lead the font stacks",
  );
});

test("glass degrades without backdrop-filter", () => {
  assert(css.includes("@supports not"), "opaque fallback required");
  assert(css.includes("prefers-reduced-transparency"), "reduced-transparency fallback required");
});

test("desktop does not reintroduce screen-specific palette overrides", () => {
  assert(!css.includes("--color-sidebar:"), "sidebar must use a canonical surface token");
  assert(!css.includes("--color-amber-hue:"), "amber must have one canonical token");
  assert(css.includes("height: 100dvh"), "desktop shell must use the stable dynamic viewport height");
});

test("selected project tabs have an explicit visible state", () => {
  assert(
    css.includes('[data-slot="tabs-trigger"][data-state="active"]'),
    "Radix selected state must have an explicit Lumi CSS rule",
  );
  assert(
    css.includes("background: var(--color-lumi-blue-soft)"),
    "selected tabs must expose the selected surface",
  );
  assert(
    css.includes("border-bottom-color: var(--color-lumi-blue)"),
    "selected tabs must expose the selected rail",
  );
});

test("desktop keeps the shared job usable at the minimum window size", () => {
  assert(css.includes("font-size: 16px"), "the root scale must preserve the documented desktop base");
  assert(css.includes("font-size: 1rem"), "body and native fields must retain the documented input scale");
  assert(css.includes("min-height: 40px; padding: 10px 12px"), "tabs must retain the documented control rhythm");
  assert(css.includes("min-height: 52px"), "task rows must retain a readable minimum height");
  assert(css.includes(".action-cards, .recent-grid, .stat-cards { grid-template-columns: 1fr; }"), "dense grids must stack on narrow windows");
  assert(css.includes(".topbar { flex-wrap: wrap; gap: 8px; padding-inline: 12px; }"), "topbar must reflow within the minimum window");
});

test("the desktop stylesheet participates in the Tailwind cascade deliberately", () => {
  assert(css.includes("@layer base"), "resets must live in the base layer");
  assert(css.includes("@layer components"), "Lumi component styles must be layered");
});
