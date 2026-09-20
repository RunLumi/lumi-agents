/* Pure helpers for the XLSX data view (Spec 30.9): cell value mapping
   and bounded row extraction. No React, no DOM — unit-testable in
   isolation against real ExcelJS workbooks. Formula cells are reported
   as { formula: true, cached } with their raw cached result; the
   component layer adds the "Last saved result" / "Not calculated"
   labels (formula cells are read-only and never calculated here). */

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type AnyCell = any;

export const MAX_ROWS = 500;
export const MAX_COLS = 50;

export function cellText(cell: AnyCell): { text: string; formula: boolean; cached: boolean } {
  const value = cell?.value;
  if (value == null) return { text: "", formula: false, cached: false };
  // Formula cells: value = { formula, result? } (shared formulas too).
  if (cell.type === 6 || (typeof value === "object" && value.formula != null)) {
    return {
      text: value.result != null ? String(value.result) : "",
      formula: true,
      cached: value.result != null,
    };
  }
  if (value instanceof Date) {
    return { text: value.toISOString().slice(0, 10), formula: false, cached: false };
  }
  if (typeof value === "object") {
    if (Array.isArray(value.richText)) {
      return {
        text: value.richText.map((rt: { text?: string }) => rt.text ?? "").join(""),
        formula: false,
        cached: false,
      };
    }
    if (value.error != null) return { text: String(value.error), formula: false, cached: false };
    if (value.text != null) return { text: String(value.text), formula: false, cached: false };
    if (value.hyperlink != null)
      return { text: String(value.hyperlink), formula: false, cached: false };
  }
  return { text: String(value), formula: false, cached: false };
}

export function boundedRows(worksheet: AnyCell): AnyCell[][] {
  // ExcelJS getSheetValues(): index 0 is an unused slot and sheet rows
  // start at 1 — drop it, then bound the grid.
  const values = (worksheet.getSheetValues() as unknown[][]).slice(1, 1 + MAX_ROWS);
  return values.map((row) => ((row ?? []) as unknown[]).slice(1, MAX_COLS + 1));
}
