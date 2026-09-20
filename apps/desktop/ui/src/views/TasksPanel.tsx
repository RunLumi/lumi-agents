/* Tasks panel — durable task list, provider setup, and the Run button
   that wires a task to the desktop's real planning loop. The provider
   credential is held in the shell's memory for this session only; the
   form says so and the backend never persists or returns it. */
import { useEffect, useRef, useState } from "react";
import { Glyph } from "../components/Icons";
import { t } from "../lib/i18n";
import { timeAgo, toast } from "../lib/ui";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { StatusBadge, type BadgeVariant } from "@/components/ui/badge";
import { Separator } from "@/components/ui/overlays";
import {
  providerGetConfig, providerSetConfig, taskList, taskRun,
} from "../ipc/commands";
import type { Task } from "../ipc/types";

const TASK_STATUS_KEY: Record<string, string> = {
  CREATED: "status.created", QUEUED: "status.queued", RUNNING: "status.running",
  WAITING_APPROVAL: "status.waitingApproval", WAITING_USER: "status.waitingUser",
  WAITING_EXTERNAL: "status.waitingExternal", PAUSED: "status.paused",
  COMPLETED: "status.completed", FAILED: "status.failed",
  AMBIGUOUS: "status.ambiguous", CANCELLED: "status.cancelled",
};

/** Domain status → §10.4 badge family (same mapping as ProjectDetail). */
function statusVariant(status: string): BadgeVariant {
  if (status === "COMPLETED") return "status-done";
  if (status === "FAILED") return "status-critical";
  if (status === "RUNNING") return "status-in-progress";
  if (status.startsWith("WAITING") || status === "AMBIGUOUS") return "status-overdue";
  return "status-open";
}

const RUNNABLE = new Set(["CREATED", "FAILED", "CANCELLED"]);

interface Props {
  projectId: string;
  tasks: Task[];
  onChanged: () => void;
}

export function TasksPanel({ projectId, tasks, onChanged }: Props) {
  const [configured, setConfigured] = useState<boolean | null>(null);
  const [endpoint, setEndpoint] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [runningTask, setRunningTask] = useState<string | null>(null);
  const pollRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    providerGetConfig().then((c) => setConfigured(c.configured)).catch(() => setConfigured(null));
  }, []);

  // While a run is live, poll task status so the transition to
  // COMPLETED/FAILED shows up without a manual refresh.
  useEffect(() => {
    if (!runningTask) return;
    const timer = window.setInterval(() => {
      taskList(projectId).then((tasks) => {
        onChanged();
        const live = tasks.find((task) => task.task_id === runningTask);
        if (live && live.status !== "RUNNING") {
          setRunningTask(null);
          window.clearInterval(timer);
        }
      }).catch(() => {});
    }, 2000);
    pollRef.current = timer;
    return () => window.clearInterval(timer);
  }, [runningTask, projectId, onChanged]);

  const saveProvider = () => {
    if (!endpoint.trim() || !model.trim() || !apiKey.trim()) {
      toast(t("provider.missingFields"), "error");
      return;
    }
    providerSetConfig({
      family: "openai-compatible",
      endpoint: endpoint.trim(),
      model: model.trim(),
      apiKey: apiKey.trim(),
    }).then((c) => {
      setConfigured(c.configured);
      setApiKey("");
      toast(t("provider.saved"), "success");
    }).catch((e) => toast(String(e), "error"));
  };

  const run = (task: Task) => {
    taskRun(task.task_id).then(() => {
      setRunningTask(task.task_id);
      onChanged();
    }).catch((e) => toast(String(e), "error"));
  };

  return (
    <div className="card">
      {configured === false && (
        <>
          <div className="inline-form">
            <Glyph name="insight" />
            <span className="small" style={{ flex: 1 }}>{t("provider.notConfigured")}</span>
          </div>
          <div className="inline-form">
            <Input value={endpoint} onChange={(e) => setEndpoint(e.target.value)} placeholder={t("provider.endpoint")} />
            <Input value={model} onChange={(e) => setModel(e.target.value)} placeholder={t("provider.model")} />
            <Input value={apiKey} onChange={(e) => setApiKey(e.target.value)} placeholder={t("provider.apiKey")} type="password" />
            <Button variant="secondary" size="sm" onClick={saveProvider}>{t("provider.save")}</Button>
          </div>
          <p className="muted small">{t("provider.desc")}</p>
          <Separator style={{ margin: "12px 0" }} />
        </>
      )}
      {configured === true && (
        <div className="inline-form">
          <Glyph name="verified" />
          <span className="muted small" style={{ flex: 1 }}>{t("provider.configured")}</span>
        </div>
      )}

      {tasks.length === 0
        ? <div className="empty-state">{t("tasks.none")}</div>
        : (
          <div className="mini-list">
            {tasks.map((task) => {
              const runnable = RUNNABLE.has(task.status);
              const isRunning = task.status === "RUNNING" || runningTask === task.task_id;
              return (
                <div key={task.task_id} className="task-row">
                  <Glyph name="task" />
                  <span className="task-row-goal">{task.goal}</span>
                  <span className="muted small">{timeAgo(task.created_at)}</span>
                  {runnable && (
                    <Button size="sm" disabled={runningTask !== null} onClick={() => run(task)}>
                      <Glyph name="play" /> {t("tasks.run")}
                    </Button>
                  )}
                  <StatusBadge variant={statusVariant(task.status)}>
                    {t(TASK_STATUS_KEY[task.status] ?? "status.unknown")}
                  </StatusBadge>
                  {isRunning && <span className="muted small">{t("tasks.runningNow")}</span>}
                </div>
              );
            })}
          </div>
        )}
    </div>
  );
}
