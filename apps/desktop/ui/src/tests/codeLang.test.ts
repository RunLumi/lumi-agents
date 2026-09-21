/* Unit tests for code-language detection (Spec 30.7): path→language
   routing for the read view and the editor. Only the pure matching
   layer is tested here; support loading is browser-side. */
import test from "node:test";
import assert from "node:assert/strict";
import { languageDescriptionFor } from "../lib/codeLang.ts";

test("dedicated modern languages match by extension", () => {
  const cases: [string, string][] = [
    ["main.rs", "Rust"],
    ["app.py", "Python"],
    ["index.js", "JavaScript"],
    ["comp.tsx", "TSX"],
    ["data.json", "JSON"],
    ["style.css", "CSS"],
    ["page.html", "HTML"],
    ["query.sql", "SQL"],
    ["conf.yaml", "YAML"],
    ["conf.yml", "YAML"],
    ["engine.cpp", "C++"],
    ["server.go", "Go"],
    ["Main.java", "Java"],
  ];
  for (const [file, expected] of cases) {
    const desc = languageDescriptionFor(`src/${file}`);
    assert.ok(desc, `${file} should match a language`);
    assert.equal(desc!.name, expected, file);
  }
});

test("bundled legacy modes match without extra packages", () => {
  for (const file of ["build.sh", "Cargo.toml", "app.rb", "index.php", "lib.lua", "server.py"]) {
    assert.ok(languageDescriptionFor(file), `${file} should match`);
  }
});

test("match is case-insensitive on extension", () => {
  assert.equal(languageDescriptionFor("A/MAIN.RS")?.name, "Rust");
  assert.equal(languageDescriptionFor("Index.TSX")?.name, "TSX");
});

test("unknown extensions return null and render plain", () => {
  assert.equal(languageDescriptionFor("x.unknownext"), null);
  assert.equal(languageDescriptionFor(""), null);
});
