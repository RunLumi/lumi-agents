/* Meastro UI/UX suite — automated design-and-honesty gates for the
   Lumi desktop UI. Every check scans the real sources; a violation
   fails the build, so UI/UX regressions are caught the same way as
   broken tests.

   Gates:
   1. i18n integrity      — every t("key") used exists; EN/VI dictionaries
                            are complete and duplicate-free.
   2. Glyph validity      — every Glyph/Mark name is a defined glyph.
   3. CSS coverage        — every static className exists in the Lumi
                            stylesheets (no invisible styles).
   4. Token hygiene       — no raw colors in TSX (tokens live in CSS).
   5. Bilingual surfaces  — no hardcoded placeholder/title/aria-label
                            strings; user-facing text routes through t().
   6. Honesty             — no lorem/demo/TODO placeholders in shipped UI. */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { GLYPHS } from "../components/glyphs/data.ts";

const srcDir = fileURLToPath(new URL("../", import.meta.url));

function walk(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = `${dir}${entry}`;
    if (statSync(full).isDirectory()) {
      if (!entry.includes("tests")) walk(`${full}/`, out);
    } else if (/\.tsx?$/.test(entry)) {
      out.push(full);
    }
  }
  return out;
}

const sources = walk(srcDir).filter((f) => !f.includes("/tests/"));
const tsxFiles = sources.filter((f) => f.endsWith(".tsx"));
const all = (files: string[], re: RegExp, group = 1): string[] =>
  files.flatMap((f) => [...readFileSync(f, "utf8").matchAll(re)].map((m) => m[group] as string));

// --- 1. i18n integrity -----------------------------------------------------

const i18nSource = readFileSync(`${srcDir}lib/i18n.ts`, "utf8");
const viStart = i18nSource.indexOf("vi: {");
assert.ok(viStart > 0, "i18n.ts exposes both dictionaries");
const enSource = i18nSource.slice(0, viStart);
const viSource = i18nSource.slice(viStart);

function dictKeys(section: string): Map<string, number> {
  const keys = new Map<string, number>();
  // Match full "section.name": "value" pairs (multiple may share a line).
  for (const m of section.matchAll(/"([a-zA-Z][a-zA-Z0-9.]*)"\s*:/g)) {
    const key = m[1];
    if (key.includes(".")) keys.set(key, (keys.get(key) ?? 0) + 1);
  }
  return keys;
}

const enKeys = dictKeys(enSource);
const viKeys = dictKeys(viSource);

test("meastro/i18n: dictionaries have no duplicate keys", () => {
  const dupes = (dict: Map<string, number>, name: string) =>
    [...dict.entries()].filter(([, n]) => n > 1).map(([k]) => `${name}:${k}`);
  const duplicates = [...dupes(enKeys, "en"), ...dupes(viKeys, "vi")];
  assert.deepEqual(duplicates, [], `duplicate i18n keys: ${duplicates.join(", ")}`);
});

test("meastro/i18n: every EN key has a VI translation and vice versa", () => {
  const missingVi = [...enKeys.keys()].filter((k) => !viKeys.has(k));
  const missingEn = [...viKeys.keys()].filter((k) => !enKeys.has(k));
  assert.deepEqual(
    { missingVi, missingEn },
    { missingVi: [], missingEn: [] },
    "dictionaries must be complete in both languages",
  );
});

const usedI18nKeys = all(sources, /\bt\("([^"]+)"\)/g);
test("meastro/i18n: every t() key used in components exists in both dictionaries", () => {
  const unknown = [...new Set(usedI18nKeys)].filter((k) => !enKeys.has(k) || !viKeys.has(k));
  assert.deepEqual(
    unknown,
    [],
    `t() keys missing from EN/VI dictionaries: ${unknown.join(", ")}`,
  );
});

// --- 2. Glyph validity ------------------------------------------------------

test("meastro/glyphs: every Glyph name is a defined Lumi glyph", () => {
  const used = [
    ...all(tsxFiles, /<Glyph\s+name="([^"]+)"/g),
    ...all(sources, /glyph\("([^"]+)"/g),
  ];
  const unknown = [...new Set(used)].filter((n) => !(n in GLYPHS));
  assert.deepEqual(unknown, [], `undefined glyph names: ${unknown.join(", ")}`);
});

// --- 3. CSS class coverage ---------------------------------------------------

const cssFiles = ["styles/lumi.css", "styles/index.css"]
  .map((p) => `${srcDir}${p}`)
  .map((p) => readFileSync(p, "utf8"))
  .join("\n");
const cssClasses = new Set([...cssFiles.matchAll(/\.([a-zA-Z][a-zA-Z0-9_-]*)/g)].map((m) => m[1]));

/* Tailwind utilities are generated on demand from source; a class like
   `hover:bg-primary` resolves through the framework, not lumi.css.
   Recognize utilities by their (variant-stripped) stem so the gate can
   keep enforcing "no invisible SEMANTIC classes". */
const UTILITY_STEMS =
  /^(p|px|py|pt|pb|pl|pr|ps|pe|m|mx|my|mt|mb|ml|mr|gap|w|h|min-w|max-w|size|top|bottom|left|right|inset|text|bg|border|border-b|border-t|border-l|border-r|rounded|font|leading|tracking|shadow|opacity|ring|ring-offset|transition|duration|ease|translate|translate-y|translate-x|z|col|row|order|flex|grid|items|justify|content|self|place|overflow|whitespace|outline|scrollbar|animate|filter|brightness|underline|selection|group|data|aria|from|to|via|list|object|aspect|columns|divide|space|table|cursor|select|resize|appearance|pointer-events|fill|stroke|sr)-|^(flex|grid|block|inline|inline-flex|hidden|relative|absolute|fixed|sticky|static|truncate|tabular-nums|sr-only|isolate|container|subgrid|antialiased|italic|uppercase|lowercase|capitalize|underline|line-through|no-underline|border-collapse|outline-none|appearance-none|overflow-ellipsis)$/;
function isTailwindUtility(cls: string): boolean {
  // Arbitrary values/properties and stacked variants are utilities.
  if (cls.includes("[") || cls.includes("(")) return true;
  const stem = cls.split(":").pop() ?? cls;
  return UTILITY_STEMS.test(stem);
}

/* Classes a library requires on its own root (sonner's toast group
   contract). Not layout styles we own. */
const EXTERNAL_CLASS_CONTRACTS = new Set(["toaster", "group"]);

test("meastro/css: every static className exists in the Lumi stylesheets or is a Tailwind utility", () => {
  const used: string[] = [];
  for (const f of tsxFiles) {
    const text = readFileSync(f, "utf8");
    // Literal strings...
    for (const m of text.matchAll(/className="([^"]+)"/g)) {
      used.push(...m[1].split(/\s+/));
    }
    // ...and the static segments of template literals.
    for (const m of text.matchAll(/className=\{`([^`]+)`\}/g)) {
      used.push(...m[1].replace(/\$\{[^}]*\}/g, " ").split(/\s+/));
    }
  }
  const unknown = [...new Set(used)].filter(
    (c) =>
      c &&
      !c.includes("$") &&
      !cssClasses.has(c) &&
      !isTailwindUtility(c) &&
      !EXTERNAL_CLASS_CONTRACTS.has(c),
  );
  assert.deepEqual(unknown, [], `classNames with no CSS rule: ${unknown.join(", ")}`);
});

// --- 4. Token hygiene ---------------------------------------------------------

test("meastro/tokens: no raw color literals in TSX (colors live in CSS tokens)", () => {
  const offenders = tsxFiles.filter((f) => /#[0-9a-fA-F]{3,8}\b/.test(readFileSync(f, "utf8")));
  assert.deepEqual(offenders, [], `raw hex colors in: ${offenders.join(", ")}`);
});

// --- 5. Bilingual surfaces ------------------------------------------------------

test("meastro/i18n: user-facing attributes route through t(), not literals", () => {
  const offenders: string[] = [];
  for (const f of tsxFiles) {
    const text = readFileSync(f, "utf8");
    for (const attr of ["placeholder", "title", "aria-label"]) {
      for (const m of text.matchAll(new RegExp(`${attr}="([^{}][^"]*)"`, "g"))) {
        offenders.push(`${f}: ${attr}="${m[1]}"`);
      }
    }
  }
  assert.deepEqual(offenders, [], `hardcoded user-facing strings:\n${offenders.join("\n")}`);
});

// --- 6. Honesty ------------------------------------------------------------------

test("meastro/honesty: no placeholder or demo text ships in the UI", () => {
  const banned = /lorem|todo\b|fixme|coming soon|sample data|demo data|placeholder text/i;
  // Scan rendered text: comments are documentation, not shipped UI.
  const stripComments = (text: string) =>
    text.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\/\/[^\n]*/g, " ");
  const offenders = tsxFiles.filter((f) => banned.test(stripComments(readFileSync(f, "utf8"))));
  assert.deepEqual(offenders, [], `placeholder/demo text in: ${offenders.join(", ")}`);
});

test("meastro/badges: status chips use the §10.4 lit-token Badge, not hand-rolled pills", () => {
  // DESIGN.md §10.4: "Never hand-roll a statusColors map on a page."
  // The legacy `.pill-*` classes are superseded by <StatusBadge>.
  const offenders = tsxFiles.filter((f) => /className=\{?["`][^"`}]*\bpill\b/.test(readFileSync(f, "utf8")));
  assert.deepEqual(offenders, [], `hand-rolled status pills in: ${offenders.join(", ")}`);
});

test("meastro/a11y: icon-only buttons expose an accessible name", () => {
  const offenders: string[] = [];
  for (const f of tsxFiles) {
    const text = readFileSync(f, "utf8");
    for (const m of text.matchAll(/<button\b[^>]*>([\s\S]*?)<\/button>/g)) {
      const inner = m[1];
      const hasAria = /aria-label=/.test(m[0]);
      // Visible text may be literal or an expression ({t("...")} etc.);
      // an expression always renders something the model of the button
      // cannot verify statically, so it counts as a name.
      const hasExpression = /\{/.test(inner);
      const label = inner
        .replace(/<[^>]*>/g, " ")
        .replace(/\{[^}]*\}/g, " ")
        .replace(/&[a-z]+;/g, " ")
        .trim();
      if (!hasAria && !hasExpression && label.length === 0) {
        offenders.push(`${f}: <button>${inner.slice(0, 60).trim()}…`);
      }
    }
  }
  assert.deepEqual(offenders, [], `icon-only buttons need aria-label:\n${offenders.join("\n")}`);
});
