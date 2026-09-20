import { useCallback, useEffect, useState } from "react";
import { open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { Sidebar, Topbar } from "./components/chrome";
import type { NavKey } from "./components/chrome";
import { ProjectsHome } from "./views/ProjectsHome";
import { ProjectDetail } from "./views/ProjectDetail";
import { toast } from "./lib/ui";
import { t, detect } from "./lib/i18n";
import { setLang } from "./lib/i18n";
import type { Lang } from "./lib/i18n";
import {
  emergencyStop, getOperationsSnapshot, projectListRecent, projectOpenFolder,
  projectOverview, gitStatus, taskList, changeSets, artifactsList,
} from "./ipc/commands";
import type {
  ArtifactEntry, ChangeSet, GitStatus, OperationsSnapshot,
  ProjectOverview, ProjectSummary, Task,
} from "./ipc/types";

interface ProjectData {
  overview: ProjectOverview;
  git: GitStatus | null;
  tasks: Task[];
  sets: ChangeSet[];
  artifacts: ArtifactEntry[];
}

async function loadProject(pid: string): Promise<ProjectData> {
  const [overview, git, tasks, sets, artifacts] = await Promise.all([
    projectOverview(pid),
    gitStatus(pid).catch(() => null),
    taskList(pid),
    changeSets(pid),
    artifactsList(pid),
  ]);
  return { overview, git, tasks, sets, artifacts };
}

export default function App() {
  const [lang, setLangState] = useState<Lang>(detect());
  const [projects, setProjects] = useState<ProjectSummary[] | null>(null);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [data, setData] = useState<ProjectData | null>(null);
  const [snapshot, setSnapshot] = useState<OperationsSnapshot | null>(null);

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
    loadProject(projectId).then(setData).catch(() => {});
  }, [projectId]);

  const openProject = useCallback((pid: string) => {
    setProjectId(pid);
    setData(null);
  }, []);

  const closeProject = useCallback(() => {
    setProjectId(null);
    setData(null);
  }, []);

  const reload = useCallback(() => {
    if (projectId) loadProject(projectId).then(setData).catch(() => {});
    projectListRecent().then(setProjects).catch(() => {});
  }, [projectId]);

  const openFolder = useCallback(async () => {
    const picked = await openFolderDialog({ directory: true, multiple: false });
    if (typeof picked !== "string" || !picked) return;
    try {
      const opened = await projectOpenFolder(picked);
      projectListRecent().then(setProjects).catch(() => {});
      openProject(opened.project.project_id);
      toast(t("misc.projectOpened"), "success");
    } catch {
      toast(t("misc.openFailed"), "error");
    }
  }, [openProject]);

  const switchLanguage = useCallback((next: Lang) => {
    setLang(next);
    setLangState(next);
  }, []);

  const stopAll = useCallback(() => {
    emergencyStop().then(() => {
      toast(t("misc.stopped"), "success");
      getOperationsSnapshot().then(setSnapshot).catch(() => {});
    }).catch(() => toast(t("misc.stopFail"), "error"));
  }, []);

  const activeNav: NavKey = projectId ? "tasks" : "projects";
  const badgeTasks = data?.tasks.filter(
    (task) => !["COMPLETED", "FAILED", "CANCELLED"].includes(task.status),
  ).length ?? 0;
  const badgeApprovals = snapshot?.pending_approvals?.length ?? 0;

  const crumbs = projectId && data
    ? [{ label: t("nav.projects"), go: "#projects" }, { label: data.overview.display_name }]
    : [{ label: t("nav.projects") }];

  return (
    <div className="app">
      <Sidebar
        active={activeNav}
        badgeTasks={badgeTasks}
        badgeApprovals={badgeApprovals}
        snapshot={snapshot}
        collapsed={false}
        onToggleCollapse={() => {}}
        onNav={(key) => {
          if (key === "projects") closeProject();
        }}
        onStop={stopAll}
      />
      <div className="main">
        <Topbar
          crumbs={crumbs}
          onSearch={openFolder}
          onStop={stopAll}
          lang={lang}
          onSwitch={switchLanguage}
          right={
            <button className="btn btn-primary btn-sm" onClick={openFolder}>
              {t("projects.openFolder")}
            </button>
          }
        />
        <main className="content">
          {projectId && data ? (
            <ProjectDetail
              overview={data.overview}
              git={data.git}
              tasks={data.tasks}
              sets={data.sets}
              artifacts={data.artifacts}
              pendingApprovals={snapshot?.pending_approvals ?? []}
              reload={reload}
              onReveal={() => openPath(data.overview.primary_root).catch(() => {})}
            />
          ) : projects ? (
            <ProjectsHome projects={projects} onOpen={openProject} />
          ) : (
            <div className="empty-state">{t("misc.loading")}</div>
          )}
        </main>
      </div>
    </div>
  );
}
