/* DOCX preview (Spec 30.8): docx-preview renderAsync — best-effort
   document layout. renderAltChunks disabled, no remote relationships;
   the document renders inside an isolated container with its own
   injected styles (style-src 'unsafe-inline' covers it). Substituted
   fonts and non-paginated layout are disclosed limitations. */
import { useEffect, useRef, useState } from "react";
import { t } from "../lib/i18n";

export default function DocxPreview({
  path,
  project,
}: {
  path: string;
  project: string;
}) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [failed, setFailed] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    const host = hostRef.current;
    if (!host) return;
    host.innerHTML = "";
    (async () => {
      try {
        const [{ fileReadBase64 }, docxPreview] = await Promise.all([
          import("../ipc/commands"),
          import("docx-preview"),
        ]);
        const fc = await fileReadBase64(project, path);
        if (!alive) return;
        const bin = atob(fc.content_base64);
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        await docxPreview.renderAsync(bytes, host, undefined, {
          // Spec 30.8: disable altChunks and keep the render inert.
          renderAltChunks: false,
          inWrapper: true,
          ignoreLastRenderedPageBreak: false,
          experimental: false,
          useBase64URL: true,
        });
      } catch (e) {
        if (alive) setFailed(String(e));
      }
    })();
    return () => {
      alive = false;
      if (host) host.innerHTML = "";
    };
  }, [path, project]);

  if (failed) return <div className="empty-state">{failed}</div>;
  return (
    <div>
      <p className="muted small">{t("preview.docxDisclaimer")}</p>
      <div ref={hostRef} className="docx-host" />
    </div>
  );
}
