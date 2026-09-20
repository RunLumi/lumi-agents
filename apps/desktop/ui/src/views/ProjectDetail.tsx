/* Project detail — Work-mode tabs (Home/Tasks/Changes/Git/Artifacts/
   Evidence/Approvals/Settings). Every section renders runtime-derived
   state with honest empty states; no sample data. */
import { useEffect, useState } from "react";
import { Glyph } from "../components/Icons";
import { t } from "../lib/i18n";
import { fmtSize, timeAgo, toast } from "../lib/ui";
import {
  gitBranches, gitCommit, gitLog, gitSwitch, taskCreate,
} from "../ipc/commands";
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
  reload: () => void;
  onReveal: () => void;
}

const TABS: [string, string][] = [
  ["home", "nav.home"], ["tasks", "nav.tasks"], ["changes", "nav.changes"],
  ["git", "nav.git"], ["artifacts", "nav.artifacts"], ["evidence", "nav.evidence"],
  ["approvals", "nav.approvals"], ["settings", "nav.settings"],
];

const TASK_STATUS_KEY: Record<string, string> = {
  CREATED: "status.created", QUEUED: "status.queued", RUNNING: "status.running",
  WAITING_APPROVAL: "status.waitingApproval", WAITING_USER: "status.waitingUser",
  WAITING_EXTERNAL: "status.waitingExternal", PAUSED: "status.paused",
  COMPLETED: "status.completed", FAILED: "status.failed",
  AMBIGUOUS: "status.ambiguous", CANCELLED: "status.cancelled",
};

function taskPillClass(status: string): string {
  if (status === "COMPLETED") return "pill pill-green";
  if (status === "FAILED") return "pill pill-red";
  if (status === "RUNNING") return "pill pill-blue";
  if (status.startsWith("WAITING") || status === "AMBIGUOUS") return "pill pill-amber";
  return "pill pill-gray";
}

function validationPill(v: ValidationRecord): string {
  if (v.status === "passed") return "pill pill-green";
  if (v.status === "failed") return "pill pill-red";
  if (v.status === "ambiguous") return "pill pill-amber";
  return "pill pill-gray";
}

function TaskRow({ task }: { task: Task }) {
  return (
    <div className="task-row">
      <Glyph name="task" />
      <span className="task-row-goal">{task.goal}</span>
      <span className="muted small">{timeAgo(task.created_at)}</span>
      <span className={taskPillClass(task.status)}>{t(TASK_STATUS_KEY[task.status] ?? "status.unknown")}</span>
    </div>
  );
}

function ValidationRow({ v }: { v: ValidationRecord }) {
  return (
    <div className="mini-row">
      <Glyph name="verified" />
      <span style={{ flex: 1, minWidth: 0 }} className="mono small">{v.command}</span>
      <span className="muted small">{v.role}{v.exit_code != null ? ` · exit ${v.exit_code}` : ""}</span>
      <span className={validationPill(v)}>{v.status}</span>
    </div>
  );
}

function EntryRow({ e }: { e: ChangeSet["entries"][number] }) {
  const glyph = e.kind === "deleted" ? "trash" : e.kind === "moved" ? "external" : e.kind === "created" ? "plus" : "edit";
  const external = e.source === "external_conflict";
  return (
    <div className="mini-row">
      <Glyph name={glyph} />
      <span style={{ flex: 1, minWidth: 0 }} className="mono small">{e.path}</span>
      {external && <span className="pill pill-amber">{t("changes.externalConflict")}</span>}
      <span className="muted small">{e.kind}</span>
    </div>
  );
}

export function ProjectDetail({ overview, git, tasks, sets, artifacts, pendingApprovals, reload, onReveal }: Props) {
  const [tab, setTab] = useState<string>("home");
  const [goal, setGoal] = useState("");
  const [commitMsg, setCommitMsg] = useState("");
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [branchName, setBranchName] = useState("");
  const [log, setLog] = useState<CommitInfo[] | null>(null);
  const [branches, setBranches] = useState<string[] | null>(null);

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
    <span className={`pill ${passing ? "pill-green" : "pill-amber"}`}>{passing ? t(ok) : t(none ?? attention)}</span>
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
            <span className={`pill ${overview.health === "available" ? "pill-green" : "pill-amber"}`}>
              {t(overview.health === "available" ? "status.available" : "status.moved")}
            </span>
          </div>
        </div>
        <button className="btn btn-sm" onClick={onReveal}><Glyph name="external" /> {t("project.reveal")}</button>
      </div>

      <div className="tabs">
        {TABS.map(([key, i18n]) => (
          <button key={key} className={`tab${tab === key ? " active" : ""}`} onClick={() => setTab(key)}>
            {t(i18n)}
          </button>
        ))}
      </div>

      {tab === "home" && (
        <div className="two-cards">
          <div className="card">
            <h3>{t("git.readiness")}</h3>
            <div className="kv"><span className="kv-key">{t("git.currentBranch")}</span><span className="kv-val mono">{git?.branch ?? t("git.detached")}</span></div>
            <div className="kv">
              <span className="kv-key">{t("git.rWorkingTree")}</span>
              <span className="kv-val">
                {git
                  ? readiness("status.clean", "status.dirty", git.staged.length + git.unstaged.length + git.untracked.length === 0)
                  : <span className="pill pill-gray">{t("status.noGit")}</span>}
              </span>
            </div>
            <div className="kv">
              <span className="kv-key">{t("git.rValidations")}</span>
              <span className="kv-val">
                {validations.length === 0
                  ? <span className="pill pill-gray">{t("git.rNone")}</span>
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
      )}

      {tab === "tasks" && (
        <div className="card">
          <div className="inline-form">
            <input
              className="input"
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              placeholder={t("tasks.placeholder")}
              onKeyDown={(e) => e.key === "Enter" && createTask()}
            />
            <button className="btn btn-primary btn-sm" onClick={createTask}>{t("tasks.create")}</button>
          </div>
          {tasks.length === 0
            ? <div className="empty-state">{t("tasks.none")}</div>
            : <div className="mini-list">{tasks.map((task) => <TaskRow key={task.task_id} task={task} />)}</div>}
        </div>
      )}

      {tab === "changes" && (
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
      )}

      {tab === "git" && (!git ? (
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
              <input
                className="input"
                value={commitMsg}
                onChange={(e) => setCommitMsg(e.target.value)}
                placeholder={t("git.commitMsg")}
              />
              <button className="btn btn-primary btn-sm" onClick={commitPicked}>{t("git.commitSelected")}</button>
            </div>
            <p className="muted small">{t("git.pathScoped")}</p>
          </div>
          <div className="two-cards">
            <div className="card">
              <h3>{t("git.branches")}</h3>
              <div className="inline-form">
                <input className="input" value={branchName} onChange={(e) => setBranchName(e.target.value)} placeholder={t("git.newBranch")} />
              </div>
              <div className="mini-list">
                {(branches ?? []).map((b) => (
                  <div key={b} className="mini-row">
                    <Glyph name="branch" />
                    <span style={{ flex: 1 }} className="mono small">{b}</span>
                    {b === git.branch
                      ? <span className="pill pill-blue">{t("git.current")}</span>
                      : <button className="btn btn-sm" onClick={() => switchBranch(b)}>{t("git.switchTo")}</button>}
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
      ))}

      {tab === "artifacts" && (
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
                    {a.artifact_type && <span className="chip">{a.artifact_type}</span>}
                    {a.lifecycle && <span className="pill pill-blue">{a.lifecycle}</span>}
                    <span className="muted small">{fmtSize(a.size)}</span>
                  </div>
                ))}
              </div>
            )}
        </div>
      )}

      {tab === "evidence" && (
        <div className="card">
          <h3>{t("evidence.title")}</h3>
          <p className="muted small">{t("evidence.sub")}</p>
          <div className="empty-state">{t("evidence.empty")}</div>
        </div>
      )}

      {tab === "approvals" && (
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
                  <span className="pill pill-amber">{t("status.waitingApproval")}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {tab === "settings" && (
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
          </div>
          <div className="card">
            <h3>{t("settings.caps")}</h3>
            <p className="muted small">{t("settings.capsSub")}</p>
            <div className="project-chips">
              {overview.capabilities.map((c) => <span key={c} className="chip">{c}</span>)}
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
          </div>
        </div>
      )}
    </div>
  );
}
