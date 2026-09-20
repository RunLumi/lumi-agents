/* Unit tests for preview-kind detection (Spec 30.3/30.9): routing
   tables for images, media, archives, and honest-unsupported formats. */
import test from "node:test";
import assert from "node:assert/strict";
import {
  extensionOf,
  isBinaryKind,
  mimeFor,
  previewKindFor,
} from "../lib/previewKinds.ts";

test("image formats map to the image kind with correct mimes", () => {
  for (const ext of ["png", "jpg", "jpeg", "webp", "gif", "bmp", "ico", "avif", "svg"]) {
    assert.equal(previewKindFor(`a/shot.${ext}`, true), "image", ext);
  }
  assert.equal(mimeFor("x.PNG"), "image/png", "extension match is case-insensitive");
  assert.equal(mimeFor("a.jpg"), "image/jpeg");
  assert.equal(mimeFor("a.svg"), "image/svg+xml");
});

test("media formats map to the media kind with av/audio mimes", () => {
  assert.equal(previewKindFor("clip.mp4", true), "media");
  assert.equal(previewKindFor("clip.webm", true), "media");
  assert.equal(previewKindFor("clip.mov", true), "media");
  assert.equal(previewKindFor("song.mp3", true), "media");
  assert.equal(previewKindFor("song.flac", true), "media");
  assert.equal(mimeFor("clip.m4v"), "video/mp4");
  assert.equal(mimeFor("song.m4a"), "audio/mp4");
  assert.equal(mimeFor("song.wav"), "audio/wav");
});

test("zip archives get their own read-only listing kind", () => {
  assert.equal(previewKindFor("bundle.zip", true), "zip");
});

test("legacy office, macro-enabled, and undecodable images stay honestly unsupported", () => {
  for (const ext of ["doc", "xls", "ppt", "docm", "xlsm", "pptm", "heic", "tiff", "tif", "pptx"]) {
    assert.equal(previewKindFor(`f.${ext}`, true), "unsupported", ext);
  }
});

test("text families still route to text/markdown/csv and documents keep theirs", () => {
  assert.equal(previewKindFor("a/notes.md", true), "markdown");
  assert.equal(previewKindFor("a/data.csv", true), "csv");
  assert.equal(previewKindFor("a/data.tsv", true), "csv");
  assert.equal(previewKindFor("a/main.rs", true), "text");
  assert.equal(previewKindFor("a/paper.pdf", true), "pdf");
  assert.equal(previewKindFor("a/spec.docx", true), "docx");
  assert.equal(previewKindFor("a/book.xlsx", true), "xlsx");
});

test("binary kinds never go through the text read", () => {
  assert.equal(isBinaryKind("image"), true);
  assert.equal(isBinaryKind("media"), true);
  assert.equal(isBinaryKind("pdf"), true);
  assert.equal(isBinaryKind("docx"), true);
  assert.equal(isBinaryKind("xlsx"), true);
  assert.equal(isBinaryKind("zip"), true);
  assert.equal(isBinaryKind("unsupported"), true, "unsupported binaries must not hit read_text");
  assert.equal(isBinaryKind("text"), false);
  assert.equal(isBinaryKind("markdown"), false);
  assert.equal(isBinaryKind("csv"), false);
});

test("extensionOf handles dotfiles and no-extension names", () => {
  assert.equal(extensionOf("a/b/c.PNG"), "png");
  assert.equal(extensionOf(".gitignore"), "gitignore");
  assert.equal(extensionOf("Makefile"), "");
});
