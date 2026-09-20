import { useCallback, useEffect, useRef, useState } from "react";
import { open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { Sidebar, Topbar } from "./components/chrome";
import { ProjectsHome } from "./views/ProjectsHome";
import { ProjectDetail } from "./views/ProjectDetail";
import type { FileRequest } from "./views/FilesTab";
import { Palette } from "./components/palette";
import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/sonner";
import { toast } from "./lib/ui";
import { t, detect } from "./lib/i18n";
import { setLang } from "./lib/i18n";
import type { Lang } from "./lib/i18n";
import { activeNavFor, type NavKey } from "./lib/navigation";
import {
  emergencyStop, getOperationsSnapshot, projectListRecent, projectOpenFolder,
  projectOverview, gitStatus, taskList, changeSets, artifactsList, automationsTick,
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
  const [detailTab, setDetailTab] = useState("home");
  const [fileRequest, setFileRequest] = useState<FileRequest | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const searchTriggerRef = useRef<HTMLButtonElement>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    try { return localStorage.getItem("lumi-sidebar-collapsed") === "1"; } catch { return false; }
  });

  // Collapsed rail is a body-level style (lumi.css) and persists across
  // launches, same contract as the vanilla implementation.
  useEffect(() => {
    document.body.classList.toggle("sidebar-collapsed", sidebarCollapsed);
    try { localStorage.setItem("lumi-sidebar-collapsed", sidebarCollapsed ? "1" : "0"); } catch {}
  }, [sidebarCollapsed]);

  // ⌘K / Ctrl+K opens the palette; the topbar search box opens it too.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    projectListRecent().then(setProjects).catch(() => {});
    getOperationsSnapshot().then(setSnapshot).catch(() => {});
    const timer = setInterval(() => {
      getOperationsSnapshot().then(setSnapshot).catch(() => {});
    }, 5000);
    return () => clearInterval(timer);
  }, []);

  // Automation tick (Spec 27): every 30s the shell checks due project
  // schedules; the webview supplies the device's local UTC offset. A
  // tick while a run holds the runtime is skipped honestly by the shell.
  useEffect(() => {
    const tick = () => {
      automationsTick(-new Date().getTimezoneOffset() * 60).catch(() => {});
    };
    tick();
    const timer = setInterval(tick, 30_000);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!projectId) return;
    loadProject(projectId).then(setData).catch(() => {});
  }, [projectId]);

  const openProject = useCallback((pid: string) => {
    setProjectId(pid);
    setData(null);
    setDetailTab("home");
  }, []);

  const closeProject = useCallback(() => {
    setProjectId(null);
    setData(null);
    setFileRequest(null);
  }, []);

  const openFileFromPalette = useCallback((path: string) => {
    if (!projectId) return;
    setDetailTab("files");
    setFileRequest({ path, nonce: Date.now() });
  }, [projectId]);

  const sidebarNav = useCallback((key: NavKey) => {
    if (key === "projects") { closeProject(); return; }
    // Every other rail entry is a detail view of the open project.
    setDetailTab(key);
  }, [closeProject]);

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

  const setPaletteVisibility = useCallback((open: boolean) => {
    setPaletteOpen(open);
    if (!open) window.requestAnimationFrame(() => searchTriggerRef.current?.focus());
  }, []);

  const stopAll = useCallback(() => {
    emergencyStop().then(() => {
      toast(t("misc.stopped"), "success");
      getOperationsSnapshot().then(setSnapshot).catch(() => {});
    }).catch(() => toast(t("misc.stopFail"), "error"));
  }, []);

  const activeNav: NavKey = activeNavFor(projectId, detailTab);
  const badgeTasks = data?.tasks.filter(
    (task) => !["COMPLETED", "FAILED", "CANCELLED"].includes(task.status),
  ).length ?? 0;
  const badgeApprovals = snapshot?.pending_approvals?.length ?? 0;

  const crumbs = projectId && data
    ? [{ label: t("nav.projects"), go: "#projects" }, { label: data.overview.display_name }]
    : [{ label: t("nav.projects") }];

  return (
    <div className="app">
      <Toaster position="bottom-right" />
      <Palette
        open={paletteOpen}
        onOpenChange={setPaletteVisibility}
        projects={projects ?? []}
        projectId={projectId}
        tasks={data?.tasks ?? []}
        onOpenProject={openProject}
        onOpenFile={openFileFromPalette}
        onNavigate={setDetailTab}
        onOpenFolder={openFolder}
        onSwitchLang={switchLanguage}
      />
      <Sidebar
        active={activeNav}
        badgeTasks={badgeTasks}
        badgeApprovals={badgeApprovals}
        snapshot={snapshot}
        collapsed={sidebarCollapsed}
        onToggleCollapse={() => setSidebarCollapsed((c) => !c)}
        onNav={sidebarNav}
        onStop={stopAll}
      />
      <div className="main">
        <Topbar
          crumbs={crumbs}
          onSearch={() => setPaletteOpen(true)}
          searchRef={searchTriggerRef}
          onStop={stopAll}
          lang={lang}
          onSwitch={switchLanguage}
          right={
            <Button size="sm" onClick={openFolder}>
              {t("projects.openFolder")}
            </Button>
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
              tab={detailTab}
              onTabChange={setDetailTab}
              fileRequest={fileRequest}
              onRequestConsumed={() => setFileRequest(null)}
              onProjectRemoved={closeProject}
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
