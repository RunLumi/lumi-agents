import { createContext, useContext, useState, type ReactNode } from "react";

export interface Route {
  view: "projects" | "project";
  projectId?: string;
  tab?: string;
  taskId?: string;
}

const Ctx = createContext<{ route: Route; refreshKey: number }>({ route: { view: "projects" }, refreshKey: 0 });

export function StoreProvider({ children, route }: { children: ReactNode; route: Route }) {
  const [refreshKey] = useState(0);
  return <Ctx.Provider value={{ route, refreshKey }}>{children}</Ctx.Provider>;
}

export function useRoute(): Route {
  return useContext(Ctx).route;
}
