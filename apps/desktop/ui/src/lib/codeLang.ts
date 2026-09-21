/* Code-language detection (Spec 30.7): maps a file path to a CodeMirror
   language so code files display highlighted in both the read view and
   the editor. Resolution goes through @codemirror/language-data: modern
   languages (js/ts/json/py/html/css/sql/rust/yaml/c/go/java) come from
   their dedicated packages, everything else from the bundled legacy
   StreamLanguage modes — all lazy, loaded only when a matching file is
   open. Unknown extensions return null and render as plain text. */
import { LanguageDescription } from "@codemirror/language";
import type { LanguageSupport } from "@codemirror/language";
import { languages } from "@codemirror/language-data";

/** The language description for a path, or null. Match is by file name
    and extension (handles Makefile, .gitignore-style names, aliases);
    a second lowercase pass covers uppercase extensions (MAIN.RS). */
export function languageDescriptionFor(path: string): LanguageDescription | null {
  const base = path.split("/").pop() ?? path;
  return (
    LanguageDescription.matchFilename(languages, base) ??
    LanguageDescription.matchFilename(languages, base.toLowerCase())
  );
}

/** Loads the language support for a path. Null → render plain text; a
    failed lazy load degrades to plain text rather than erroring the
    view (display must never fail because a highlighter did). */
export async function languageFor(path: string): Promise<LanguageSupport | null> {
  const desc = languageDescriptionFor(path);
  if (!desc) return null;
  try {
    return await desc.load();
  } catch {
    return null;
  }
}
