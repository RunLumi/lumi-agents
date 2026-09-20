/* Shared UI helpers. */
import { t } from "./i18n";

export function timeAgo(rfc3339: string | undefined): string {
  if (!rfc3339) return "";
  const then = new Date(rfc3339).getTime();
  if (Number.isNaN(then)) return "";
  const mins = Math.max(0, Math.round((Date.now() - then) / 60000));
  if (mins < 1) return t("misc.justNow");
  if (mins < 60) return `${mins} ${t("misc.minAgo")}`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `${hours} ${t(hours === 1 ? "misc.hourAgo" : "misc.hoursAgo")}`;
  const days = Math.round(hours / 24);
  return `${days} ${t(days === 1 ? "misc.dayAgo" : "misc.daysAgo")}`;
}

export function fmtSize(bytes?: number): string {
  if (bytes == null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

export function langOf(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    ts: "TypeScript", tsx: "TypeScript", js: "JavaScript", rs: "Rust",
    py: "Python", go: "Go", json: "JSON", md: "Markdown", toml: "TOML",
    yml: "YAML", yaml: "YAML", html: "HTML", css: "CSS", sh: "Shell",
  };
  return map[ext] ?? "Text";
}

export function gitChanged(git: { staged: unknown[]; unstaged: unknown[]; untracked: unknown[] } | null | undefined): number {
  if (!git) return 0;
  return git.staged.length + git.unstaged.length + git.untracked.length;
}

export function gitIsClean(git: { staged: unknown[]; unstaged: unknown[]; untracked: unknown[] } | null | undefined): boolean {
  return !!git && gitChanged(git) === 0;
}

export function toast(message: string, kind: "" | "error" | "success" = ""): void {
  const el = document.createElement("div");
  el.className = `toast ${kind}`;
  el.textContent = message;
  document.getElementById("toasts")?.appendChild(el);
  setTimeout(() => el.remove(), 4200);
}

export function setRoute(hash: string): void {
  window.location.hash = hash;
}
