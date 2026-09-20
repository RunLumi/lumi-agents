/* Command-surface drift test (goal §8): every command registered in
   src-tauri generate_handler! must exist in COMMAND_NAMES, and vice
   versa. Renaming/removing either side fails this test. */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { COMMAND_NAMES } from "../ipc/commands.ts";

const uiDir = fileURLToPath(new URL("../", import.meta.url));
const repoRoot = uiDir.replace(/apps\/desktop\/ui\/src\/$/, "");

const libRs = readFileSync(`${repoRoot}apps/desktop/src-tauri/src/lib.rs`, "utf8");
const handlerMatch = libRs.match(/generate_handler!\[([\s\S]*?)\]/);
assert(handlerMatch, "generate_handler! block found in lib.rs");
const rustCommands = new Set(
  [...handlerMatch[1].matchAll(/(\w+)/g)].map((m) => m[1]),
);

for (const name of COMMAND_NAMES) {
  assert(rustCommands.has(name), `command ${name} in COMMAND_NAMES is missing from generate_handler!`);
}
for (const name of rustCommands) {
  assert(
    (COMMAND_NAMES as readonly string[]).includes(name),
    `generate_handler! command ${name} is missing from COMMAND_NAMES`,
  );
}

test("command surface is in sync on both sides", () => {
  assert.equal(rustCommands.size, COMMAND_NAMES.length);
});
