import { GLYPHS, UTILITY_GLYPHS, PRODUCT_GLYPHS } from "./data.ts";

/** Lumi product concepts — resolvable as product glyphs. */
export type ProductGlyphName =
  | "project"
  | "task"
  | "agent"
  | "evidence"
  | "approval"
  | "artifact"
  | "verification"
  | "recovery"
  | "exception"
  | "authority"
  | "guardrail"
  | "workflow";

export type GlyphName = keyof typeof GLYPHS | ProductGlyphName;

const PRODUCT_ALIASES: Record<ProductGlyphName, string> = {
  project: "project",
  task: "task",
  agent: "agent",
  evidence: "evidence",
  approval: "approval",
  artifact: "artifact",
  verification: "verification",
  recovery: "recovery",
  exception: "exception",
  authority: "authority",
  guardrail: "guardrail",
  workflow: "workflow",
};

const FALLBACK = UTILITY_GLYPHS.file;

/**
 * Lumi Glyph System — utility + product glyphs (ICON.md).
 * 20×20 grid, 1.75 authored stroke, currentColor, aria-hidden by default:
 * accessible labels belong at the call site (ICON.md §8).
 */
export function Glyph({
  name,
  class: className,
  label,
}: {
  name: GlyphName;
  class?: string;
  label?: string;
}) {
  const key = name in PRODUCT_ALIASES ? name : (name in GLYPHS ? name : "file");
  const body = PRODUCT_ALIASES[key as ProductGlyphName]
    ? (PRODUCT_GLYPHS[key as ProductGlyphName] ?? GLYPHS[key] ?? FALLBACK)
    : (GLYPHS[key] ?? FALLBACK);
  return (
    <svg
      className={className}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden={label ? undefined : true}
      role={label ? "img" : undefined}
      dangerouslySetInnerHTML={{ __html: body }}
    />
  );
}
