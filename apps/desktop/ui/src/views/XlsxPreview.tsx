/* XLSX data view + basic edit (Spec 30.9): ExcelJS browser I/O behind
   a lazy import. Sheet tabs, row/column addresses, literal values.
   Formula cells show the expression and the CACHED result labeled
   "Last saved result" — formula cells are read-only here and never
   calculated. Literal cells are editable; saving serializes the whole
   workbook and traverses the same gated save as every other UI write
   (USER Task/Run + ActionProposal + checksum postconditions, §30.12).
   Hidden sheets are listed with a marker; empty trailing cells trimmed. */
import { useEffect, useRef, useState } from "react";
import { t } from "../lib/i18n";
import { toast } from "../lib/ui";
import { boundedRows, cellText } from "../lib/xlsxGrid";
import { cellValueFromText, editKey, parseEditKey } from "../lib/xlsxEdit";
import { fileEditBase64 } from "../ipc/commands";
import { Button } from "@/components/ui/button";
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyCell = any;
const MAX_COLS = 50;

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

export default function XlsxPreview({
  path,
  project,
}: {
  path: string;
  project: string;
}) {
  const [sheetNames, setSheetNames] = useState<string[]>([]);
  const [active, setActive] = useState(0);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const workbookRef = useRef<any>(null);
  const shaRef = useRef<string>("");
  const [rows, setRows] = useState<AnyCell[][] | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  // Pending edits keyed "sheet:row:col" → edited text. Row/col are
  // 1-based ExcelJS addresses.
  const [edits, setEdits] = useState<Map<string, string>>(new Map());
  const [saving, setSaving] = useState(false);
  const [reload, setReload] = useState(0);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const [{ fileReadBase64 }, ExcelJS] = await Promise.all([
          import("../ipc/commands"),
          import("exceljs"),
        ]);
        const fc = await fileReadBase64(project, path);
        if (!alive) return;
        shaRef.current = fc.sha256;
        const bin = atob(fc.content_base64);
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        const wb = new (ExcelJS as unknown as {
          Workbook: new () => import("exceljs").Workbook;
        }).Workbook();
        await wb.xlsx.load(
          bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength),
        );
        if (!alive) return;
        workbookRef.current = wb;
        setSheetNames(wb.worksheets.map((ws: { name: string }) => ws.name));
        setActive(0);
        setRows(boundedRows(wb.worksheets[0]));
      } catch (e) {
        if (alive) setFailed(String(e));
      }
    })();
    return () => {
      alive = false;
      workbookRef.current = null;
    };
  }, [path, project, reload]);

  const pickSheet = (index: number) => {
    setActive(index);
    const wb = workbookRef.current;
    const sheet: AnyCell | undefined = wb?.worksheets?.[index];
    if (sheet) setRows(boundedRows(sheet));
  };

  const recordEdit = (ri: number, ci: number, original: string, text: string) => {
    const key = editKey(active, ri + 1, ci + 1);
    setEdits((prev) => {
      const next = new Map(prev);
      if (text === original) {
        next.delete(key);
      } else {
        next.set(key, text);
      }
      return next;
    });
  };

  const saveEdits = async () => {
    const wb = workbookRef.current;
    if (!wb || edits.size === 0 || saving) return;
    setSaving(true);
    try {
      for (const [key, text] of edits) {
        const { sheet, row, col } = parseEditKey(key);
        const ws = wb.worksheets[sheet];
        if (!ws) throw new Error(`sheet ${sheet} disappeared`);
        const cell = ws.getRow(row).getCell(col);
        cell.value = cellValueFromText(text, typeof cell.value === "number");
      }
      const buffer: ArrayBuffer = await wb.xlsx.writeBuffer();
      const base64 = bytesToBase64(new Uint8Array(buffer));
      await fileEditBase64(project, path, shaRef.current, base64);
      setEdits(new Map());
      setReload((k) => k + 1);
      toast(t("preview.xlsxSaved"), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  if (failed) return <div className="empty-state">{failed}</div>;

  return (
    <div>
      {sheetNames.length > 0 && (
        <div className="inline-form">
          {sheetNames.map((name, i) => (
            <button
              key={name}
              className={`tab${active === i ? " active" : ""}`}
              onClick={() => pickSheet(i)}
            >
              {name}
            </button>
          ))}
        </div>
      )}
      <p className="muted small">{t("preview.xlsxEditHint")}</p>
      {rows === null ? (
        <div className="muted small">{t("misc.loading")}</div>
      ) : (
        <div style={{ overflow: "auto", maxHeight: 520 }}>
          <table className="tabular" style={{ borderCollapse: "collapse", fontSize: 12 }}>
            <tbody>
              {rows.map((row, ri) => (
                <tr key={ri}>
                  <td
                    className="muted small"
                    style={{
                      border: "1px solid var(--color-border-subtle)",
                      padding: "3px 8px",
                      background: "var(--color-surface-white)",
                    }}
                  >
                    {ri + 1}
                  </td>
                  {Array.from({ length: MAX_COLS }, (_, ci) => {
                    const parsed = cellText(row[ci + 1]);
                    if (parsed.formula) {
                      return (
                        <td
                          key={ci}
                          title={t("preview.xlsxFormula")}
                          style={{
                            border: "1px solid var(--color-border-subtle)",
                            padding: "3px 8px",
                            whiteSpace: "pre",
                            fontStyle: "italic",
                            color: "var(--color-soft-slate)",
                          }}
                        >
                          {parsed.text}
                        </td>
                      );
                    }
                    return (
                      <td
                        key={ci}
                        style={{
                          border: "1px solid var(--color-border-subtle)",
                          padding: 0,
                        }}
                      >
                        <input
                          className="xlsx-cell"
                          value={
                            edits.get(editKey(active, ri + 1, ci + 1)) ?? parsed.text
                          }
                          onChange={(e) => recordEdit(ri, ci, parsed.text, e.target.value)}
                          aria-label={`R${ri + 1}C${ci + 1}`}
                        />
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <div className="inline-form" style={{ marginTop: 8 }}>
        <Button size="sm" onClick={saveEdits} disabled={edits.size === 0 || saving}>
          {saving ? t("preview.xlsxSaving") : t("preview.xlsxSave")}
        </Button>
        {edits.size > 0 && (
          <span className="muted small">
            {edits.size === 1 ? t("preview.xlsxChange") : t("preview.xlsxChanges")}
          </span>
        )}
      </div>
    </div>
  );
}
