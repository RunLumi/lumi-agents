/* The ⌘K command palette — search across projects, tasks, and project
   files, plus quick actions. File hits come from the backend's bounded
   filename search (file_search); selecting one opens it in the Files
   tab. Palette i18n keys: palette.* (§ lib/i18n). */
import { useEffect, useRef, useState } from "react";
import { Glyph } from "./Icons";
import { t } from "../lib/i18n";
import type { Lang } from "../lib/i18n";
import { fileSearch } from "../ipc/commands";
import type { ProjectSummary, SearchHit, Task } from "../ipc/types";
import {
  CommandDialog, CommandEmpty, CommandGroup, CommandInput,
  CommandItem, CommandList, CommandSeparator,
} from "@/components/ui/command";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projects: ProjectSummary[];
  projectId: string | null;
  tasks: Task[];
  onOpenProject: (projectId: string) => void;
  onOpenFile: (path: string) => void;
  onNavigate: (tab: string) => void;
  onOpenFolder: () => void;
  onSwitchLang: (lang: Lang) => void;
}

const FILE_SEARCH_MIN = 2;
const FILE_SEARCH_DEBOUNCE_MS = 200;

export function Palette({
  open, onOpenChange, projects, projectId, tasks,
  onOpenProject, onOpenFile, onNavigate, onOpenFolder, onSwitchLang,
}: Props) {
  const [fileHits, setFileHits] = useState<SearchHit[]>([]);
  const [query, setQuery] = useState("");
  const debounce = useRef<number | undefined>(undefined);
  const queryRef = useRef("");
  queryRef.current = query;

  // Live file search once there is an open project and enough to search on.
  useEffect(() => {
    if (!open || !projectId || query.trim().length < FILE_SEARCH_MIN) {
      setFileHits([]);
      return;
    }
    window.clearTimeout(debounce.current);
    debounce.current = window.setTimeout(() => {
      const q = queryRef.current;
      fileSearch(projectId, q, "filename").then((hits) => {
        if (queryRef.current === q) setFileHits(hits.slice(0, 8));
      }).catch(() => setFileHits([]));
    }, FILE_SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(debounce.current);
  }, [query, open, projectId]);

  const run = (action: () => void) => () => {
    onOpenChange(false);
    action();
  };

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("palette.title")}
      description={t("palette.description")}
    >
      <CommandInput
        value={query}
        onValueChange={setQuery}
        placeholder={t("palette.placeholder")}
        aria-label={t("palette.placeholder")}
      />
      <CommandList>
        <CommandEmpty>{t("palette.noMatches")}</CommandEmpty>

        {projectId && (
          <CommandGroup heading={t("palette.files")}>
            {fileHits.map((hit) => (
              <CommandItem key={hit.path} value={`file ${hit.path}`} onSelect={run(() => onOpenFile(hit.path))}>
                <Glyph name="file" />
                <span className="mono flex-1 truncate">{hit.path}</span>
                <span className="text-muted-foreground text-xs">{t("palette.jumpTo")}</span>
              </CommandItem>
            ))}
          </CommandGroup>
        )}

        {tasks.length > 0 && (
          <CommandGroup heading={t("palette.tasks")}>
            {tasks.slice(0, 5).map((task) => (
              <CommandItem key={task.task_id} value={`task ${task.goal}`} onSelect={run(() => onNavigate("tasks"))}>
                <Glyph name="task" />
                <span className="flex-1 truncate">{task.goal}</span>
              </CommandItem>
            ))}
          </CommandGroup>
        )}

        {projects.length > 0 && (
          <CommandGroup heading={t("palette.projects")}>
            {projects.map((p) => (
              <CommandItem key={p.project_id} value={`project ${p.display_name}`} onSelect={run(() => onOpenProject(p.project_id))}>
                <Glyph name="folder" />
                <span className="flex-1 truncate">{p.display_name}</span>
              </CommandItem>
            ))}
          </CommandGroup>
        )}

        <CommandSeparator />
        <CommandGroup heading={t("palette.all")}>
          <CommandItem value="action open folder" onSelect={run(onOpenFolder)}>
            <Glyph name="folder" /> {t("home.openFolder")}
          </CommandItem>
          <CommandItem value="action language english" onSelect={run(() => onSwitchLang("en"))}>
            {t("misc.language")}: EN
          </CommandItem>
          <CommandItem value="action language vietnamese" onSelect={run(() => onSwitchLang("vi"))}>
            {t("misc.language")}: VI
          </CommandItem>
        </CommandGroup>
      </CommandList>
      <div className="border-t border-[var(--color-border)] px-4 py-2 text-muted-foreground text-xs">
        {t("palette.protip")}: {t("palette.protipBody")}
      </div>
    </CommandDialog>
  );
}
