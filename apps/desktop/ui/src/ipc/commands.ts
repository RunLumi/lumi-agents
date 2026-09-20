/* Typed wrappers for every Tauri command the UI consumes. The command
   list is cross-checked against src-tauri/src/lib.rs generate_handler!
   by src/tests/commandSurface.test.ts — renaming or removing a command
   breaks the build/CI on both sides. */

import { getTransport } from "./transport.ts";
import type {
  ChangeSet,
  CommitInfo,
  FileContent,
  GitStatus,
  KillSwitchDto,
  ListedEntry,
  MemoryRecord,
  OpenedProject,
  OperationsSnapshot,
  ProjectOverview,
  ProjectSummary,
  SearchHit,
  Task,
  ValidationRecord,
} from "./types.ts";

// provider session (in-memory only; the key never round-trips)
export interface ProviderConfigDto {
  configured: boolean;
  family?: string | null;
  endpoint?: string | null;
  model?: string | null;
}
export const providerGetConfig = () =>
  getTransport().invoke<ProviderConfigDto>("provider_get_config");
export const providerSetConfig = (config: {
  family: string; endpoint: string; model: string; apiKey: string;
}) => getTransport().invoke<ProviderConfigDto>("provider_set_config", config);
export const providerClearConfig = () =>
  getTransport().invoke<ProviderConfigDto>("provider_clear_config");

// task execution
export const taskRun = (taskId: string) =>
  getTransport().invoke<{ started: boolean; task_id: string }>("task_run", { taskId });

// operations
export const getKillSwitch = () => getTransport().invoke<KillSwitchDto>("get_kill_switch");
export const emergencyStop = () => getTransport().invoke<KillSwitchDto>("emergency_stop");
export const getOperationsSnapshot = () =>
  getTransport().invoke<OperationsSnapshot>("get_operations_snapshot");

// projects
export const projectOpenFolder = (path: string, displayName?: string) =>
  getTransport().invoke<OpenedProject>("project_open_folder", { path, displayName });
export const projectListRecent = () =>
  getTransport().invoke<ProjectSummary[]>("project_list_recent");
export const projectOverview = (projectId: string) =>
  getTransport().invoke<ProjectOverview>("project_overview", { projectId });
export const projectRelink = (projectId: string, path: string) =>
  getTransport().invoke<ProjectSummary>("project_relink", { projectId, path });
export const projectRemove = (projectId: string) =>
  getTransport().invoke<void>("project_remove", { projectId });

// files
export const fileList = (projectId: string, path: string) =>
  getTransport().invoke<ListedEntry[]>("file_list", { projectId, path });
export const fileRead = (projectId: string, path: string) =>
  getTransport().invoke<FileContent>("file_read", { projectId, path });
export const fileCreate = (projectId: string, path: string, content: string) =>
  getTransport().invoke<void>("file_create", { projectId, path, content });
export const fileEdit = (
  projectId: string,
  path: string,
  expectedSha256: string,
  content: string,
) => getTransport().invoke<void>("file_edit", { projectId, path, expectedSha256, content });
export const fileDelete = (projectId: string, path: string, expectedSha256?: string) =>
  getTransport().invoke<void>("file_delete", { projectId, path, expectedSha256 });
export const fileSearch = (projectId: string, query: string, mode: string) =>
  getTransport().invoke<SearchHit[]>("file_search", { projectId, query, mode });

// git
export const gitStatus = (projectId: string) =>
  getTransport().invoke<GitStatus | null>("git_status", { projectId });
export const gitLog = (projectId: string, limit: number) =>
  getTransport().invoke<CommitInfo[]>("git_log", { projectId, limit });
export const gitBranches = (projectId: string) =>
  getTransport().invoke<string[]>("git_branches", { projectId });
export const gitCreateBranch = (projectId: string, name: string) =>
  getTransport().invoke<void>("git_create_branch", { projectId, name });
export const gitSwitch = (projectId: string, name: string) =>
  getTransport().invoke<void>("git_switch", { projectId, name });
export const gitCommit = (projectId: string, message: string, paths: string[]) =>
  getTransport().invoke<string>("git_commit", { projectId, message, paths });

// tasks + validation + changes
export const taskCreate = (projectId: string, goal: string) =>
  getTransport().invoke<Task>("task_create", { projectId, goal });
export const taskList = (projectId: string) =>
  getTransport().invoke<Task[]>("task_list", { projectId });
export const validationRun = (projectId: string, taskId: string, command: string) =>
  getTransport().invoke<ValidationRecord>("validation_run", { projectId, taskId, command });
export const changeSet = (projectId: string, taskId: string) =>
  getTransport().invoke<ChangeSet>("change_set", { projectId, taskId });
export const changeSets = (projectId: string) =>
  getTransport().invoke<ChangeSet[]>("change_sets", { projectId });

// project connections (Spec 29: metadata here, credential in broker)
export interface ConnectionRecord {
  connection_id: string;
  project_id: string;
  name: string;
  kind: string;
  endpoint: string;
  credential_ref: string;
  created_at: string;
}
export const connectionsList = (projectId: string) =>
  getTransport().invoke<ConnectionRecord[]>("connections_list", { projectId });
export const connectionsConnect = (
  projectId: string,
  input: { name: string; kind: string; endpoint: string; credential: string },
) => getTransport().invoke<ConnectionRecord>("connections_connect", { projectId, ...input });
export const connectionsDisconnect = (projectId: string, connectionId: string) =>
  getTransport().invoke<boolean>("connections_disconnect", { projectId, connectionId });
// Checks the broker still holds the credential; never returns its value.
export const connectionsVerify = (projectId: string, connectionId: string) =>
  getTransport().invoke<boolean>("connections_verify", { projectId, connectionId });

// binary gated saves (Spec 30: Office artifacts traverse the same
// gate as text saves; the payload arrives base64-encoded)
export const fileCreateBase64 = (projectId: string, path: string, contentBase64: string) =>
  getTransport().invoke<void>("file_create_base64", { projectId, path, contentBase64 });
export const fileEditBase64 = (
  projectId: string,
  path: string,
  expectedSha256: string,
  contentBase64: string,
) =>
  getTransport().invoke<void>("file_edit_base64", {
    projectId,
    path,
    expectedSha256,
    contentBase64,
  });

// project automations (Spec 27: schedules materialize durable tasks)
export interface AutomationRecord {
  automation_id: string;
  schedule: { cron_expression: string; timezone: string; enabled: boolean };
  project_id: string;
  workspace_root: string;
  goal: string;
  enabled: boolean;
}
export const automationsList = (projectId: string) =>
  getTransport().invoke<AutomationRecord[]>("automations_list", { projectId });
export const automationsCreate = (projectId: string, goal: string, cron: string) =>
  getTransport().invoke<AutomationRecord>("automations_create", { projectId, goal, cron });
export const automationsSetEnabled = (
  projectId: string,
  automationId: string,
  enabled: boolean,
) =>
  getTransport().invoke<AutomationRecord>("automations_set_enabled", {
    projectId,
    automationId,
    enabled,
  });
export const automationsDelete = (projectId: string, automationId: string) =>
  getTransport().invoke<boolean>("automations_delete", { projectId, automationId });
export interface TickResult {
  fired: string[];
  refused: string[];
}
export const automationsTick = (localOffsetSeconds: number) =>
  getTransport().invoke<TickResult>("automations_tick", { localOffsetSeconds });

// preview (Spec 30 phase A: binary formats via bounded base64)
export interface FileContentBase64 {
  path: string;
  content_base64: string;
  sha256: string;
}
export const fileReadBase64 = (projectId: string, path: string) =>
  getTransport().invoke<FileContentBase64>("file_read_base64", { projectId, path });

// evidence (audit ledger read model)
export interface EvidenceSummaryEntry {
  action_id: string;
  operation: string;
  trust_label: string;
  verification?: string | null;
  target?: string | null;
}
export interface EvidenceDto {
  running: boolean;
  entries: EvidenceSummaryEntry[];
}
export const projectEvidence = (projectId: string) =>
  getTransport().invoke<EvidenceDto>("project_evidence", { projectId });

// artifacts + memory
export const artifactsList = (projectId: string) =>
  getTransport().invoke<import("./types").ArtifactEntry[]>("artifacts_list", { projectId });
export const memoryRemember = (
  projectId: string,
  submission: {
    memory_id: string;
    kind: string;
    content: string;
    task_id: string;
    command?: string;
    evidence?: string;
  },
) => getTransport().invoke<void>("memory_remember", { projectId, submission });
export const memoryList = (projectId: string) =>
  getTransport().invoke<[MemoryRecord, boolean][]>("memory_list", { projectId });
export const memoryInvalidate = (projectId: string, memoryId: string, reason: string) =>
  getTransport().invoke<void>("memory_invalidate", { projectId, memoryId, reason });

/* Canonical command names — must match generate_handler! in
   src-tauri/src/lib.rs exactly (drift test: src/tests/commandSurface.test.ts). */
export const COMMAND_NAMES = [
  "get_kill_switch",
  "emergency_stop",
  "get_operations_snapshot",
  "project_open_folder",
  "project_list_recent",
  "project_overview",
  "project_relink",
  "project_remove",
  "file_list",
  "file_read",
  "file_create",
  "file_edit",
  "file_delete",
  "file_search",
  "git_status",
  "git_log",
  "git_branches",
  "git_create_branch",
  "git_switch",
  "git_commit",
  "task_create",
  "task_list",
  "validation_run",
  "change_set",
  "change_sets",
  "project_clone",
  "artifacts_list",
  "memory_remember",
  "memory_list",
  "memory_invalidate",
  "provider_get_config",
  "provider_set_config",
  "provider_clear_config",
  "task_run",
  "project_evidence",
  "file_read_base64",
  "connections_list",
  "connections_connect",
  "connections_disconnect",
  "connections_verify",
  "file_create_base64",
  "file_edit_base64",
  "automations_list",
  "automations_create",
  "automations_set_enabled",
  "automations_delete",
  "automations_tick",
] as const;
