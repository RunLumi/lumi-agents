/* Lumi Glyph System — React port (ICON.md §7). Path data verbatim from
   glyphs/data.ts; 20×20 grid, 2.5 authored stroke = 2px at 16px. */
import { GLYPHS } from "./glyphs/data";

export type GlyphName = keyof typeof GLYPHS;

export function Glyph({ name, className = "" }: { name: string; className?: string }) {
  const body = GLYPHS[name] ?? GLYPHS.file;
  return (
    <svg
      className={className}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: body }}
    />
  );
}

export function Mark({ name, className = "" }: { name: "clarity" | "annotationDot" | "lCorner" | "diagonal"; className?: string }) {
  const body =
    name === "clarity"
      ? '<path d="M10 2.25 11.8 8.2 17.75 10 11.8 11.8 10 17.75 8.2 11.8 2.25 10l5.95-1.8z"/>'
      : name === "lCorner"
        ? '<path d="M3.75 3.75v12.5h12.5"/>'
        : name === "diagonal"
          ? '<path d="M3.25 16.75 16.75 3.25"/>'
          : '<circle cx="10" cy="10" r="3.5" fill="currentColor" stroke="none"/>';
  return (
    <svg
      className={className}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: body }}
    />
  );
}
