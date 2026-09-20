/* TypeScript mirrors of the Rust serde DTOs returned by the Tauri
   commands (crates/lumi-desktop). Field names match serde serialization
   exactly; Rust-side methods (is_clean, changed_count) are NOT serialized
   and must be computed client-side. Drift is caught by the command-surface
   test (src/tests/commandSurface.test.ts). */

export interface FileStatus {
  path: string;
  original_path?: string;
  index_state?: string;
  worktree_state?: string;
}

export interface GitStatus {
  branch?: string;
  head?: string;
  staged: FileStatus[];
  unstaged: FileStatus[];
  untracked: string[];
}

export interface CommitInfo {
  hash: string;
  short_hash: string;
  author: string;
  subject: string;
  timestamp: number;
}

export interface WorktreeEntry {
  path: string;
  head?: string;
  branch?: string;
}

export interface ListedEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number;
}

export interface FileContent {
  path: string;
  content: string;
  sha256: string;
}

export interface SearchHit {
  path: string;
  line?: number;
  snippet?: string;
}

export interface ValidationRecord {
  role: string;
  command: string;
  status: "passed" | "failed" | "skipped" | "unavailable" | "ambiguous";
  exit_code?: number;
  duration_ms: number;
  output_tail?: string;
  recorded_at: string;
}

export interface ChangeEntry {
  kind: "created" | "modified" | "moved" | "deleted" | "restored";
  source: "agent" | "external_conflict";
  path: string;
  from_path?: string;
  sha256_before?: string;
  sha256_after?: string;
  patch?: string;
  task_id: string;
  recorded_at: string;
}

export interface CommandRecord {
  purpose: string;
  command: string;
  exit_code?: number;
  duration_ms: number;
  recorded_at: string;
}

export interface ChangeSet {
  project_id: string;
  task_id: { task_id: string };
  entries: ChangeEntry[];
  updated_at: string;
  commands?: CommandRecord[];
  validations?: ValidationRecord[];
}

export interface ArtifactEntry {
  name: string;
  path: string;
  artifact_type?: string;
  lifecycle?: string;
  sha256?: string;
  size: number;
  modified_at: number;
}

export interface ProjectSummary {
  project_id: string;
  display_name: string;
  primary_root: string;
  environment_id: string;
  detected_source: string;
  capabilities: string[];
  is_repository: boolean;
  health: "available" | "missing" | "moved";
  last_opened_at: string;
}

export interface OpenedProject {
  created: boolean;
  project: ProjectSummary;
}

export interface ProjectDiscovery {
  manifests: string[];
  lockfiles: string[];
  proposed_commands: { role: string; command: string; evidence: string }[];
  ci_configs: string[];
  entry_points: string[];
  truncated: boolean;
}

export interface ProjectOverview {
  project_id: string;
  display_name: string;
  primary_root: string;
  environment_id: string;
  detected_source: string;
  capabilities: string[];
  instructions: string[];
  health: "available" | "missing" | "moved";
  git?: GitStatus;
  discovery: ProjectDiscovery;
}

export interface Task {
  task_id: string;
  tenant_id: string;
  goal: string;
  created_at: string;
  status: string;
  project_binding?: {
    project_id: string;
    execution_environment_id: string;
    workspace_root: string;
    workspace_kind: string;
  };
}

export interface MemoryRecord {
  memory_id: string;
  kind: string;
  content: string;
  provenance: { task_id: string; command?: string; evidence?: string };
  validated_at_head?: string;
  created_at: string;
  invalidated?: { reason: string; at: string } | null;
}

/* Kill switch / operations snapshot (spec 15 surface) */
export type KillSwitchState = "running" | "stopped";

export interface KillSwitchDto {
  state: KillSwitchState;
  running: boolean;
}

export type ConnectionState = "connected" | "disconnected" | "unknown";
export type ExecutionState = "enabled" | "stopped" | "unavailable";

export interface OperationsSnapshot {
  connection: ConnectionState;
  execution: ExecutionState;
  queue: unknown[] | null;
  progress: unknown;
  pending_approvals: unknown[] | null;
  exceptions: unknown[] | null;
  evidence: unknown[] | null;
  economics: unknown;
  permissions: unknown;
}
