/* CodeMirror 6 editor for Spec 30.7 text/Markdown editing: undo/redo
   (history), find/replace (search panel, Mod-f), markdown language when
   the file is .md. Deliberately minimal per §30.15 — no autocompletion,
   folding or active-line decoration in the basic profile. */
import CodeMirror from "@uiw/react-codemirror";
import { markdown as markdownLang } from "@codemirror/lang-markdown";
import { EditorView } from "@codemirror/view";

export default function CodeEditor({
  value,
  path,
  onChange,
}: {
  value: string;
  path: string;
  onChange: (value: string) => void;
}) {
  const isMarkdown = path.toLowerCase().endsWith(".md");
  return (
    <CodeMirror
      value={value}
      height="420px"
      theme="light"
      extensions={[
        EditorView.lineWrapping,
        ...(isMarkdown ? [markdownLang()] : []),
      ]}
      basicSetup={{
        lineNumbers: true,
        foldGutter: false,
        highlightActiveLine: false,
        autocompletion: false,
        searchKeymap: true, // Mod-f find/replace panel (§30.7)
        history: true, // undo/redo
      }}
      onChange={onChange}
      style={{ fontSize: 13 }}
    />
  );
}
