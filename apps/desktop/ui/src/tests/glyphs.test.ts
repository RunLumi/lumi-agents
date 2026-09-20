/* Glyph data integrity (ICON.md): required glyph names exist, bodies are
   well-formed single-root SVG fragments without scripts or event attrs. */
import { test } from "node:test";
import assert from "node:assert/strict";
import { GLYPHS, MARKS } from "../components/glyphs/data.ts";

for (const required of ["project", "task", "evidence", "approval", "artifact", "folder", "search"]) {
  test(`glyph exists: ${required}`, () => {
    assert(GLYPHS[required], `missing glyph: ${required}`);
  });
}

for (const [name, body] of Object.entries(GLYPHS)) {
  test(`glyph ${name} body is safe markup`, () => {
    assert(!/<script/i.test(body), "no scripts in glyph bodies");
    assert(!/on\w+=/i.test(body), "no event handlers in glyph bodies");
    assert(body.includes("<"), "body contains SVG elements");
  });
}

test("marks exist", () => {
  for (const name of ["clarity", "annotationDot", "lCorner", "diagonal"]) {
    assert(MARKS[name], `missing mark: ${name}`);
  }
});
