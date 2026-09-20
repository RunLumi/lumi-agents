#!/usr/bin/env node
/* Verifies i18n parity and usage (ICON.md/DESIGN.md release bar):
   1. en and vi dictionaries have identical key sets;
   2. every data-i18n* key referenced in index.html exists;
   3. no user-facing emoji icons remain in static chrome. */
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url).pathname;
const fail = (msg) => { console.error("i18n check FAILED:", msg); process.exit(1); };

const i18nSrc = readFileSync(`${root}apps/desktop/src/i18n.js`, "utf8");
const enMatch = i18nSrc.match(/en:\s*\{([\s\S]*?)\n\s*\},\n\s*vi:/);
const viMatch = i18nSrc.match(/vi:\s*\{([\s\S]*?)\n\s*\},?\n/);
if (!enMatch || !viMatch) fail("cannot parse STRINGS en/vi blocks");
const keys = (block) => new Set([...block.matchAll(/"([\w.]+)":/g)].map((m) => m[1]));
const en = keys(enMatch[1]);
const vi = keys(viMatch[1]);
for (const k of en) if (!vi.has(k)) fail(`missing vi key: ${k}`);
for (const k of vi) if (!en.has(k)) fail(`missing en key: ${k}`);

const html = readFileSync(`${root}apps/desktop/src/index.html`, "utf8");
for (const m of html.matchAll(/data-i18n(?:-placeholder|-title)?="([^"]+)"/g)) {
  if (!en.has(m[1])) fail(`index.html references unknown key: ${m[1]}`);
}

// Typographic status marks (check/cross/dot) are allowed; colorful
// emoji as icons are not (ICON.md §5).
const ALLOWED_MARKS = new Set(["\u2713", "\u2717", "\u2715", "\u25CF", "\u25CB", "\u2192"]);
const EMOJI = /[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}\u{2B00}-\u{2BFF}\u{FE0F}]/gu;
for (const name of ["index.html", "app.js"]) {
  const text = readFileSync(`${root}apps/desktop/src/${name}`, "utf8");
  const emojiHits = text.split("\n").map((line, i) => {
    for (const ch of line) {
      if (EMOJI.test(ch) && !ALLOWED_MARKS.has(ch)) {
        EMOJI.lastIndex = 0;
        return { line: i + 1, ch };
      }
      EMOJI.lastIndex = 0;
    }
    return null;
  }).filter(Boolean);
  if (emojiHits.length) fail(`${name} still contains emoji icon characters at lines ${emojiHits.map((h) => `${h.line}(${h.ch})`).join(", ")}`);
}

console.log(`i18n check OK — ${en.size} keys, en/vi parity, no emoji chrome`);
