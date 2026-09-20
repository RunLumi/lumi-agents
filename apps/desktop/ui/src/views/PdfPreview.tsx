/* PDF preview (Spec 30.11): self-hosted pdfjs-dist with a matched
   bundled worker, lazy page rendering, text search over page content.
   Scripting/launch actions/attachments/remote content are never
   executed — only canvas rendering of page bitmaps. The worker and
   library are dynamically imported so they leave the initial shell
   bundle (§30.15). */
import { useEffect, useMemo, useRef, useState } from "react";
import { t } from "../lib/i18n";
import { Field } from "@/components/ui/field";

const MAX_SEARCH_PAGES = 200;

export default function PdfPreview({
  path,
  project,
}: {
  path: string;
  project: string;
}) {
  const [numPages, setNumPages] = useState(0);
  const [failed, setFailed] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<Map<number, number> | null>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const docRef = useRef<any>(null);
  const canvases = useRef<Map<number, HTMLCanvasElement>>(new Map());

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const [{ fileReadBase64 }, pdfjs] = await Promise.all([
          import("../ipc/commands"),
          import("pdfjs-dist"),
        ]);
        const workerUrl = (await import("pdfjs-dist/build/pdf.worker.min.mjs?url"))
          .default;
        pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;
        const fc = await fileReadBase64(project, path);
        if (!alive) return;
        const bin = atob(fc.content_base64);
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        const doc = await pdfjs.getDocument({ data: bytes }).promise;
        if (!alive) return;
        docRef.current = doc;
        setNumPages(doc.numPages);
      } catch (e) {
        if (alive) setFailed(String(e));
      }
    })();
    return () => {
      alive = false;
      docRef.current?.destroy?.();
      docRef.current = null;
    };
  }, [path, project]);

  // Lazy page rendering: each mounted canvas renders its own page once.
  useEffect(() => {
    const doc = docRef.current;
    if (!doc || numPages === 0) return;
    let cancelled = false;
    (async () => {
      for (const [pageNum, canvas] of canvases.current) {
        if (cancelled || canvas.dataset.rendered === "1") continue;
        const page = await doc.getPage(pageNum).catch(() => null);
        if (!page || cancelled) continue;
        const scale = 1.4;
        const viewport = page.getViewport({ scale });
        const dpr = window.devicePixelRatio || 1;
        canvas.width = viewport.width * dpr;
        canvas.height = viewport.height * dpr;
        canvas.style.width = `${viewport.width}px`;
        canvas.style.height = `${viewport.height}px`;
        const ctx = canvas.getContext("2d");
        if (!ctx) continue;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        await page.render({ canvasContext: ctx, viewport }).promise;
        canvas.dataset.rendered = "1";
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [numPages, hits]);

  const search = useMemo(
    () => async () => {
      const doc = docRef.current;
      const q = query.trim().toLowerCase();
      if (!doc || !q) {
        setHits(null);
        return;
      }
      const found = new Map<number, number>();
      const limit = Math.min(doc.numPages, MAX_SEARCH_PAGES);
      for (let p = 1; p <= limit; p++) {
        const page = await doc.getPage(p).catch(() => null);
        if (!page) continue;
        const text = await page.getTextContent();
        let count = 0;
        for (const item of text.items as Array<{ str?: string }>) {
          const hay = (item.str ?? "").toLowerCase();
          let idx = hay.indexOf(q);
          while (idx !== -1) {
            count++;
            idx = hay.indexOf(q, idx + q.length);
          }
        }
        if (count > 0) found.set(p, count);
      }
      setHits(found);
    },
    [query],
  );

  if (failed) return <div className="empty-state">{failed}</div>;

  return (
    <div>
      <div className="inline-form">
        <Field id="pdf-search" label={t("preview.pdfSearchLabel")}>
          <input
            id="pdf-search"
            className="input"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("preview.pdfSearch")}
            onKeyDown={(e) => e.key === "Enter" && search()}
          />
        </Field>
        <button className="btn btn-sm" onClick={search}>{t("preview.pdfSearchGo")}</button>
        <span className="muted small">
          {numPages > 0 ? `${numPages} pages` : t("misc.loading")}
        </span>
      </div>
      {hits && (
        <div className="mini-list">
          {hits.size === 0 && (
            <div className="muted small">{t("preview.pdfNoHits")}</div>
          )}
          {[...hits.entries()].map(([page, count]) => (
            <button
              key={page}
              className="mini-row"
              style={{ cursor: "pointer", textAlign: "left" }}
              onClick={() =>
                canvases.current
                  .get(page)
                  ?.scrollIntoView({ behavior: "smooth", block: "start" })
              }
            >
              <span className="small">
                {t("preview.pdfPage")} {page} — {count}
              </span>
            </button>
          ))}
        </div>
      )}
      <div style={{ overflow: "auto", maxHeight: 560 }}>
        {Array.from({ length: numPages }, (_, i) => (
          <div key={i + 1} style={{ marginBottom: 12 }}>
            <PdfCanvas
              register={(canvas) => {
                if (canvas) canvases.current.set(i + 1, canvas);
                else canvases.current.delete(i + 1);
              }}
              page={i + 1}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

function PdfCanvas({
  page,
  register,
}: {
  page: number;
  register: (canvas: HTMLCanvasElement | null) => void;
}) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  useEffect(() => {
    register(ref.current);
    return () => register(null);
  }, [register]);
  return (
    <div style={{ position: "relative" }}>
      <canvas ref={ref} data-page={page} style={{ border: "1px solid var(--color-border)" }} />
      <span className="muted small" style={{ position: "absolute", top: 4, right: 8 }}>
        {page}
      </span>
    </div>
  );
}
