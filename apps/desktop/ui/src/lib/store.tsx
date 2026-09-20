/* Typed central store — the React port of the vanilla S block.
   Views are pure functions of this state; loaders live here. */
import { createContext, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import * as api from "../ipc/commands";
import { getTransport } from "../ipc/transport";
import type {
  ArtifactEntry, ChangeSet, GitStatus, OperationsSnapshot,
  ProjectOverview, ProjectSummary, Task,
} from "../ipc/types";
import { getTransport } from "../ipc/transport";

export interface Route {
  view: "projects" | "project";
  projectId?: string;
  tab?: string;
  taskId?: string;
}

export interface OpenFile {
  path: string;
  content: string;
  sha256: string;
}

export interface AppState {
  route: Route;
  projects: ProjectSummary[] | null;
  project: ProjectOverview | null;
  summary: ProjectSummary | null;
  git: GitStatus | null;
  tasks: Task[];
  changeSets: ChangeSet[];
  artifacts: ArtifactEntry[];
  snapshot: OperationsSnapshot | null;
  openFiles: OpenFile[];
  activeFile: string | null;
  collapsed: boolean;
  refreshKey: number;
}

const initialState: AppState = {
  route: { view: "projects" },
  projects: null,
  project: null,
  summary: null,
  git: null,
  tasks: [],
  changeSets: [],
  artifacts: [],
  snapshot: null,
  openFiles: [],
  activeFile: null,
  collapsed: false,
  refreshKey: 0,
};

interface Store extends AppState {
  set: (patch: Partial<AppState>) => void;
  refresh: () => Promise<void>;
  reloadProject: (projectId: string) => Promise<void>;
}

const StoreContext = createContext<Store | null>(null);

export function parseRoute(hash: string): Route {
  const parts = hash.replace(/^#\/?/, "").split("/").filter(Boolean);
  if (parts[0] === "p" && parts[1]) {
    return {
      view: "project",
      projectId: decodeURIComponent(parts[1]),
      tab: parts[2] || "home",
      taskId: parts[3] ? decodeURIComponent(parts[3]) : undefined,
    };
  }
  return { view: (parts[0] as Route["view"]) || "projects" };
}

export function StoreProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AppState>(initialState);

  const set = (patch: Partial<AppState>) => setState((s: AppState) => ({ ...s, ...patch }));

  const store = useMemo<Store>(() => ({
    ...state,
    set,
    async refresh() {
      set((s) => ({ ...s, refreshKey: s.refreshKey + 1 }));
    },
    async reloadProject(projectId: string) {
      const [overview, git, tasks, changeSets, artifacts] = await Promise.all([
        api.projectOverview(projectId),
        api.gitStatus(projectId).catch(() => null),
        api.taskList(projectId).catch(() => [] as Task[]),
        api.changeSets(projectId).catch(() => []),
        api.artifactsList(projectId).catch(() => []),
      ]);
      set({
        project: overview,
        git: git ?? null,
        tasks: tasks ?? [],
        changeSets: changeSets ?? [],
        artifacts: artifacts ?? [],
      });
    },
  }), [state]);

  return <StoreContext.Provider value={store}>{children}</StoreContext.Provider>;
}

export function useStore(): Store {
  const store = useContext(StoreContext);
  if (!store) throw new Error("useStore outside StoreProvider");
  return store;
}

/** Loads the operations snapshot; returns null when the runtime is away. */
export async function loadSnapshot(): Promise<OperationsSnapshot | null> {
  try {
    return await getTransport().invoke<OperationsSnapshot>("get_operations_snapshot");
  } catch {
    return null;
  }
}
