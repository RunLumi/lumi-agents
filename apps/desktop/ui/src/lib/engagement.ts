/** Intent metadata, never a permission grant. Both IPC and runtime revalidate it. */
export const TOOL_IDS = ["files", "browser", "chrome", "computer", "shell"] as const;
export type ToolId = typeof TOOL_IDS[number];
export interface Capability {
  id: ToolId;
  state: "ready" | "setup_required" | "unavailable";
  reason: string;
}
export interface EngagementSnapshot {
  capabilities: Capability[];
  browser_origins: string[];
  provider_configured: boolean;
  stopped: boolean;
}
export interface ComposerDraft {
  version: 1;
  goal: string;
  tools: ToolId[];
  requestId: string;
  createdTaskId?: string;
}
export const MAX_GOAL_LENGTH = 65_536;
export const draftKey = (projectId: string) => `lumi.task-draft.v1:${encodeURIComponent(projectId)}`;
export const isToolId = (value: unknown): value is ToolId =>
  typeof value === "string" && (TOOL_IDS as readonly string[]).includes(value);

/** Code examples and email addresses are not capability selections. */
export function proseOnly(text: string): string {
  let fence: string | null = null;
  return text.split("\n").map((line) => {
    const match = /^\s*(`{3,}|~{3,})/.exec(line);
    if (match) {
      if (!fence) fence = match[1][0];
      else if (match[1][0] === fence) fence = null;
      return "";
    }
    return fence ? "" : line.replace(/`+[^`]*`+/g, " ");
  }).join("\n");
}
export function explicitTools(text: string): ToolId[] {
  const found = new Set<ToolId>();
  for (const match of proseOnly(text).matchAll(/(?:^|\s)@(files|browser|chrome|computer|shell)\b/gi)) {
    found.add(match[1].toLowerCase() as ToolId);
  }
  return [...found];
}
export function mentionAt(text: string, caret: number): { start: number; query: string } | null {
  const before = text.slice(0, caret);
  const line = before.slice(before.lastIndexOf("\n") + 1);
  if (!proseOnly(before).endsWith(line)) return null;
  const match = /(?:^|\s)@([a-z]*)$/i.exec(line);
  return match ? { start: caret - match[1].length - 1, query: match[1].toLowerCase() } : null;
}
export function requestedTools(goal: string, selected: readonly ToolId[]): ToolId[] {
  // Prose is data, not authorization. The picker writes structured tool
  // selections into the draft; backend validation remains authoritative.
  void goal;
  return [...new Set<ToolId>(["files", ...selected])];
}
export function shouldSubmit(event: {
  key: string; metaKey: boolean; ctrlKey: boolean; isComposing: boolean; pickerOpen?: boolean;
}): boolean {
  return event.key === "Enter" && (event.metaKey || event.ctrlKey)
    && !event.isComposing && !event.pickerOpen;
}
export function parseDraft(raw: string | null): ComposerDraft | null {
  if (!raw) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (!value || typeof value !== "object") return null;
    const d = value as Partial<ComposerDraft>;
    if (d.version !== 1 || typeof d.goal !== "string" || d.goal.length > MAX_GOAL_LENGTH
      || typeof d.requestId !== "string" || !d.requestId || !Array.isArray(d.tools)
      || !d.tools.every(isToolId)
      || (d.createdTaskId !== undefined && typeof d.createdTaskId !== "string")) return null;
    return { version: 1, goal: d.goal, tools: [...new Set(d.tools)], requestId: d.requestId,
      ...(d.createdTaskId ? { createdTaskId: d.createdTaskId } : {}) };
  } catch { return null; }
}

export type ScheduleSpec =
  | { kind: "once"; local_datetime: string }
  | { kind: "every"; minutes: number }
  | { kind: "daily"; time: string }
  | { kind: "weekly"; time: string; weekdays: number[] };
export interface AutomationInput {
  automation_id?: string;
  revision?: number;
  name: string;
  goal: string;
  tools: ToolId[];
  schedule: ScheduleSpec;
  timezone: string;
  catch_up: "run_once" | "skip";
  enabled: boolean;
  authorization_until: number | null;
}
export interface AutomationRun {
  occurrence_id: string;
  scheduled_at: number;
  task_id: string | null;
  status: string;
  note: string | null;
}
export interface Automation extends AutomationInput {
  automation_id: string;
  project_id: string;
  revision: number;
  next_run_at: number | null;
  runs: AutomationRun[];
}
export interface AutomationList {
  items: Automation[];
  provider_configured: boolean;
  stopped: boolean;
  scheduler_available: boolean;
}
export interface AutomationSeed { goal: string; tools: ToolId[]; nonce: number }
