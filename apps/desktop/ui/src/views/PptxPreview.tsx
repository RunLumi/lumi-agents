/* PPTX static preview (Spec 30.10 phase C): pptx-react-viewer in
   read-only mode behind a lazy import. canEdit is hardwired false and
   no content-change callback exists — previewing never writes the
   source (§30.12), and basic-edit qualification is a separate, later
   step per the spec's supported-profile gate. The host supplies an
   empty react-i18next instance: the viewer falls back to English
   labels for its chrome; Lumi's own UI strings stay in lib/i18n.
   Disclosed limits: no animation/media autoplay, macro-enabled (.pptm)
   and legacy binary (.ppt) files stay honest-unsupported. */
import { lazy, useMemo, Suspense, useEffect, useState } from "react";
import i18next from "i18next";
import { I18nextProvider } from "react-i18next";
import { t } from "../lib/i18n";
import { fileReadBase64 } from "../ipc/commands";
import "pptx-react-viewer/styles.css";

// The package's root entry eagerly references its whole feature graph
// (charts, AI chat, templates) — the "./viewer" subpath exports the
// same PowerPointViewer with a fraction of the code, keeping the
// preview a lazy chunk instead of bloating the app bundle.
const PowerPointViewer = lazy(async () => {
  const mod = await import("pptx-react-viewer/viewer");
  return { default: mod.PowerPointViewer };
});

function baseName(path: string): string {
  return path.split("/").pop() ?? path;
}

export default function PptxPreview({
  path,
  project,
}: {
  path: string;
  project: string;
}) {
  const [bytes, setBytes] = useState<Uint8Array | null>(null);
  const [failed, setFailed] = useState<string | null>(null);

  // An isolated i18next instance: the viewer renders its chrome through
  // react-i18next; with no dictionary it derives English labels from
  // the keys. Lumi's bilingual UI strings are entirely separate.
  const viewerI18n = useMemo(() => {
    const instance = i18next.createInstance();
    void instance.init({ lng: "en", fallbackLng: "en", resources: {} });
    return instance;
  }, []);

  useEffect(() => {
    let alive = true;
    setBytes(null);
    setFailed(null);
    (async () => {
      try {
        const fc = await fileReadBase64(project, path);
        if (!alive) return;
        const bin = atob(fc.content_base64);
        const out = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
        setBytes(out);
      } catch (e) {
        if (alive) setFailed(String(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [path, project]);

  if (failed) return <div className="empty-state">{failed}</div>;
  if (bytes === null) return <div className="muted small">{t("misc.loading")}</div>;
  return (
    <div>
      <p className="muted small">{t("preview.pptxNote")}</p>
      <I18nextProvider i18n={viewerI18n}>
        <Suspense fallback={<div className="muted small">{t("misc.loading")}</div>}>
          <PowerPointViewer
            content={bytes}
            canEdit={false}
            fileName={baseName(path)}
          />
        </Suspense>
      </I18nextProvider>
    </div>
  );
}
