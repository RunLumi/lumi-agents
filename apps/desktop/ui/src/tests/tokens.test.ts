/* Token bridge integrity (DESIGN.md §19): the Tailwind theme file must
   define the contract tokens and the liquid-glass family. */
import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const css = readFileSync(new URL("../styles/index.css", import.meta.url), "utf8");

const REQUIRED_TOKENS = [
  "--color-lumi-blue: #006093",
  "--color-civic-navy: #102A43",
  "--color-paper-white: #F4F0E8",
  "--color-signal-amber: #F4A62A",
  "--color-risk-red: #C2410C",
  "--color-success-green: #1F7A4D",
  "--glass-tint",
  "--glass-tint-strong",
  "--glass-blur: 18px",
  "--glass-blur-strong: 32px",
  "--elevation-1",
  "--shadow-modal",
];

for (const token of REQUIRED_TOKENS) {
  test(`token present: ${token}`, () => {
    assert(css.includes(token), `missing token: ${token}`);
  });
}

test("glass degrades without backdrop-filter", () => {
  assert(css.includes("@supports not"), "opaque fallback required");
  assert(css.includes("prefers-reduced-transparency"), "reduced-transparency fallback required");
});
