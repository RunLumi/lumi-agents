/* Project detail — Work-mode tabs (Home/Tasks/Changes/Git/Artifacts/
   Evidence/Approvals/Settings), built on shadcn/ui primitives themed to
   the Lumi design system. Status chips use the §10.4 lit-token Badge —
   no hand-rolled status color maps. Every section renders
   runtime-derived state with honest empty states; no sample data. */
import { useEffect, useState } from "react";
import { Glyph } from "../components/Icons";
import { t } from "../lib/i18n";
import { fmtSize, timeAgo, toast } from "../lib/ui";
import { Button } from "@/components/ui/button";
import { Field } from "@/components/ui/field";
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent,
  AlertDialogDescription, AlertDialogTitle, AlertDialogTrigger,
} from "@/components/ui/overlays";
import { Badge, StatusBadge, type BadgeVariant } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/overlays";
import { FilesTab, type FileRequest } from "./FilesTab";
import { TasksPanel } from "./TasksPanel";
import {
  connectionsConnect, connectionsDisconnect, connectionsList, connectionsVerify, gitBranches,
  gitCommit, gitLog, gitSwitch, projectRelink, projectRemove, taskCreate,
} from "../ipc/commands";
import type { ConnectionRecord } from "../ipc/commands";
import { useCallback } from "react";
import { projectEvidence, type EvidenceSummaryEntry } from "../ipc/commands";
import type {
  ArtifactEntry, ChangeSet, CommitInfo, GitStatus,
  PendingApproval, ProjectOverview, Task, ValidationRecord,
} from "../ipc/types";

interface Props {
  overview: ProjectOverview;
  git: GitStatus | null;
  tasks: Task[];
  sets: ChangeSet[];
  artifacts: ArtifactEntry[];
  pendingApprovals: PendingApproval[];
  tab: string;
  onTabChange: (tab: string) => void;
  fileRequest: FileRequest | null;
  onRequestConsumed: () => void;
  onProjectRemoved: () => void;
  reload: () => void;
  onReveal: () => void;
}

const TABS: [string, string][] = [
  ["home", "nav.home"], ["tasks", "nav.tasks"], ["files", "nav.files"],
  ["changes", "nav.changes"], ["git", "nav.git"], ["artifacts", "nav.artifacts"],
  ["evidence", "nav.evidence"], ["approvals", "nav.approvals"], ["settings", "nav.settings"],
];

const TASK_STATUS_KEY: Record<string, string> = {
  CREATED: "status.created", QUEUED: "status.queued", RUNNING: "status.running",
  WAITING_APPROVAL: "status.waitingApproval", WAITING_USER: "status.waitingUser",
  WAITING_EXTERNAL: "status.waitingExternal", PAUSED: "status.paused",
  COMPLETED: "status.completed", FAILED: "status.failed",
  AMBIGUOUS: "status.ambiguous", CANCELLED: "status.cancelled",
};

/** Domain status → §10.4 badge family (the only allowed mapping). */
function taskStatusVariant(status: string): BadgeVariant {
  if (status === "COMPLETED") return "status-done";
  if (status === "FAILED") return "status-critical";
  if (status === "RUNNING") return "status-in-progress";
  if (status.startsWith("WAITING") || status === "AMBIGUOUS") return "status-overdue";
  return "status-open";
}

function validationVariant(status: ValidationRecord["status"]): BadgeVariant {
  if (status === "passed") return "status-done";
  if (status === "failed") return "status-critical";
  if (status === "ambiguous") return "status-overdue";
  return "status-open";
}

function TaskRow({ task }: { task: Task }) {
  return (
    <div className="task-row">
      <Glyph name="task" />
      <span className="task-row-goal">{task.goal}</span>
      <span className="muted small">{timeAgo(task.created_at)}</span>
      <StatusBadge variant={taskStatusVariant(task.status)}>
        {t(TASK_STATUS_KEY[task.status] ?? "status.unknown")}
      </StatusBadge>
    </div>
  );
}

function ValidationRow({ v }: { v: ValidationRecord }) {
  return (
    <div className="mini-row">
      <Glyph name="verified" />
      <span style={{ flex: 1, minWidth: 0 }} className="mono small">{v.command}</span>
      <span className="muted small">{v.role}{v.exit_code != null ? ` · exit ${v.exit_code}` : ""}</span>
      <StatusBadge variant={validationVariant(v.status)}>{v.status}</StatusBadge>
    </div>
  );
}

/** Trust-language label → §10.4 badge family (never render "done"
    for anything the ledger did not verify). */
function trustVariant(label: string): BadgeVariant {
  if (label.startsWith("Verified")) return "status-done";
  if (label.startsWith("Failed")) return "status-critical";
  if (label.startsWith("Ambiguous")) return "status-overdue";
  if (label.startsWith("Executed") || label.startsWith("Attempted")) return "status-in-progress";
  return "status-open";
}

function EntryRow({ e }: { e: ChangeSet["entries"][number] }) {
  const glyph = e.kind === "deleted" ? "trash" : e.kind === "moved" ? "external" : e.kind === "created" ? "plus" : "edit";
  const external = e.source === "external_conflict";
  return (
    <div className="mini-row">
      <Glyph name={glyph} />
      <span style={{ flex: 1, minWidth: 0 }} className="mono small">{e.path}</span>
      {external && <StatusBadge variant="status-overdue">{t("changes.externalConflict")}</StatusBadge>}
      <span className="muted small">{e.kind}</span>
    </div>
  );
}

export function ProjectDetail({
  overview, git, tasks, sets, artifacts, pendingApprovals,
  tab, onTabChange, fileRequest, onRequestConsumed, onProjectRemoved, reload, onReveal,
}: Props) {
  const [relinkPath, setRelinkPath] = useState("");
  const [goal, setGoal] = useState("");
  const [connections, setConnections] = useState<ConnectionRecord[]>([]);
  const [connName, setConnName] = useState("");
  const [connKind, setConnKind] = useState("api_key");
  const [connEndpoint, setConnEndpoint] = useState("");
  const [connCredential, setConnCredential] = useState("");

  const refreshConnections = useCallback(() => {
    connectionsList(overview.project_id).then(setConnections).catch(() => {});
  }, [overview.project_id]);

  useEffect(() => {
    refreshConnections();
  }, [refreshConnections]);

  const connectConnection = () => {
    if (!connName.trim() || !connEndpoint.trim() || !connCredential.trim()) {
      toast(t("connections.missingFields"), "error");
      return;
    }
    connectionsConnect(overview.project_id, {
      name: connName.trim(),
      kind: connKind,
      endpoint: connEndpoint.trim(),
      credential: connCredential.trim(),
    }).then(() => {
      setConnName(""); setConnEndpoint(""); setConnCredential("");
      toast(t("connections.saved"), "success");
      refreshConnections();
    }).catch((e) => toast(String(e), "error"));
  };

  const disconnectConnection = (record: ConnectionRecord) => {
    connectionsDisconnect(overview.project_id, record.connection_id).then(() => {
      toast(t("connections.disconnected"), "success");
      refreshConnections();
    }).catch((e) => toast(String(e), "error"));
  };

  // Per-row verify state: undefined = not checked, true/false = broker
  // result. The check never surfaces the credential value.
  const [verifyState, setVerifyState] = useState<Record<string, boolean | undefined>>({});
  const verifyConnection = (record: ConnectionRecord) => {
    connectionsVerify(overview.project_id, record.connection_id).then((present) => {
      setVerifyState((prev) => ({ ...prev, [record.connection_id]: present }));
      toast(present ? t("connections.verifyOk") : t("connections.verifyMissing"), present ? "success" : "error");
    }).catch((e) => toast(String(e), "error"));
  };
  const [commitMsg, setCommitMsg] = useState("");
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [branchName, setBranchName] = useState("");
  const [log, setLog] = useState<CommitInfo[] | null>(null);
  const [branches, setBranches] = useState<string[] | null>(null);
  const [evidence, setEvidence] = useState<EvidenceSummaryEntry[] | null>(null);
  const [evidenceLive, setEvidenceLive] = useState(false);

  // Evidence is a live read over the runtime audit ledger: refetch when
  // the tab opens and whenever project state changes.
  useEffect(() => {
    if (tab !== "evidence") return;
    projectEvidence(overview.project_id).then((dto) => {
      setEvidence(dto.entries);
      setEvidenceLive(dto.running);
    }).catch(() => setEvidence([]));
  }, [tab, overview.project_id, tasks, sets]);

  const validations = sets.flatMap((s) => s.validations ?? []);
  const failing = validations.filter((v) => v.status === "failed" || v.status === "ambiguous");
  const changedFiles = [...(git?.staged ?? []), ...(git?.unstaged ?? [])];

  useEffect(() => {
    if (tab !== "git" || !git) return;
    gitLog(overview.project_id, 8).then(setLog).catch(() => setLog([]));
    gitBranches(overview.project_id).then(setBranches).catch(() => setBranches([]));
  }, [tab, git, overview.project_id]);

  const createTask = () => {
    if (!goal.trim()) { toast(t("tasks.describeGoal"), "error"); return; }
    taskCreate(overview.project_id, goal.trim()).then(() => {
      setGoal("");
      toast(t("tasks.created"), "success");
      reload();
    }).catch(() => toast(t("misc.openFailed"), "error"));
  };

  const commitPicked = () => {
    if (!commitMsg.trim() || picked.size === 0) { toast(t("git.tickFirst"), "error"); return; }
    gitCommit(overview.project_id, commitMsg.trim(), [...picked]).then(() => {
      setCommitMsg(""); setPicked(new Set());
      toast(t("git.committed"), "success");
      reload();
      gitLog(overview.project_id, 8).then(setLog).catch(() => {});
      gitBranches(overview.project_id).then(setBranches).catch(() => {});
    }).catch(() => toast(t("files.saveFail"), "error"));
  };

  const switchBranch = (name: string) => {
    gitSwitch(overview.project_id, name).then(() => {
      toast(t("git.current"), "success");
      reload();
    }).catch(() => toast(t("misc.openFailed"), "error"));
  };

  const togglePick = (path: string) => {
    setPicked((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path); else next.add(path);
      return next;
    });
  };

  const readiness = (ok: string, attention: string, passing: boolean, none?: string) => (
    <StatusBadge variant={passing ? "status-done" : "status-overdue"}>
      {passing ? t(ok) : t(none ?? attention)}
    </StatusBadge>
  );

  return (
    <div>
      <div className="project-head">
        <div className="project-ico"><Glyph name="folder" /></div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <h1 className="project-title">{overview.display_name}</h1>
          <p className="project-desc">{overview.detected_source}</p>
          <div className="project-chips">
            <span className="chip mono">{overview.primary_root}</span>
            {git?.branch && <span className="chip"><Glyph name="branch" /> {git.branch}</span>}
            {overview.health === "available"
              ? <StatusBadge variant="status-done">{t("status.available")}</StatusBadge>
              : <StatusBadge variant="status-overdue">{t("status.moved")}</StatusBadge>}
          </div>
        </div>
        <Button variant="secondary" size="sm" onClick={onReveal}>
          <Glyph name="external" /> {t("project.reveal")}
        </Button>
      </div>

      <Tabs value={tab} onValueChange={onTabChange}>
        <TabsList>
          {TABS.map(([key, i18n]) => (
            <TabsTrigger key={key} value={key}>{t(i18n)}</TabsTrigger>
          ))}
        </TabsList>

        <TabsContent value="home">
          <div className="two-cards">
            <div className="card">
              <h3>{t("git.readiness")}</h3>
              <div className="kv"><span className="kv-key">{t("git.currentBranch")}</span><span className="kv-val mono">{git?.branch ?? t("git.detached")}</span></div>
              <div className="kv">
                <span className="kv-key">{t("git.rWorkingTree")}</span>
                <span className="kv-val">
                  {git
                    ? readiness("status.clean", "status.dirty", git.staged.length + git.unstaged.length + git.untracked.length === 0)
                    : <Badge variant="status-open">{t("status.noGit")}</Badge>}
                </span>
              </div>
              <div className="kv">
                <span className="kv-key">{t("git.rValidations")}</span>
                <span className="kv-val">
                  {validations.length === 0
                    ? <Badge variant="status-open">{t("git.rNone")}</Badge>
                    : readiness("git.rAllPassing", "git.rAttention", failing.length === 0)}
                </span>
              </div>
            </div>
            <div className="card">
              <h3>{t("nav.tasks")}</h3>
              {tasks.length === 0
                ? <div className="empty-state">{t("tasks.none")}</div>
                : <div className="mini-list">{tasks.slice(0, 4).map((task) => <TaskRow key={task.task_id} task={task} />)}</div>}
            </div>
          </div>
        </TabsContent>

        <TabsContent value="tasks">
          <div className="card">
            <div className="inline-form">
              <Field id="task-goal" label={t("tasks.goalLabel")}>
                <Input
                  id="task-goal"
                  value={goal}
                  onChange={(e) => setGoal(e.target.value)}
                  placeholder={t("tasks.placeholder")}
                  onKeyDown={(e) => e.key === "Enter" && createTask()}
                />
              </Field>
              <Button size="sm" onClick={createTask}>{t("tasks.create")}</Button>
            </div>
            <Separator style={{ margin: "12px 0" }} />
            <TasksPanel projectId={overview.project_id} tasks={tasks} onChanged={reload} />
          </div>
        </TabsContent>

        <TabsContent value="files">
          <FilesTab
            projectId={overview.project_id}
            request={fileRequest}
            onRequestConsumed={onRequestConsumed}
            onMutated={reload}
          />
        </TabsContent>

        <TabsContent value="changes">
          <div className="card">
            <h3>{t("changes.title")}</h3>
            <p className="muted small">{t("changes.preExisting")}</p>
            {sets.length === 0
              ? <div className="empty-state">{t("changes.empty")}</div>
              : sets.map((s) => (
                <div key={s.task_id.task_id} className="note-card">
                  <div className="kv">
                    <span className="kv-key">{t("changes.task")}</span>
                    <span className="kv-val mono small">{s.task_id.task_id}</span>
                    <span className="muted small">{s.entries.length} {t("changes.fileChanges")}</span>
                  </div>
                  <div className="mini-list">
                    {s.entries.map((e, i) => <EntryRow key={i} e={e} />)}
                    {(s.validations ?? []).map((v, i) => <ValidationRow key={i} v={v} />)}
                  </div>
                </div>
              ))}
          </div>
        </TabsContent>

        <TabsContent value="git">
          {!git ? (
            <div className="card"><div className="empty-state">{t("git.noRepo")}</div></div>
          ) : (
            <>
              <div className="card">
                <h3>{t("git.changesInWorkdir")}</h3>
                {changedFiles.length === 0 && git.untracked.length === 0
                  ? <div className="empty-state">{t("git.cleanTree")}</div>
                  : (
                    <div className="mini-list">
                      {changedFiles.map((f) => (
                        <label key={f.path} className="mini-row" style={{ cursor: "pointer" }}>
                          <input type="checkbox" checked={picked.has(f.path)} onChange={() => togglePick(f.path)} />
                          <span style={{ flex: 1, minWidth: 0 }} className="mono small">{f.path}</span>
                          <span className="muted small">{f.worktree_state ?? f.index_state ?? ""}</span>
                        </label>
                      ))}
                      {git.untracked.map((p) => (
                        <label key={p} className="mini-row" style={{ cursor: "pointer" }}>
                          <input type="checkbox" checked={picked.has(p)} onChange={() => togglePick(p)} />
                          <span style={{ flex: 1, minWidth: 0 }} className="mono small">{p}</span>
                          <span className="muted small">{t("git.untracked")}</span>
                        </label>
                      ))}
                    </div>
                  )}
                <div className="inline-form">
                  <Field id="git-commit-message" label={t("git.commitMsgLabel")}>
                    <Input
                      id="git-commit-message"
                      value={commitMsg}
                      onChange={(e) => setCommitMsg(e.target.value)}
                      placeholder={t("git.commitMsg")}
                    />
                  </Field>
                  <Button size="sm" onClick={commitPicked}>{t("git.commitSelected")}</Button>
                </div>
                <p className="muted small">{t("git.pathScoped")}</p>
              </div>
              <div className="two-cards">
                <div className="card">
                  <h3>{t("git.branches")}</h3>
                  <div className="inline-form">
                    <Field id="git-new-branch" label={t("git.newBranchLabel")}>
                      <Input id="git-new-branch" value={branchName} onChange={(e) => setBranchName(e.target.value)} placeholder={t("git.newBranch")} />
                    </Field>
                  </div>
                  <div className="mini-list">
                    {(branches ?? []).map((b) => (
                      <div key={b} className="mini-row">
                        <Glyph name="branch" />
                        <span style={{ flex: 1 }} className="mono small">{b}</span>
                        {b === git.branch
                          ? <StatusBadge variant="status-in-progress">{t("git.current")}</StatusBadge>
                          : <Button variant="secondary" size="sm" onClick={() => switchBranch(b)}>{t("git.switchTo")}</Button>}
                      </div>
                    ))}
                  </div>
                </div>
                <div className="card">
                  <h3>{t("git.recentCommits")}</h3>
                  {(log ?? []).length === 0
                    ? <div className="empty-state">{t("git.noCommits")}</div>
                    : (
                      <div className="mini-list">
                        {log!.map((c) => (
                          <div key={c.hash} className="mini-row">
                            <Glyph name="commit" />
                            <span className="mono small">{c.short_hash}</span>
                            <span style={{ flex: 1, minWidth: 0 }} className="small">{c.subject}</span>
                          </div>
                        ))}
                      </div>
                    )}
                </div>
              </div>
            </>
          )}
        </TabsContent>

        <TabsContent value="artifacts">
          <div className="card">
            <h3>{t("artifacts.title")}</h3>
            <p className="muted small">{t("artifacts.sub")}</p>
            {artifacts.length === 0
              ? <div className="empty-state">{t("artifacts.empty")}</div>
              : (
                <div className="mini-list">
                  {artifacts.map((a) => (
                    <div key={a.path} className="mini-row">
                      <Glyph name="artifact" />
                      <span style={{ flex: 1, minWidth: 0 }} className="mono small">{a.name}</span>
                      {a.artifact_type && <Badge variant="secondary" className="font-mono">{a.artifact_type}</Badge>}
                      {a.lifecycle && <StatusBadge variant="status-in-progress">{a.lifecycle}</StatusBadge>}
                      <span className="muted small">{fmtSize(a.size)}</span>
                    </div>
                  ))}
                </div>
              )}
          </div>
        </TabsContent>

        <TabsContent value="evidence">
          <div className="card">
            <h3>{t("evidence.title")}</h3>
            <p className="muted small">{t("evidence.sub")}</p>
            {evidenceLive && <p className="muted small">{t("evidence.running")}</p>}
            {evidence === null
              ? <div className="muted small">{t("misc.loading")}</div>
              : evidence.length === 0
                ? <div className="empty-state">{t("evidence.empty")}</div>
                : (
                  <div className="mini-list">
                    {evidence.map((entry) => (
                      <div key={entry.action_id} className="mini-row">
                        <Glyph name="verified" />
                        <span style={{ flex: 1, minWidth: 0 }} className="small">{entry.operation}</span>
                        {entry.target && (
                          <span className="mono muted small" style={{ maxWidth: "40%", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                            {entry.target.replace("file://", "")}
                          </span>
                        )}
                        <StatusBadge variant={trustVariant(entry.trust_label)}>{entry.trust_label}</StatusBadge>
                      </div>
                    ))}
                  </div>
                )}
          </div>
        </TabsContent>

        <TabsContent value="approvals">
          <div className="card">
            <h3>{t("approvals.title")}</h3>
            <p className="muted small">{t("approvals.sub")}</p>
            {pendingApprovals.length === 0 ? (
              <div className="empty-state">{t("approvals.none")}</div>
            ) : (
              <div className="mini-list">
                {pendingApprovals.map((a, i) => (
                  <div key={a.action_id ?? i} className="mini-row">
                    <Glyph name="approval" />
                    <span style={{ flex: 1, minWidth: 0 }}>{a.operation ?? a.action_id}</span>
                    <StatusBadge variant="status-overdue">{t("status.waitingApproval")}</StatusBadge>
                  </div>
                ))}
              </div>
            )}
          </div>
        </TabsContent>

        <TabsContent value="settings">
          <div className="two-cards">
            <div className="card">
              <h3>{t("settings.title")}</h3>
              <div className="kv"><span className="kv-key">{t("settings.projectId")}</span><span className="kv-val mono small">{overview.project_id}</span></div>
              <div className="kv"><span className="kv-key">{t("settings.root")}</span><span className="kv-val mono small">{overview.primary_root}</span></div>
              <div className="kv"><span className="kv-key">{t("settings.env")}</span><span className="kv-val mono small">{overview.environment_id}</span></div>
              <div className="kv"><span className="kv-key">{t("settings.source")}</span><span className="kv-val">{overview.detected_source}</span></div>
              <div className="kv">
                <span className="kv-key">{t("settings.instructions")}</span>
                <span className="kv-val">{overview.instructions.length ? overview.instructions.join(", ") : t("settings.noneFound")}</span>
              </div>
              <h3 style={{ marginTop: 16 }}>{t("connections.title")}</h3>
              {connections.length === 0 ? (
                <p className="muted small">{t("connections.empty")}</p>
              ) : (
                <div className="mini-list">
                  {connections.map((record) => (
                    <div key={record.connection_id} className="mini-row">
                      <Glyph name="branch" />
                      <span style={{ flex: 1, minWidth: 0 }} className="small">
                        {record.name}
                        <span className="muted small" style={{ marginLeft: 6 }}>
                          {new Date(record.created_at).toLocaleDateString()}
                        </span>
                      </span>
                      <span className="mono muted small" style={{ maxWidth: "38%", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {record.endpoint}
                      </span>
                      {verifyState[record.connection_id] === true ? (
                        <StatusBadge variant="status-done">{t("connections.verifyOk")}</StatusBadge>
                      ) : verifyState[record.connection_id] === false ? (
                        <StatusBadge variant="status-critical">{t("connections.verifyMissing")}</StatusBadge>
                      ) : (
                        <StatusBadge variant="status-done">{t("connections.connected")}</StatusBadge>
                      )}
                      <Button variant="secondary" size="sm" onClick={() => verifyConnection(record)}>
                        {t("connections.verify")}
                      </Button>
                      <Button variant="secondary" size="sm" onClick={() => disconnectConnection(record)}>
                        {t("connections.disconnect")}
                      </Button>
                    </div>
                  ))}
                </div>
              )}
              <h3 style={{ marginTop: 16 }}>{t("connections.add")}</h3>
              <p className="muted small">{t("connections.desc")}</p>
              <div className="inline-form">
                <Input value={connName} onChange={(e) => setConnName(e.target.value)} placeholder={t("connections.name")} />
                <select
                  className="input"
                  value={connKind}
                  onChange={(e) => setConnKind(e.target.value)}
                  aria-label={t("connections.kind")}
                >
                  <option value="api_key">api_key</option>
                  <option value="basic">basic</option>
                  <option value="custom">custom</option>
                </select>
              </div>
              <div className="inline-form">
                <Input value={connEndpoint} onChange={(e) => setConnEndpoint(e.target.value)} placeholder={t("connections.endpoint")} />
                <Input value={connCredential} onChange={(e) => setConnCredential(e.target.value)} placeholder={t("connections.credential")} type="password" />
                <Button size="sm" onClick={connectConnection}>{t("connections.add")}</Button>
              </div>
              <p className="muted small">{t("connections.kind")}: {connKind}</p>
            </div>
            <div className="card">
              <h3>{t("settings.caps")}</h3>
              <p className="muted small">{t("settings.capsSub")}</p>
              <div className="project-chips">
                {overview.capabilities.map((c) => (
                  <Badge key={c} variant="secondary" className="font-mono">{c}</Badge>
                ))}
              </div>
              <h3 style={{ marginTop: 16 }}>{t("settings.trustTitle")}</h3>
              <ul className="check-list">
                {["settings.trust1", "settings.trust2", "settings.trust3", "settings.trust4"].map((k) => (
                  <li key={k}>
                    <span className="check-yes"><Glyph name="verified" /></span>
                    <span>{t(k)}</span>
                  </li>
                ))}
              </ul>
              <h3 style={{ marginTop: 16 }}>{t("settings.relink")}</h3>
              <div className="inline-form">
                <Field id="settings-relink-path" label={t("settings.relinkLabel")}>
                  <Input
                    id="settings-relink-path"
                    value={relinkPath}
                    onChange={(e) => setRelinkPath(e.target.value)}
                    placeholder={t("settings.relinkPrompt")}
                  />
                </Field>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => {
                    const path = relinkPath.trim();
                    if (!path) { toast(t("files.saveFail"), "error"); return; }
                    projectRelink(overview.project_id, path).then(() => {
                      setRelinkPath("");
                      toast(t("misc.relabeled"), "success");
                      reload();
                    }).catch(() => toast(t("files.saveFail"), "error"));
                  }}
                >
                  {t("settings.relink")}
                </Button>
              </div>
              <div style={{ marginTop: 16 }}>
                <AlertDialog>
                  <AlertDialogTrigger asChild>
                    <Button variant="destructive" size="sm">{t("settings.remove")}</Button>
                  </AlertDialogTrigger>
                  <AlertDialogContent>
                    <AlertDialogTitle>{t("settings.remove")}</AlertDialogTitle>
                    <AlertDialogDescription>{t("settings.removeConfirm")}</AlertDialogDescription>
                    <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
                      <AlertDialogCancel asChild>
                        <Button variant="secondary" size="sm">{t("files.cancel")}</Button>
                      </AlertDialogCancel>
                      <AlertDialogAction asChild>
                        <Button variant="destructive" size="sm" onClick={() => {
                          projectRemove(overview.project_id).then(() => {
                            toast(t("misc.removed"), "success");
                            onProjectRemoved();
                          }).catch(() => toast(t("files.saveFail"), "error"));
                        }}>
                          {t("settings.remove")}
                        </Button>
                      </AlertDialogAction>
                    </div>
                  </AlertDialogContent>
                </AlertDialog>
              </div>
            </div>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
