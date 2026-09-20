/* ZIP archive listing (Spec 30): JSZip behind a lazy import — a
   read-only entry table (name, sizes). No extraction: writing files
   out of an archive is a gated write action, not a preview side
   effect, and stays one. Bounded at 500 entries. */
import { useEffect, useState } from "react";
import { t } from "../lib/i18n";
import { fmtSize } from "../lib/ui";

interface ZipEntry {
  name: string;
  size: number;
  compressed: number;
  dir: boolean;
}

const MAX_ENTRIES = 500;

export default function ZipPreview({
  path,
  project,
}: {
  path: string;
  project: string;
}) {
  const [entries, setEntries] = useState<ZipEntry[] | null>(null);
  const [failed, setFailed] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const [{ fileReadBase64 }, JSZip] = await Promise.all([
          import("../ipc/commands"),
          import("jszip"),
        ]);
        const fc = await fileReadBase64(project, path);
        const bin = atob(fc.content_base64);
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        const zip = await JSZip.loadAsync(bytes);
        if (!alive) return;
        const list: ZipEntry[] = [];
        zip.forEach((relPath, file) => {
          if (list.length >= MAX_ENTRIES) return;
          // Sizes live on the internal record in jszip 3.x (pinned);
          // absent metadata degrades to an em-dash in the table.
          const meta = file as unknown as {
            _data?: { uncompressedSize?: number; compressedSize?: number };
          };
          list.push({
            name: relPath,
            size: meta._data?.uncompressedSize ?? -1,
            compressed: meta._data?.compressedSize ?? -1,
            dir: file.dir,
          });
        });
        list.sort((a, b) => a.name.localeCompare(b.name));
        setEntries(list);
      } catch (e) {
        if (alive) setFailed(String(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [path, project]);

  if (failed) return <div className="empty-state">{failed}</div>;
  if (entries === null) return <div className="muted small">{t("misc.loading")}</div>;
  return (
    <div>
      <p className="muted small">{t("preview.zipNote")}</p>
      {entries.length === 0 ? (
        <p className="muted small">{t("preview.zipEmpty")}</p>
      ) : (
        <div style={{ overflow: "auto", maxHeight: 520 }}>
          <table className="tabular" style={{ borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr>
                <th className="muted small" style={{ textAlign: "left", padding: "3px 8px" }}>
                  {t("preview.zipName")}
                </th>
                <th className="muted small" style={{ textAlign: "right", padding: "3px 8px" }}>
                  {t("preview.zipSize")}
                </th>
                <th className="muted small" style={{ textAlign: "right", padding: "3px 8px" }}>
                  {t("preview.zipCompressed")}
                </th>
              </tr>
            </thead>
            <tbody>
              {entries.map((entry, i) => (
                <tr key={`${entry.name}:${i}`}>
                  <td className="mono small" style={{ padding: "3px 8px", whiteSpace: "pre" }}>
                    {entry.dir ? `${entry.name}/` : entry.name}
                  </td>
                  <td className="muted small" style={{ padding: "3px 8px", textAlign: "right" }}>
                    {entry.dir || entry.size < 0 ? "—" : fmtSize(entry.size)}
                  </td>
                  <td className="muted small" style={{ padding: "3px 8px", textAlign: "right" }}>
                    {entry.dir || entry.compressed < 0 ? "—" : fmtSize(entry.compressed)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {entries.length >= MAX_ENTRIES && (
            <p className="muted small">{t("preview.zipTruncated")}</p>
          )}
        </div>
      )}
    </div>
  );
}
