/* DOCX preview + basic text edit (Spec 30.8): docx-preview renderAsync
   for the layout view; an Edit-text mode rewrites body paragraph text
   in word/document.xml via JSZip and saves through the same gated
   path as every other UI write (USER Task/Run + ActionProposal +
   checksum postconditions, §30.12). renderAltChunks disabled, no
   remote relationships; the render stays inert inside an isolated
   container. Disclosed limits: run-level formatting inside an edited
   paragraph collapses to the first run's; headers/footers are not
   editable; substituted fonts and non-paginated layout remain. */
import { useEffect, useRef, useState } from "react";
import { t } from "../lib/i18n";
import { toast } from "../lib/ui";
import { boundParagraphs, changedParagraphs } from "../lib/docxEdit";
import { fileEditBase64 } from "../ipc/commands";
import { Button } from "@/components/ui/button";

const W_NS = "w:t";
const W_NS_URI = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

function base64ToBytes(base64: string): Uint8Array {
  const bin = atob(base64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

async function loadParagraphTexts(project: string, path: string): Promise<{
  bytes: Uint8Array;
  sha256: string;
  paragraphs: string[];
  zip: import("jszip");
  documentPath: string;
}> {
  const [{ fileReadBase64 }, JSZip] = await Promise.all([
    import("../ipc/commands"),
    import("jszip"),
  ]);
  const fc = await fileReadBase64(project, path);
  const bytes = base64ToBytes(fc.content_base64);
  const zip = await JSZip.loadAsync(bytes);
  const documentPath =
    zip.file("word/document.xml") != null
      ? "word/document.xml"
      : (Object.keys(zip.files).find((name) => name.endsWith("document.xml")) ?? "");
  if (!documentPath) throw new Error("document.xml not found in package");
  const xml = await zip.file(documentPath)!.async("string");
  const doc = new DOMParser().parseFromString(xml, "application/xml");
  const paragraphNodes = doc.getElementsByTagName("w:p");
  const paragraphs: string[] = [];
  for (let i = 0; i < paragraphNodes.length; i++) {
    let text = "";
    const texts = paragraphNodes[i].getElementsByTagName(W_NS);
    for (let j = 0; j < texts.length; j++) text += texts[j].textContent ?? "";
    paragraphs.push(text);
  }
  return { bytes, sha256: fc.sha256, paragraphs, zip, documentPath };
}

export default function DocxPreview({
  path,
  project,
}: {
  path: string;
  project: string;
}) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const [mode, setMode] = useState<"preview" | "edit">("preview");
  // Edit-mode state: the paragraph list, the loaded zip (for the save
  // round-trip), and pending edits keyed by paragraph index.
  const [paragraphs, setParagraphs] = useState<string[] | null>(null);
  const [originals, setOriginals] = useState<string[]>([]);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const zipRef = useRef<any>(null);
  const documentPathRef = useRef<string>("");
  const shaRef = useRef<string>("");
  const [saving, setSaving] = useState(false);
  const [reload, setReload] = useState(0);

  useEffect(() => {
    let alive = true;
    const host = hostRef.current;
    if (mode !== "preview" || !host) return;
    host.innerHTML = "";
    (async () => {
      try {
        const [, docxPreview] = await Promise.all([
          import("../ipc/commands"),
          import("docx-preview"),
        ]);
        const { bytes } = await loadParagraphTexts(project, path);
        if (!alive) return;
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
  }, [path, project, mode, reload]);

  const openEditMode = () => {
    setMode("edit");
    setFailed(null);
    (async () => {
      try {
        const loaded = await loadParagraphTexts(project, path);
        zipRef.current = loaded.zip;
        documentPathRef.current = loaded.documentPath;
        shaRef.current = loaded.sha256;
        const bounded = boundParagraphs(loaded.paragraphs);
        setOriginals(bounded);
        setParagraphs(bounded);
      } catch (e) {
        toast(String(e), "error");
      }
    })();
  };

  const recordEdit = (index: number, text: string) => {
    setParagraphs((prev) => {
      if (!prev) return prev;
      const next = [...prev];
      next[index] = text;
      return next;
    });
  };

  const saveEdits = async () => {
    const zip = zipRef.current;
    if (!zip || !paragraphs || saving) return;
    const changes = changedParagraphs(originals, paragraphs);
    if (changes.size === 0) return;
    setSaving(true);
    try {
      const xml = await zip.file(documentPathRef.current)!.async("string");
      const doc = new DOMParser().parseFromString(xml, "application/xml");
      const paragraphNodes = doc.getElementsByTagName("w:p");
      for (const [index, text] of changes) {
        const node = paragraphNodes[index];
        if (!node) continue;
        let texts = node.getElementsByTagName(W_NS);
        if (texts.length === 0 && text !== "") {
          // Paragraph had no text run (blank line): create one bound to
          // the wordprocessingml namespace so the edit actually lands.
          node.appendChild(doc.createElementNS(W_NS_URI, "w:t"));
          texts = node.getElementsByTagName(W_NS);
        }
        if (texts.length === 0) continue;
        // Basic edit semantics: the new text carries the FIRST run's
        // formatting; remaining runs in the paragraph are emptied
        // (disclosed in the hint — never silently).
        texts[0].textContent = text;
        for (let j = 1; j < texts.length; j++) texts[j].textContent = "";
      }
      const serialized = new XMLSerializer().serializeToString(doc);
      zip.file(documentPathRef.current, serialized);
      const out = await zip.generateAsync({ type: "base64" });
      await fileEditBase64(project, path, shaRef.current, out);
      setMode("preview");
      setReload((k) => k + 1);
      toast(t("preview.docxSaved"), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  if (failed) return <div className="empty-state">{failed}</div>;

  const pending = paragraphs ? changedParagraphs(originals, paragraphs).size : 0;

  return (
    <div>
      <div className="inline-form">
        <button
          className={`tab${mode === "preview" ? " active" : ""}`}
          onClick={() => setMode("preview")}
        >
          {t("preview.docxPreviewTab")}
        </button>
        <button
          className={`tab${mode === "edit" ? " active" : ""}`}
          onClick={openEditMode}
        >
          {t("preview.docxEditTab")}
        </button>
      </div>
      {mode === "preview" ? (
        <>
          <p className="muted small">{t("preview.docxDisclaimer")}</p>
          <div ref={hostRef} className="docx-host" />
        </>
      ) : (
        <>
          <p className="muted small">{t("preview.docxEditHint")}</p>
          {paragraphs === null ? (
            <div className="muted small">{t("misc.loading")}</div>
          ) : (
            <div className="docx-edit-list">
              {paragraphs.map((text, i) => (
                <textarea
                  key={i}
                  className="docx-para"
                  rows={Math.max(1, Math.ceil(text.length / 90))}
                  value={text}
                  onChange={(e) => recordEdit(i, e.target.value)}
                  aria-label={`${t("preview.docxEditTab")} ${i + 1}`}
                />
              ))}
            </div>
          )}
          <div className="inline-form" style={{ marginTop: 8 }}>
            <Button size="sm" onClick={saveEdits} disabled={pending === 0 || saving}>
              {saving ? t("preview.docxSaving") : t("preview.docxSave")}
            </Button>
            {pending > 0 && (
              <span className="muted small">
                {pending === 1
                  ? t("preview.docxChange")
                  : t("preview.docxChanges").replace("{count}", String(pending))}
              </span>
            )}
          </div>
        </>
      )}
    </div>
  );
}
