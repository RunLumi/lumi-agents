import { useState, useEffect } from "react";
import { Sidebar } from "./components/chrome";
import { Glyph } from "./components/Icons";
import {
  projectListRecent,
  projectOverview,
  getOperationsSnapshot,
  emergencyStop,
  gitStatus,
  taskList,
  changeSets,
  artifactsList,
} from "./ipc/commands";
import type {
  ProjectSummary,
  ProjectOverview,
  GitStatus,
  Task,
  ChangeSet,
  ArtifactEntry,
  OperationsSnapshot,
} from "./ipc/types";

type Tab =
  | "home" | "tasks" | "files" | "changes" | "git"
  | "artifacts" | "evidence" | "approvals" | "settings";

interface ProjectData {
  summary: ProjectSummary;
  overview: ProjectOverview;
  git: GitStatus | null;
  tasks: Task[];
  sets: ChangeSet[];
  artifacts: ArtifactEntry[];
}

export default function App() {
  const [projects, setProjects] = useState<ProjectSummary[] | null>(null);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [data, setData] = useState<ProjectData | null>(null);
  const [snapshot, setSnapshot] = useState<OperationsSnapshot | null>(null);
  const [tab, setTab] = useState<Tab>("home");

  useEffect(() => {
    projectListRecent().then(setProjects).catch(() => {});
    getOperationsSnapshot().then(setSnapshot).catch(() => {});
    const timer = setInterval(() => {
      getOperationsSnapshot().then(setSnapshot).catch(() => {});
    }, 5000);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!projectId) return;
    projectOverview(projectId).then(setOverview).catch(() => {});
    gitStatus(projectId).then(setGit).catch(() => {});
    taskList(projectId).then(setTasks).catch(() => {});
    changeSets(projectId).then(setSets).catch(() => {});
    artifactsList(projectId).then(setArtifacts).catch(() => {});
  }, [projectId]);

  const setOverview = (o: ProjectOverview) => setData((d) => d ? { ...d, overview: o } : d);
  const setGit = (g: GitStatus | null) => setData((d) => d ? { ...d, git: g } : d);
  const setTasks = (t: Task[]) => setData((d) => d ? { ...d, tasks: t } : d);
  const setSets = (s: ChangeSet[]) => setData((d) => d ? { ...d, sets: s } : d);
  const setArtifacts = (a: ArtifactEntry[]) => setData((d) => d ? { ...d, artifacts: a } : d);

  const openProject = (pid: string) => {
    setProjectId(pid);
    setTab("home");
    projectOverview(pid).then(setOverview).catch(() => {});
    gitStatus(pid).then(setGit).catch(() => {});
    taskList(pid).then(setTasks).catch(() => {});
    changeSets(pid).then(setSets).catch(() => {});
    artifactsList(pid).then(setArtifacts).catch(() => {});
  };

  const activeNav = projectId ? (tab === "settings" ? "settings" : tab) : "projects";
  const badgeTasks = data?.tasks.filter(
    (t) => !["COMPLETED", "FAILED", "CANCELLED"].includes(t.status)
  ).length ?? 0;

  return (
    <div className="app">
      <Sidebar
        active={activeNav as any}
        badgeTasks={badgeTasks}
        badgeApprovals={0}
        snapshot={snapshot}
        collapsed={false}
        onToggleCollapse={() => {}}
        onNav={(key: string) => {
          if (key === "projects") { setProjectId(null); setTab("home" as Tab); }
          else if (projectId) setTab(key as Tab);
        }}
        onStop={() => {
          emergencyStop().then(() => {
            getOperationsSnapshot().then(setSnapshot).catch(() => {});
          });
        }}
      />
      <div className="main">
        <header className="topbar" data-tauri-drag-region>
          <div className="breadcrumb">
            <span className="crumb">Projects</span>
            {projectId && data && (
              <>
                <span className="crumb-sep">›</span>
                <span className="crumb">{data.overview.display_name}</span>
                <span className="crumb-sep">›</span>
                <span className="crumb current">{tab}</span>
              </>
            )}
            {!projectId && <span className="crumb current">Projects</span>}
          </div>
          <div className="topbar-right">
            <button className="btn btn-danger-outline btn-sm" onClick={() => emergencyStop().catch(() => {})}>
              <Glyph name="stop" /> Stop Agent
            </button>
          </div>
        </header>
        <main className="content">
          {projects == null ? (
            <div className="empty-state">Loading projects…</div>
          ) : !projectId ? (
            <ProjectList projects={projects} onOpen={openProject} />
          ) : data ? (
            <ProjectView data={data} tab={tab} setTab={setTab} />
          ) : null}
        </main>
      </div>
    </div>
  );
}

function ProjectList({ projects, onOpen }: {
  projects: ProjectSummary[];
  onOpen: (pid: string) => void;
}) {
  return (
    <div>
      <div className="section-head"><h2>Recent Projects</h2></div>
      <p className="section-sub">Click to open and continue with Lumi.</p>
      {!projects.length ? (
        <div className="empty-state">No projects yet.</div>
      ) : (
        <div className="recent-grid">
          {projects.map((p) => (
            <button key={p.project_id} className="recent-card" onClick={() => onOpen(p.project_id)}>
              <div className="recent-top">
                <span className="recent-ico"><Glyph name="folder" /></span>
                <span style={{ flex: 1, minWidth: 0 }}>
                  <div className="recent-name">{p.display_name}</div>
                  <div className="recent-desc">{p.detected_source}</div>
                </span>
                <span className={`pill ${p.health === "available" ? "pill-green" : "pill-amber"}`}>
                  {p.health === "available" ? "Available" : "Needs attention"}
                </span>
              </div>
              <div className="recent-meta">
                <span className="chip">{p.primary_root}</span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function ProjectView({ data, tab, setTab }: {
  data: ProjectData;
  tab: string;
  setTab: (t: Tab) => void;
}) {
  const { overview, git, tasks, artifacts } = data;
  return (
    <div>
      <div className="project-head">
        <div className="project-ico"><Glyph name="folder" /></div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <h1 className="project-title">{overview.display_name}</h1>
          <p className="project-desc">{overview.detected_source}</p>
          <div className="project-chips">
            <span className="chip">{overview.primary_root}</span>
            {git?.branch && <span className="chip">{git.branch}</span>}
            <span className="pill pill-green">Local • Safe</span>
          </div>
        </div>
      </div>
      <div className="tabs">
        {(["home", "tasks", "files", "changes", "git", "artifacts", "evidence", "approvals", "settings"] as const).map((t) => (
          <button key={t} className={`tab${tab === t ? " active" : ""}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>
      <div className="card" style={{ padding: 20 }}>
        <h2 style={{ marginTop: 0, textTransform: "capitalize", color: "var(--color-civic-navy)" }}>{tab}</h2>
        {git && (
          <div>
            <div className="kv"><span className="kv-key">Branch</span><span className="kv-val mono">{git.branch ?? "detached"}</span></div>
            <div className="kv"><span className="kv-key">Staged</span><span className="kv-val">{git.staged.length}</span></div>
            <div className="kv"><span className="kv-key">Modified</span><span className="kv-val">{git.unstaged.length}</span></div>
            <div className="kv"><span className="kv-key">Untracked</span><span className="kv-val">{git.untracked.length}</span></div>
          </div>
        )}
        {tasks.length > 0 && (
          <div style={{ marginTop: 12 }}>
            {tasks.map((t, i) => (
              <div key={i} className="mini-row">
                <span style={{ flex: 1 }}>{t.goal}</span>
                <span className="pill pill-blue">{t.status}</span>
              </div>
            ))}
          </div>
        )}
        {artifacts.length > 0 && (
          <div style={{ marginTop: 12 }}>
            <h3>Artifacts</h3>
            <div className="mini-list">
              {artifacts.map((a, i) => (
                <div key={i} className="mini-row">
                  <Glyph name="fileText" /><span style={{ flex: 1 }} className="mono">{a.name}</span>
                  <span className="muted small">{a.size} B</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
