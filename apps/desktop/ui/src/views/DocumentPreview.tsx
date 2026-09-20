/* Document preview (Spec 30): format-detected read surfaces over the
   bounded file read IPC. Markdown renders through react-markdown +
   remark-gfm with raw HTML disabled; links are inert. Images render
   via inert <img> data URLs; media via <video>/<audio> with an honest
   decode-failure state; zip containers list their entries read-only.
   CSV/TSV is a literal bounded table — no type coercion, duplicate
   headers and ragged rows preserved. Kind detection lives in
   lib/previewKinds.ts (pure, unit-tested). Unsupported formats state
   so explicitly instead of faking a viewer. */
import { lazy, memo, Suspense, useEffect, useState } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { t } from "../lib/i18n";
import { fileReadBase64 } from "../ipc/commands";
import {
  extensionOf,
  mimeFor,
  previewKindFor,
  type PreviewKind,
} from "../lib/previewKinds";

export { extensionOf, previewKindFor };
export type { PreviewKind };

const PdfPreview = lazy(() => import("./PdfPreview"));
const DocxPreview = lazy(() => import("./DocxPreview"));
const XlsxPreview = lazy(() => import("./XlsxPreview"));
const ZipPreview = lazy(() => import("./ZipPreview"));

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

export function DataUrlPreview({ path, project, kind }: { path: string; project: string; kind: "image" | "media" }) {
  const [dataUrl, setDataUrl] = useState<string | null>(null);
  const [decodeFailed, setDecodeFailed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    setDataUrl(null);
    setDecodeFailed(false);
    setError(null);
    fileReadBase64(project, path).then((fc) => {
      if (!alive) return;
      setDataUrl(`data:${mimeFor(path)};base64,${fc.content_base64}`);
    }).catch((e) => alive && setError(String(e)));
    return () => { alive = false; };
  }, [path, project]);
  if (error) return <div className="empty-state">{error}</div>;
  if (!dataUrl) return <div className="muted small">{t("misc.loading")}</div>;
  if (kind === "image") {
    return (
      <div style={{ overflow: "auto", maxHeight: 520 }}>
        <img
          src={dataUrl}
          alt={t("preview.image")}
          style={{ maxWidth: "100%", display: "block" }}
          onError={() => setDecodeFailed(true)}
        />
        {decodeFailed && <p className="muted small">{t("preview.decodeFail")}</p>}
      </div>
    );
  }
  const isVideo = mimeFor(path).startsWith("video/");
  return (
    <div style={{ overflow: "auto", maxHeight: 520 }}>
      {isVideo ? (
        <video
          src={dataUrl}
          controls
          style={{ maxWidth: "100%", display: "block" }}
          onError={() => setDecodeFailed(true)}
        />
      ) : (
        <audio
          src={dataUrl}
          controls
          style={{ width: "100%" }}
          onError={() => setDecodeFailed(true)}
        />
      )}
      {decodeFailed && <p className="muted small">{t("preview.decodeFail")}</p>}
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
  const sep = (firstLine.includes("\t") ? "\t" : ",") as string;
  const lines = content.split(/\r?\n/).filter((_, i, all) => i < MAX_ROWS || i === all.length - 1);
  if (lines.length > MAX_ROWS) lines.length = MAX_ROWS;
  return (
    <div style={{ overflow: "auto", maxHeight: 520 }}>
      <table className="tabular" style={{ borderCollapse: "collapse", fontSize: 12 }}>
        <tbody>
          {lines.map((line, ri) => (
            <tr key={ri}>
              {line.split(sep).slice(0, MAX_COLS).map((cell, ci) => (
                <td
                  key={ci}
                  style={{
                    border: "1px solid var(--color-border-subtle)",
                    padding: "3px 8px",
                    whiteSpace: "pre",
                    background: ri === 0 ? "var(--color-surface-white)" : undefined,
                    fontWeight: ri === 0 ? 600 : undefined,
                  }}
                >
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
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
    case "media":
      return <DataUrlPreview path={path} project={project} kind={kind} />;
    case "csv":
      return <CsvPreview content={content} />;
    case "pdf":
      return (
        <Suspense fallback={<div className="muted small">{t("misc.loading")}</div>}>
          <PdfPreview path={path} project={project} />
        </Suspense>
      );
    case "docx":
      return (
        <Suspense fallback={<div className="muted small">{t("misc.loading")}</div>}>
          <DocxPreview path={path} project={project} />
        </Suspense>
      );
    case "xlsx":
      return (
        <Suspense fallback={<div className="muted small">{t("misc.loading")}</div>}>
          <XlsxPreview path={path} project={project} />
        </Suspense>
      );
    case "zip":
      return (
        <Suspense fallback={<div className="muted small">{t("misc.loading")}</div>}>
          <ZipPreview path={path} project={project} />
        </Suspense>
      );
    case "unsupported": {
      const ext = extensionOf(path);
      // Readable text (e.g. .gitignore-style or unknown text) still gets
      // the text surface; truly binary Office/PDF stay unsupported. An
      // empty content means the file was routed binary (self-fetching
      // viewers take their own path) — that is a binary too.
      const looksBinary = content === "" || content.includes("\u0000");
      return looksBinary ? <UnsupportedPreview ext={ext} /> : <pre className="code-body" style={{ maxHeight: 520, overflow: "auto" }}>{content}</pre>;
    }
    default:
      return <pre className="code-body" style={{ maxHeight: 520, overflow: "auto" }}>{content}</pre>;
  }
});
