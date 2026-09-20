import { MARKS } from "./data.ts";

export type MarkName = "clarity" | "annotationDot" | "lCorner" | "diagonal";

/**
 * Semantic modifiers (ICON.md §2.C): state attaches to an object glyph,
 * never a new icon. Render next to a <Glyph /> to annotate it.
 */
export function Mark({ name, class: className }: { name: MarkName; class?: string }) {
  const body = MARKS[name] ?? MARKS.annotationDot;
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
