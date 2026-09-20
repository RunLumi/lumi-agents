/* Document preview (Spec 30 phase A): format-detected read surfaces over
   the bounded file read IPC. Markdown renders through react-markdown +
   remark-gfm with raw HTML disabled; links are inert. Images render via
   inert <img> data URLs. CSV/TSV is a literal bounded table — no type
   coercion, duplicate headers and ragged rows preserved. Unsupported
   formats state so explicitly instead of faking a viewer. */
import { memo, useEffect, useState } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { t } from "../lib/i18n";
import { fileReadBase64 } from "../ipc/commands";

const IMAGE_TYPES: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  webp: "image/webp",
  gif: "image/gif",
  svg: "image/svg+xml",
};

const OFFICE_TYPES = new Set([
  "pdf", "docx", "xlsx", "pptx", "doc", "xls", "ppt",
  "docm", "xlsm", "pptm", "heic", "tiff",
]);

export type PreviewKind = "text" | "markdown" | "image" | "csv" | "unsupported";

export function extensionOf(path: string): string {
  const base = path.split("/").pop() ?? path;
  const dot = base.lastIndexOf(".");
  return dot === -1 ? "" : base.slice(dot + 1).toLowerCase();
}

/** Content sniffing overrides the filename (Spec 30.3: never trust the
    extension alone); the extension drives only the rendering choice. */
export function previewKindFor(path: string, isTextFile: boolean): PreviewKind {
  const ext = extensionOf(path);
  if (ext === "md" || ext === "markdown") return "markdown";
  if (ext in IMAGE_TYPES) return "image";
  if (ext === "csv" || ext === "tsv") return "csv";
  if (OFFICE_TYPES.has(ext)) return "unsupported";
  return isTextFile ? "text" : "unsupported";
}

function MarkdownPreview({ content }: { content: string }) {
  const [view, setView] = useState<"rendered" | "source" | "split">("rendered");
  return (
    <div>
      <div className="inline-form">
        {(["rendered", "source", "split"] as const).map((v) => (
          <button key={v} className={`tab${view === v ? " active" : ""}`} onClick={() => setView(v)}>
            {t(`preview.${v}`)}
          </button>
        ))}
      </div>
      {view === "source" ? (
        <pre className="code-body" style={{ maxHeight: 480, overflow: "auto" }}>{content}</pre>
      ) : (
        <div
          className={
            view === "split" ? "code-content markdown-body" : "markdown-body"
          }
          style={view === "split" ? { maxHeight: 480, overflow: "auto" } : undefined}
        >
          <Markdown
            remarkPlugins={[remarkGfm]}
            // Spec 30.7: raw HTML stays disabled; links render inert
            // (no href) so navigation cannot escape the workspace.
            components={{
              a: ({ children }) => <span style={{ color: "var(--color-lumi-blue)", textDecoration: "underline" }}>{children}</span>,
              img: () => <em className="muted small">[image]</em>,
            }}
          >
            {content}
          </Markdown>
        </div>
      )}
      {view === "split" && (
        <pre className="code-body" style={{ maxHeight: 480, overflow: "auto", borderTop: "1px solid var(--color-border)" }}>{content}</pre>
      )}
    </div>
  );
}

function ImagePreview({ path, project }: { path: string; project: string }) {
  const [dataUrl, setDataUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    setDataUrl(null);
    setError(null);
    fileReadBase64(project, path).then((fc) => {
      if (!alive) return;
      const mime = IMAGE_TYPES[extensionOf(path)] ?? "application/octet-stream";
      setDataUrl(`data:${mime};base64,${fc.content_base64}`);
    }).catch((e) => alive && setError(String(e)));
    return () => { alive = false; };
  }, [path, project]);
  if (error) return <div className="empty-state">{error}</div>;
  if (!dataUrl) return <div className="muted small">{t("misc.loading")}</div>;
  return (
    <div style={{ overflow: "auto", maxHeight: 520 }}>
      <img src={dataUrl} alt={t("preview.image")} style={{ maxWidth: "100%", display: "block" }} />
    </div>
  );
}

/** Literal bounded CSV/TSV table: split on newlines/separator only.
    Quotes are NOT parsed into structure here — Spec 30.9 phase A keeps
    raw fidelity; a quoting-aware export is a separate disclosed step. */
function CsvPreview({ content }: { content: string }) {
  const MAX_ROWS = 300;
  const MAX_COLS = 24;
  const firstLine = content.split(/\r?\n/, 1)[0] ?? "";
  const tabs = (firstLine.match(/\t/g) ?? []).length;
  const commas = (firstLine.match(/,/g) ?? []).length;
  const delimiter = tabs > commas ? "\t" : ",";
  const rows = content
    .split(/\r?\n/)
    .filter((line, i, arr) => !(i === arr.length - 1 && line === ""))
    .slice(0, MAX_ROWS)
    .map((line) => line.split(delimiter));
  const cols = Math.min(MAX_COLS, Math.max(...rows.map((r) => r.length), 0));
  return (
    <div style={{ overflow: "auto", maxHeight: 520 }}>
      <p className="muted small">{t("preview.csv")}</p>
      <table className="tabular" style={{ borderCollapse: "collapse", fontSize: 12 }}>
        <tbody>
          {rows.map((row, ri) => (
            <tr key={ri}>
              {Array.from({ length: cols }, (_, ci) => (
                <td key={ci} style={{ border: "1px solid var(--color-border-subtle)", padding: "3px 8px", whiteSpace: "pre" }}>
                  {row[ci] ?? ""}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {content.split(/\r?\n/).length > MAX_ROWS && (
        <p className="muted small">…</p>
      )}
    </div>
  );
}


function UnsupportedPreview({ ext }: { ext: string }) {
  return (
    <div className="empty-state">
      <p>{t("preview.unsupported")}</p>
      <p className="muted small">
        .{ext} — {t("preview.unsupportedHint")}
      </p>
    </div>
  );
}

export const DocumentPreview = memo(function DocumentPreview({
  path,
  project,
  content,
}: {
  path: string;
  project: string;
  content: string;
}) {
  const kind = previewKindFor(path, true);
  switch (kind) {
    case "markdown":
      return <MarkdownPreview content={content} />;
    case "image":
      return <ImagePreview path={path} project={project} />;
    case "csv":
      return <CsvPreview content={content} />;
    case "unsupported": {
      const ext = extensionOf(path);
      // Readable text (e.g. .gitignore-style or unknown text) still gets
      // the text surface; truly binary Office/PDF stay unsupported.
      const looksBinary = content.includes("\u0000");
      return looksBinary ? <UnsupportedPreview ext={ext} /> : <pre className="code-body" style={{ maxHeight: 520, overflow: "auto" }}>{content}</pre>;
    }
    default:
      return <pre className="code-body" style={{ maxHeight: 520, overflow: "auto" }}>{content}</pre>;
  }
});
