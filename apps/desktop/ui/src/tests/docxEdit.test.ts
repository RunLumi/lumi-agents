/* Unit tests for DOCX paragraph-edit helpers (Spec 30.8): change
   detection and the bounded editable surface. */
import test from "node:test";
import assert from "node:assert/strict";
import {
  boundParagraphs,
  changedParagraphs,
  MAX_EDIT_PARAGRAPHS,
} from "../lib/docxEdit.ts";

test("changedParagraphs reports only genuinely changed indices", () => {
  const original = ["one", "two", "three", ""];
  const current = ["one", "TWO", "three", "new"];
  const changes = changedParagraphs(original, current);
  assert.equal(changes.size, 2);
  assert.equal(changes.get(1), "TWO");
  assert.equal(changes.get(3), "new");
  assert.ok(!changes.has(0));
  assert.ok(!changes.has(2));
});

test("changedParagraphs treats whitespace churn as a real change", () => {
  const changes = changedParagraphs(["a"], ["a "]);
  assert.equal(changes.get(0), "a ");
});

test("changedParagraphs with no edits yields an empty map", () => {
  const original = ["x", "y"];
  assert.equal(changedParagraphs(original, [...original]).size, 0);
  assert.equal(changedParagraphs([], []).size, 0);
});

test("boundParagraphs caps the editable surface", () => {
  const many = Array.from({ length: MAX_EDIT_PARAGRAPHS + 50 }, (_, i) => `p${i}`);
  assert.equal(boundParagraphs(many).length, MAX_EDIT_PARAGRAPHS);
  assert.equal(boundParagraphs(["a"]).length, 1);
});
