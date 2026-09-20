/* Token bridge integrity (DESIGN.md §19): the Tailwind theme file must
   define the contract tokens and the liquid-glass family.
   Background values track the approved docs/screens mockups (cool
   near-white canvas) — an intentional, commented divergence from
   §19's warm paper hex. */
import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const css = readFileSync(new URL("../styles/lumi.css", import.meta.url), "utf8");
const themeCss = readFileSync(new URL("../styles/index.css", import.meta.url), "utf8");

const REQUIRED_TOKENS = [
  "--color-lumi-blue: #006093",
  "--color-civic-navy: #102A43",
  "--color-paper-white: #F6F7FA",
  "--color-sidebar: #EDEEF3",
  "--color-signal-amber",
  "--color-risk-red: #C2410C",
  "--color-success-green: #1F7A4D",
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
