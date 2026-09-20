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

/* In-memory fixture filesystem for the Files tab and palette search.
   sha256 is a djb2 stand-in: the devmock never leaves the browser. */
const fixtureFiles = new Map<string, string>([
  ["README.md", "# PrintUp\n\nReconciliation workspace for the Q3 ledger close.\n"],
  ["docs/runbook.md", "# Runbook\n\n1. Pull the CRM export.\n2. Reconcile totals.\n3. File the summary.\n"],
  ["src/index.ts", "import { reconcile } from \"./invoice/reconcile\";\n\nconsole.log(\"printup ready\");\n"],
  ["src/invoice/reconcile.ts", "export function reconcile(rows: string[]): number {\n  return rows.length * 42;\n}\n"],
  ["ledger.csv", "month,total\nJuly,1200\nAugust,1350\nSeptember,1480\n"],
]);

const sha = (content: string): string => {
  let h = 5381;
  for (let i = 0; i < content.length; i++) h = ((h << 5) + h + content.charCodeAt(i)) >>> 0;
  return `dm${h.toString(16).padStart(8, "0")}`;
};

function listDir(path: string): Array<Record<string, unknown>> {
  const prefix = path === "." ? "" : `${path}/`;
  const dirs = new Set<string>();
  const files: Array<Record<string, unknown>> = [];
  for (const full of fixtureFiles.keys()) {
    if (!full.startsWith(prefix)) continue;
    const rest = full.slice(prefix.length);
    const slash = rest.indexOf("/");
    if (slash === -1) {
      files.push({ name: rest, path: full, is_dir: false, size: fixtureFiles.get(full)!.length });
    } else {
      dirs.add(rest.slice(0, slash));
    }
  }
  return [
    ...[...dirs].map((d) => ({ name: d, path: prefix ? `${prefix}${d}` : d, is_dir: true })),
    ...files,
  ];
}

function searchFixture(query: string, mode: string): Array<Record<string, unknown>> {
  const q = query.toLowerCase();
  const hits: Array<Record<string, unknown>> = [];
  for (const [path, content] of fixtureFiles) {
    if (mode !== "text") {
      const name = path.split("/").pop()!;
      if (name.toLowerCase().includes(q)) hits.push({ path });
      continue;
    }
    content.split("\n").forEach((line, i) => {
      if (line.toLowerCase().includes(q)) hits.push({ path, line: i + 1, snippet: line.trim().slice(0, 90) });
    });
  }
  return hits.slice(0, 200);
}

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
        case "file_list":
          return listDir((args?.path as string) ?? ".") as T;
        case "file_read": {
          const path = args?.path as string;
          const content = fixtureFiles.get(path);
          if (content === undefined) throw new Error("devmock: file not found");
          return { path, content, sha256: sha(content) } as T;
        }
        case "file_create": {
          const path = args?.path as string;
          if (fixtureFiles.has(path)) throw new Error("devmock: file exists");
          fixtureFiles.set(path, (args?.content as string) ?? "");
          return null as T;
        }
        case "file_edit": {
          const path = args?.path as string;
          const current = fixtureFiles.get(path);
          if (current === undefined) throw new Error("devmock: file not found");
          if (sha(current) !== args?.expectedSha256) throw new Error("devmock: stale write refused");
          fixtureFiles.set(path, (args?.content as string) ?? "");
          return null as T;
        }
        case "file_delete": {
          const path = args?.path as string;
          const current = fixtureFiles.get(path);
          if (current === undefined) throw new Error("devmock: file not found");
          if (args?.expectedSha256 && sha(current) !== args.expectedSha256) throw new Error("devmock: stale delete refused");
          fixtureFiles.delete(path);
          return null as T;
        }
        case "file_search":
          return searchFixture(args?.query as string, args?.mode as string) as T;
        case "provider_get_config":
          return { configured: true, family: "openai-compatible", endpoint: "http://localhost:11434/v1", model: "llama3.1" } as T;
        case "provider_set_config":
          return { configured: true, family: "openai-compatible", endpoint: args?.endpoint as string, model: args?.model as string } as T;
        case "provider_clear_config":
          return { configured: false } as T;
        case "task_run": {
          const id = args?.taskId as string;
          const task = tasks.find((t) => t.task_id === id);
          if (task) task.status = "RUNNING";
          setTimeout(() => {
            const t = tasks.find((x) => x.task_id === id);
            if (t) t.status = "COMPLETED";
          }, 4000);
          return { started: true, task_id: id } as T;
        }
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
