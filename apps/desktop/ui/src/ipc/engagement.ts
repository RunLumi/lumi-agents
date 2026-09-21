import { getTransport } from "./transport.ts";
import type { Task } from "./types.ts";
import type {
  Automation, AutomationInput, AutomationList, EngagementSnapshot, ToolId,
} from "../lib/engagement.ts";

export const engagementSnapshot = (projectId: string) =>
  getTransport().invoke<EngagementSnapshot>("engagement_snapshot", { projectId });
export const engagementCheckBrowser = (projectId: string) =>
  getTransport().invoke<EngagementSnapshot>("engagement_check_browser", { projectId });
export const engagementSetBrowserOrigins = (projectId: string, origins: string[]) =>
  getTransport().invoke<EngagementSnapshot>("engagement_set_browser_origins", { projectId, origins });
export const engagementCreateTask = (projectId: string, goal: string, tools: ToolId[], requestId: string) =>
  getTransport().invoke<Task>("engagement_create_task", {
    request: { project_id: projectId, goal, tools, request_id: requestId },
  });
export const automationList = (projectId: string) =>
  getTransport().invoke<AutomationList>("automation_list", { projectId });
export const automationPreview = (input: AutomationInput) =>
  getTransport().invoke<number[]>("automation_preview", { input });
export const automationSave = (projectId: string, input: AutomationInput) =>
  getTransport().invoke<Automation>("automation_save", { projectId, input });
export const automationToggle = (projectId: string, automationId: string, revision: number, enabled: boolean) =>
  getTransport().invoke<void>("automation_toggle", { projectId, automationId, revision, enabled });
export const automationDelete = (projectId: string, automationId: string, revision: number) =>
  getTransport().invoke<void>("automation_delete", { projectId, automationId, revision });
export const automationRunNow = (projectId: string, automationId: string, requestId: string) =>
  getTransport().invoke<{ task_id: string }>("automation_run_now", { projectId, automationId, requestId });
