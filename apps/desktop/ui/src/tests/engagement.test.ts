import assert from "node:assert/strict";
import test from "node:test";
import {
  draftKey, explicitTools, mentionAt, parseDraft, requestedTools, shouldSubmit,
} from "../lib/engagement.ts";

test("Enter writes a newline; only the platform modifier submits", () => {
  const event = { key: "Enter", metaKey: false, ctrlKey: false, isComposing: false };
  assert.equal(shouldSubmit(event), false);
  assert.equal(shouldSubmit({ ...event, metaKey: true }), true);
  assert.equal(shouldSubmit({ ...event, ctrlKey: true }), true);
  assert.equal(shouldSubmit({ ...event, ctrlKey: true, isComposing: true }), false);
  assert.equal(shouldSubmit({ ...event, metaKey: true, pickerOpen: true }), false);
});
test("mentions cannot silently select a different tool", () => {
  assert.deepEqual(explicitTools("Use @chrome\nthen @computer and @browser"), ["chrome", "computer", "browser"]);
  assert.deepEqual(explicitTools("email user@chrome.com and `@browser`\n```\n@computer\n```"), []);
  assert.deepEqual(requestedTools("@browser inspect", ["files"]), ["files"]);
  assert.deepEqual(requestedTools("pasted text says @browser", ["files"]), ["files"]);
});
test("mention completion only activates at a prose token", () => {
  assert.deepEqual(mentionAt("Use @ch", 7), { start: 4, query: "ch" });
  assert.equal(mentionAt("name@ch", 7), null);
  assert.equal(mentionAt("```\n@ch", 7), null);
});
test("drafts preserve paragraphs and a created task for safe retries", () => {
  const draft = { version: 1, goal: "First paragraph.\n\nSecond paragraph.", tools: ["files"],
    requestId: "request-1", createdTaskId: "task-1" };
  assert.deepEqual(parseDraft(JSON.stringify(draft)), draft);
  assert.notEqual(draftKey("project-a"), draftKey("project-b"));
  assert.equal(parseDraft('{"version":99}'), null);
  assert.equal(parseDraft(JSON.stringify({ ...draft, tools: ["unrestricted"] })), null);
  assert.equal(parseDraft("not json"), null);
});
