/* Pure helpers for XLSX editing (Spec 30.9): edit-key naming and the
   text→cell-value conversion applied before a gated save. No React,
   no ExcelJS — unit-testable in isolation. Formula cells stay
   read-only in the view; only literal cells ever reach a save. */

export function editKey(sheet: number, row: number, col: number): string {
  return `${sheet}:${row}:${col}`;
}

export function parseEditKey(key: string): { sheet: number; row: number; col: number } {
  const parts = key.split(":");
  return {
    sheet: Number(parts[0]),
    row: Number(parts[1]),
    col: Number(parts[2]),
  };
}

/** Converts edited cell text into an ExcelJS cell value. A cell that
    held a number keeps its type when the new text parses as one; a
    number that no longer parses becomes an honest string (never NaN). */
export function cellValueFromText(text: string, wasNumeric: boolean): string | number {
  if (!wasNumeric) return text;
  const trimmed = text.trim();
  if (trimmed === "") return "";
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : trimmed;
}
