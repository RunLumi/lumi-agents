/* Unit tests for XLSX edit helpers (Spec 30.9): key naming and the
   text→cell-value conversion applied before a gated save. */
import test from "node:test";
import assert from "node:assert/strict";
import { cellValueFromText, editKey, parseEditKey } from "../lib/xlsxEdit.ts";

test("edit keys round-trip sheet/row/col", () => {
  const key = editKey(2, 14, 7);
  assert.equal(key, "2:14:7");
  assert.deepEqual(parseEditKey(key), { sheet: 2, row: 14, col: 7 });
});

test("numeric cells keep their type when the text parses", () => {
  assert.equal(cellValueFromText("42", true), 42);
  assert.equal(cellValueFromText(" 3.5 ", true), 3.5);
  assert.equal(cellValueFromText("-7", true), -7);
});

test("numeric cells degrade to honest strings, never NaN", () => {
  assert.equal(cellValueFromText("abc", true), "abc");
  assert.equal(cellValueFromText("1.2.3", true), "1.2.3");
  assert.ok(!Number.isNaN(cellValueFromText("abc", true) as unknown as number));
});

test("text cells stay strings; clearing yields empty string", () => {
  assert.equal(cellValueFromText("hello", false), "hello");
  assert.equal(cellValueFromText("123", false), "123", "text-typed cells keep text");
  assert.equal(cellValueFromText("", false), "");
  assert.equal(cellValueFromText("", true), "");
});
