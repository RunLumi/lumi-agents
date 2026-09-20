/* Pure helpers for DOCX paragraph editing (Spec 30.8): diffing the
   edited paragraph list and bounding the editable surface. No React,
   no DOM — unit-testable in isolation. The DOM parse/serialize lives
   in the component (browser-only). */

/** Maximum body paragraphs exposed for editing; the rest stay
    preview-only (bounded surface, predictable save). */
export const MAX_EDIT_PARAGRAPHS = 400;

/** Indices where the edited text differs from the original, mapped to
    the new text. Exact string comparison — whitespace churn is a real
    change the user made. */
export function changedParagraphs(
  original: string[],
  current: string[],
): Map<number, string> {
  const changes = new Map<number, string>();
  const bound = Math.min(original.length, current.length);
  for (let i = 0; i < bound; i++) {
    if (original[i] !== current[i]) changes.set(i, current[i]);
  }
  return changes;
}

/** The editable slice of the paragraph list (bounded). */
export function boundParagraphs(paragraphs: string[]): string[] {
  return paragraphs.slice(0, MAX_EDIT_PARAGRAPHS);
}
