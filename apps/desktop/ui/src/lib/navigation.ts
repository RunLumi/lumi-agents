export type NavKey =
  | "projects" | "tasks" | "files" | "changes" | "git"
  | "artifacts" | "evidence" | "approvals";

const PROJECT_NAV_KEYS = new Set<NavKey>([
  "tasks", "files", "changes", "git", "artifacts", "evidence", "approvals",
]);

/** Maps the visible Project tab to the matching sidebar destination. */
export function activeNavFor(projectId: string | null, detailTab: string): NavKey {
  if (!projectId || detailTab === "home") return "projects";
  return PROJECT_NAV_KEYS.has(detailTab as NavKey) ? detailTab as NavKey : "projects";
}
