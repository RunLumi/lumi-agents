/* Devmock transport (goal §8): implements the same command surface as
   commands.ts with an in-memory fixture project. Loaded ONLY via
   ?devmock (see main.tsx); the packaged app never contains that query
   parameter, so this module is unreachable in production. It exists so
   the UI can be developed and visually audited in a plain browser. */
import { setMockTransport, type Transport } from "./transport.ts";

export const MOCK_PROJECT_ID = "devmock-printup";

const ago = (mins: number) => new Date(Date.now() - mins * 60000).toISOString();

const tasks: Array<Record<string, unknown>> = [
  {
    task_id: "task-dm-1",
    tenant_id: "tenant-devmock",
    goal: "Reconcile Q3 invoice totals against the CRM export",
    created_at: ago(180),
    status: "COMPLETED",
    project_binding: {
      project_id: MOCK_PROJECT_ID,
      execution_environment_id: "env-devmock",
      workspace_root: "~/dev/printup",
      workspace_kind: "ProjectRoot",
    },
  },
  {
    task_id: "task-dm-2",
    tenant_id: "tenant-devmock",
    goal: "Draft the September revenue summary from ledger.csv",
    created_at: ago(45),
    status: "RUNNING",
    project_binding: {
      project_id: MOCK_PROJECT_ID,
      execution_environment_id: "env-devmock",
      workspace_root: "~/dev/printup",
      workspace_kind: "ProjectRoot",
    },
  },
  {
    task_id: "task-dm-3",
    tenant_id: "tenant-devmock",
    goal: "Send the renewal reminder to Northwind (needs approval)",
    created_at: ago(12),
    status: "WAITING_APPROVAL",
    project_binding: {
      project_id: MOCK_PROJECT_ID,
      execution_environment_id: "env-devmock",
      workspace_root: "~/dev/printup",
      workspace_kind: "ProjectRoot",
    },
  },
];

const overview = {
  project_id: MOCK_PROJECT_ID,
  display_name: "PrintUp",
  primary_root: "/Users/demo/dev/printup",
  environment_id: "env-devmock",
  detected_source: "Node workspace (package.json)",
  capabilities: ["files.read", "files.create", "files.edit", "git.read", "git.commit", "shell.execute"],
  instructions: ["AGENTS.md", ".lumi/instructions.md"],
  health: "available" as const,
  git: {
    branch: "main",
    head: "3f9c2ab",
    staged: [
      { path: "src/invoice/reconcile.ts", worktree_state: "A" },
    ],
    unstaged: [
      { path: "docs/runbook.md", worktree_state: "M" },
      { path: "ledger.csv", worktree_state: "M" },
    ],
    untracked: ["notes/september-draft.md"],
  },
  discovery: {
    manifests: ["package.json"],
    lockfiles: ["package-lock.json"],
    ci: [".github/workflows/ci.yml"],
    entry_points: ["src/index.ts"],
    proposed_commands: [{ role: "test", command: "npm test" }],
  },
};

const changeSets = [
  {
    project_id: MOCK_PROJECT_ID,
    task_id: { task_id: "task-dm-1" },
    updated_at: ago(90),
    entries: [
      {
        kind: "created",
        source: "agent",
        path: "src/invoice/reconcile.ts",
        task_id: "task-dm-1",
        recorded_at: ago(150),
        sha256_after: "9f2c…",
      },
      {
        kind: "modified",
        source: "agent",
        path: "ledger.csv",
        task_id: "task-dm-1",
        recorded_at: ago(140),
      },
      {
        kind: "modified",
        source: "external_conflict",
        path: "docs/runbook.md",
        task_id: "task-dm-1",
        recorded_at: ago(120),
      },
    ],
    validations: [
      {
        role: "test",
        command: "npm test",
        status: "passed",
        exit_code: 0,
        duration_ms: 4120,
        recorded_at: ago(95),
      },
      {
        role: "typecheck",
        command: "npx tsc --noEmit",
        status: "failed",
        exit_code: 2,
        duration_ms: 1810,
        output_tail: "src/invoice/reconcile.ts:42 — expected string, got number",
        recorded_at: ago(94),
      },
    ],
  },
];

const artifacts = [
  {
    name: "september-revenue-summary.md",
    path: ".lumi/artifacts/september-revenue-summary.md",
    artifact_type: "markdown",
    lifecycle: "draft",
    sha256: "b7e1d4…",
    size: 14820,
    modified_at: Date.now() - 30 * 60000,
  },
  {
    name: "q3-reconciliation.json",
    path: ".lumi/artifacts/q3-reconciliation.json",
    artifact_type: "json",
    lifecycle: "validated",
    sha256: "1ac90f…",
    size: 2043,
    modified_at: Date.now() - 88 * 60000,
  },
];

export function createMockTransport(): Transport {
  return {
    async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
      await new Promise((r) => setTimeout(r, 40));
      switch (command) {
        case "project_list_recent":
          return [
            {
              project_id: MOCK_PROJECT_ID,
              display_name: "PrintUp",
              primary_root: "/Users/demo/dev/printup",
              environment_id: "env-devmock",
              detected_source: "Node workspace (package.json)",
              is_repository: true,
              health: "available",
              last_opened_at: ago(2),
            },
          ] as T;
        case "project_overview":
          return overview as T;
        case "git_status":
          return (overview.git ?? null) as T;
        case "git_log":
          return [
            { hash: "3f9c2ab1", short_hash: "3f9c2ab", author: "Demo", subject: "Reconcile Q3 invoices", timestamp: Date.now() / 1000 - 5400 },
            { hash: "81ad7c40", short_hash: "81ad7c4", author: "Demo", subject: "Update September ledger", timestamp: Date.now() / 1000 - 86400 },
            { hash: "c04e9f12", short_hash: "c04e9f1", author: "Demo", subject: "Initial import", timestamp: Date.now() / 1000 - 7 * 86400 },
          ].slice(0, (args?.limit as number) ?? 8) as T;
        case "git_branches":
          return ["main", "feature/rename-sync"] as T;
        case "git_create_branch":
        case "git_switch":
        case "git_commit":
          return null as T;
        case "task_list":
          return tasks as T;
        case "task_create":
          tasks.unshift({
            task_id: `task-dm-${tasks.length + 1}`,
            tenant_id: "tenant-devmock",
            goal: args?.goal as string,
            created_at: new Date().toISOString(),
            status: "CREATED",
            project_binding: {
              project_id: MOCK_PROJECT_ID,
              execution_environment_id: "env-devmock",
              workspace_root: "~/dev/printup",
              workspace_kind: "ProjectRoot",
            },
          });
          return tasks[0] as T;
        case "change_sets":
          return changeSets as T;
        case "artifacts_list":
          return artifacts as T;
        case "get_operations_snapshot":
          return {
            connection: "connected",
            execution: "unavailable",
            queue: [],
            progress: null,
            pending_approvals: [{ action_id: "a-dm-1", operation: "send_customer_email" }],
            exceptions: null,
            evidence: null,
            economics: null,
            permissions: null,
          } as T;
        case "emergency_stop":
          return { stopped: true } as T;
        case "get_kill_switch":
          return { stopped: false } as T;
        default:
          throw new Error(`devmock: unimplemented command ${command}`);
      }
    },
  };
}

export function installMockTransport(): void {
  setMockTransport(createMockTransport());
}
