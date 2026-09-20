/* XLSX data view (Spec 30.9): ExcelJS browser I/O behind a lazy import.
   Sheet tabs, row/column addresses, literal values; formula cells show
   the expression and the CACHED result labeled "Last saved result" —
   formula cells are read-only in v1 and never calculated here. Hidden
   sheets are listed with a marker; empty trailing cells are trimmed. */
import { useEffect, useRef, useState } from "react";
import { t } from "../lib/i18n";
import { boundedRows, cellText } from "../lib/xlsxGrid";
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyCell = any;
const MAX_COLS = 50;

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
  const [rows, setRows] = useState<AnyCell[][] | null>(null);
  const [failed, setFailed] = useState<string | null>(null);

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
        (window as unknown as Record<string, unknown>).__xlsxDebug = {
          effectRan: true,
          rowCount: wb.worksheets[0].rowCount,
          row2: JSON.stringify(wb.worksheets[0].getRow(2)?.values ?? null),
        };
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
  }, [path, project]);

  const pickSheet = (index: number) => {
    setActive(index);
    const wb = workbookRef.current;
    const sheet: AnyCell | undefined = wb?.worksheets?.[index];
    if (sheet) setRows(boundedRows(sheet));
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
                    return (
                      <td
                        key={ci}
                        title={
                          parsed.formula
                            ? `${t("preview.xlsxFormula")}`
                            : undefined
                        }
                        style={{
                          border: "1px solid var(--color-border-subtle)",
                          padding: "3px 8px",
                          whiteSpace: "pre",
                          fontStyle: parsed.formula ? "italic" : undefined,
                          color: parsed.formula
                            ? "var(--color-soft-slate)"
                            : undefined,
                        }}
                      >
                        {parsed.text}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
