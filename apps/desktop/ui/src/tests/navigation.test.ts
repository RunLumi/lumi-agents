import { test } from "node:test";
import assert from "node:assert/strict";
import { activeNavFor } from "../lib/navigation.ts";

test("sidebar follows the open project tab", () => {
  assert.equal(activeNavFor(null, "home"), "projects");
  assert.equal(activeNavFor("project-1", "home"), "projects");
  assert.equal(activeNavFor("project-1", "files"), "files");
  assert.equal(activeNavFor("project-1", "settings"), "projects");
});
