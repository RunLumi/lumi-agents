/* Read-only code view (Spec 30.7): CodeMirror with per-language syntax
   highlighting and line numbers over the bounded file read. Editing
   happens through CodeEditor — this surface never mutates. Language
   resolution is async and lazy; an unknown extension or a failed
   highlighter load degrades to plain text instead of an error. */
import { useEffect, useState } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { EditorView } from "@codemirror/view";
import type { LanguageSupport } from "@codemirror/language";
import { languageFor } from "../lib/codeLang";

export default function CodeView({
  value,
  path,
}: {
  value: string;
  path: string;
}) {
  const [lang, setLang] = useState<LanguageSupport | null>(null);

  useEffect(() => {
    let alive = true;
    setLang(null);
    languageFor(path).then((support) => {
      if (alive) setLang(support);
    });
    return () => {
      alive = false;
    };
  }, [path]);

  return (
    <CodeMirror
      value={value}
      height="480px"
      theme="light"
      editable={false}
      extensions={[EditorView.lineWrapping, ...(lang ? [lang] : [])]}
      basicSetup={{
        lineNumbers: true,
        foldGutter: false,
        highlightActiveLine: false,
        autocompletion: false,
        searchKeymap: true, // Mod-f find still useful read-only
        history: false,
      }}
      style={{ fontSize: 13 }}
    />
  );
}
