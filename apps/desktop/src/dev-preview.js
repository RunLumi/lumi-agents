"use strict";
/* DEV PREVIEW fixture data. Loaded only via index.html?devmock .
   Never wired in the packaged app: no query parameter, no fixture. */
(function () {
  window.localStorage = {
    _s: {},
    getItem(k) { return this._s[k] != null ? this._s[k] : null; },
    setItem(k, v) { this._s[k] = String(v); },
    removeItem(k) { delete this._s[k]; },
  };
  window.navigator = window.navigator || {};
  const ago = (mins) => new Date(Date.now() - mins * 60000).toISOString();
  window.__TAURI__ = {
    core: {
      invoke: async (cmd) => {
        await new Promise((r) => setTimeout(r, 60));
        switch (cmd) {
          case "project_list_recent":
            return [{
              project_id: "demo-printup",
              display_name: "PrintUp",
              primary_root: "~/dev/printup",
              environment_id: "env-demo",
              detected_source: "Node",
              capabilities: ["project_open_folder", "files_read", "files_write", "git_read"],
              is_repository: true,
              health: "available",
              last_opened_at: ago(2),
            }];
          case "project_overview":
            return {
              project_id: "demo-printup",
              display_name: "PrintUp",
              primary_root: "~/dev/printup",
              environment_id: "env-demo",
              detected_source: "Node",
              capabilities: ["project_open_folder", "project_recent", "files_read", "files_write", "shell_host_bounded", "git_read"],
              instructions: ["README.md (readme)"],
              health: "available",
              discovery: {
                manifests: ["package.json"],
                lockfiles: ["package-lock.json"],
                proposed_commands: [
                  { role: "test", command: "npm test", evidence: "package.json" },
                  { role: "build", command: "npm run build", evidence: "package.json" },
                ],
                ci_configs: [".github/workflows"],
                entry_points: ["src/index.ts"],
                truncated: false,
              },
            };
          case "git_status":
            return {
              branch: "feature/order-sync", head: "a1f3c2e0000", is_clean: false,
              staged: [{ path: "src/orders/sync.ts", original_path: null, index_state: "M", worktree_state: null }],
              unstaged: [
                { path: "src/types.ts", original_path: null, index_state: null, worktree_state: "M" },
                { path: ".env.example", original_path: null, index_state: null, worktree_state: "M" },
              ],
              untracked: ["coverage-report.html"],
            };
          case "task_list":
            return [{
              task_id: "demo-task-1",
              tenant_id: "tenant-local",
              status: "RUNNING",
              goal: "Refactor order sync and make tests pass",
              created_at: ago(12),
              project_binding: { project_id: "demo-printup", workspace_kind: "PROJECT_ROOT", workspace_root: "~/dev/printup", execution_environment_id: "env-demo" },
            }];
          case "change_sets":
            return [{
              project_id: "demo-printup",
              task_id: "demo-task-1",
              updated_at: ago(2),
              entries: [
                { kind: "modified", source: "agent", path: "src/orders/sync.ts", sha256_before: "aaa", sha256_after: "bbb", recorded_at: ago(4), task_id: "demo-task-1" },
                { kind: "created", source: "agent", path: "tests/orders/sync.test.ts", recorded_at: ago(6), task_id: "demo-task-1" },
              ],
              commands: [],
              validations: [
                { role: "test", command: "npm test", status: "passed", exit_code: 0, duration_ms: 3200, recorded_at: ago(3) },
                { role: "lint", command: "npm run lint", status: "passed", exit_code: 0, duration_ms: 1100, recorded_at: ago(3) },
              ],
            }];
          case "git_log":
            return [
              { hash: "4f2c9ad000", short_hash: "4f2c9ad", author: "Lumi", subject: "Add order validation and error messages", timestamp: 1735689600 },
              { hash: "a8d3e11000", short_hash: "a8d3e11", author: "Lumi", subject: "Refactor sync to use new API client", timestamp: 1735603200 },
            ];
          case "git_branches":
            return ["main", "develop", "feature/order-sync"];
          case "file_list":
            return [
              { name: "src", path: "src", is_dir: true, size: null },
              { name: "tests", path: "tests", is_dir: true, size: null },
              { name: "package.json", path: "package.json", is_dir: false, size: 512 },
              { name: "README.md", path: "README.md", is_dir: false, size: 40 },
            ];
          case "file_read":
            return {
              path: "src/orders/sync.ts",
              content: "import { createClient } from '../config/api';\nimport { logger } from '../utils/logger';\n\n// Sync orders with the remote PrintUp service.\nexport async function syncOrders(options) {\n  const { since, maxRetries = 3 } = options;\n  return client.sync(since);\n}\n",
              sha256: "b".repeat(64),
            };
          case "artifacts_list":
            return [
              { name: "test-results-2026-01-14T10-24.zip", path: ".lumi/artifacts/test-results/results.zip", artifact_type: "test_report", lifecycle: "READY", sha256: "a3f9", size: 12_400_000, modified_at: Math.floor(Date.now() / 1000) - 120 },
              { name: "coverage-report.html", path: ".lumi/artifacts/coverage/report.html", artifact_type: "coverage", lifecycle: "READY", sha256: "7e1b", size: 3_100_000, modified_at: Math.floor(Date.now() / 1000) - 720 },
            ];
          case "get_operations_snapshot":
            return { connection: "connected", execution: "unavailable", queue: [], progress: null, pending_approvals: null, exceptions: null, evidence: null, economics: null, permissions: null };
          default:
            return null;
        }
      },
    },
    window: {
      getCurrent: () => ({
        close: () => {},
        minimize: () => {},
        toggleMaximize: () => {},
      }),
    },
    dialog: { open: async () => null },
    opener: { revealItemInDir: async () => {} },
  };
})();
