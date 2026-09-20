/* Files tab — real project file access through the bounded file
   commands: browse (file_list), read (file_read), edit with stale-write
   protection (file_edit + expected sha256), create, delete, and search
   (file_search, filename or text mode). Nothing here is decorative:
   every mutation goes through the project service's boundary checks. */
import { lazy, Suspense, useCallback, useEffect, useRef, useState } from "react";
import { Glyph } from "../components/Icons";
import { t } from "../lib/i18n";
import { fmtSize, toast } from "../lib/ui";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent,
  AlertDialogDescription, AlertDialogTitle, AlertDialogTrigger,
} from "@/components/ui/overlays";
import {
  fileCreate, fileDelete, fileEdit, fileList, fileRead, fileSearch,
} from "../ipc/commands";
import { DocumentPreview, previewKindFor } from "./DocumentPreview";

const CodeEditor = lazy(() => import("./CodeEditor"));
import type { ListedEntry, SearchHit } from "../ipc/types";

export interface FileRequest {
  path: string;
  nonce: number;
}

interface Props {
  projectId: string;
  request: FileRequest | null;
  onRequestConsumed: () => void;
  onMutated: () => void;
}

function parentDir(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx === -1 ? "" : path.slice(0, idx);
}

export function FilesTab({ projectId, request, onRequestConsumed, onMutated }: Props) {
  const [cwd, setCwd] = useState("");
  const [entries, setEntries] = useState<ListedEntry[] | null>(null);
  const [selected, setSelected] = useState<{ path: string; content: string; sha256: string } | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  // Spec 30 phase A: md/csv render read-only previews; editing requires
  // switching to the source editor (Edit button), which stays available.
  const previewable = selected
    ? previewKindFor(selected.path, true) !== "text" || selected.path.toLowerCase().endsWith(".md")
    : false;
  const [newName, setNewName] = useState("");
  const [query, setQuery] = useState("");
  const [mode, setMode] = useState<"filename" | "text">("filename");
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const requestRef = useRef(request);
  requestRef.current = request;

  const openPath = useCallback((path: string, isDir: boolean) => {
    if (isDir) {
      setCwd(path);
      return;
    }
    fileRead(projectId, path).then((fc) => {
      setSelected({ path: fc.path, content: fc.content, sha256: fc.sha256 });
      setEditing(false);
      setDraft(fc.content);
    }).catch(() => toast(t("files.saveFail"), "error"));
  }, [projectId]);

  const refresh = useCallback((dir: string) => {
    fileList(projectId, dir || ".").then(setEntries).catch(() => setEntries([]));
  }, [projectId]);

  useEffect(() => {
    refresh(cwd);
  }, [cwd, refresh]);

  // A palette hit (or other navigation) requests a specific file.
  useEffect(() => {
    if (!request) return;
    const { path } = request;
    setCwd(parentDir(path));
    openPath(path, false);
    onRequestConsumed();
  }, [request, openPath, onRequestConsumed]);

  const saveEdit = () => {
    if (!selected) return;
    fileEdit(projectId, selected.path, selected.sha256, draft).then(() => {
      toast(t("files.saved"), "success");
      setEditing(false);
      setSelected({ ...selected, content: draft });
      refresh(cwd);
      onMutated();
    }).catch(() => toast(t("files.saveFail"), "error"));
  };

  const createFile = () => {
    const path = newName.trim();
    if (!path) return;
    fileCreate(projectId, path, "").then(() => {
      setNewName("");
      toast(t("files.saved"), "success");
      refresh(cwd);
      onMutated();
      openPath(path, false);
    }).catch(() => toast(t("files.saveFail"), "error"));
  };

  const deleteSelected = () => {
    if (!selected) return;
    fileDelete(projectId, selected.path, selected.sha256).then(() => {
      toast(t("files.saved"), "success");
      setSelected(null);
      refresh(cwd);
      onMutated();
    }).catch(() => toast(t("files.saveFail"), "error"));
  };

  const runSearch = () => {
    const q = query.trim();
    if (!q) { setHits(null); return; }
    fileSearch(projectId, q, mode).then((results) => setHits(results)).catch(() => setHits([]));
  };

  const segments = cwd ? cwd.split("/") : [];

  return (
    <div className="files-layout">
      <div className="card" style={{ minWidth: 0 }}>
        <div className="inline-form">
          <Button variant="secondary" size="icon-sm" aria-label={t("files.up")}
            disabled={!cwd} onClick={() => setCwd(cwd.includes("/") ? cwd.slice(0, cwd.lastIndexOf("/")) : "")}>
            <Glyph name="chevronLeft" />
          </Button>
          <span className="mono small" style={{ flex: 1, minWidth: 0 }}>
            /{segments.join(" / ") || t("files.title")}
          </span>
        </div>

        <div className="inline-form">
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={mode === "filename" ? t("files.searchPlaceholder") : t("files.searchContent")}
            onKeyDown={(e) => e.key === "Enter" && runSearch()}
          />
          <Button variant="secondary" size="sm" onClick={() => setMode(mode === "filename" ? "text" : "filename")}>
            {mode === "filename" ? t("files.fileName") : t("files.textContent")}
          </Button>
          <Button size="sm" onClick={runSearch}>{t("files.searchButton")}</Button>
        </div>

        {hits !== null && (
          <div className="mini-list">
            {hits.length === 0 && <div className="empty-state">{t("palette.noMatches")}</div>}
            {hits.map((hit) => (
              <button key={`${hit.path}:${hit.line ?? 0}`} className="mini-row" style={{ cursor: "pointer", textAlign: "left" }}
                onClick={() => openPath(hit.path, false)}>
                <Glyph name="fileText" />
                <span style={{ flex: 1, minWidth: 0 }} className="mono small">{hit.path}{hit.line ? `:${hit.line}` : ""}</span>
                {hit.snippet && <span className="muted small" style={{ maxWidth: "45%", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{hit.snippet}</span>}
              </button>
            ))}
          </div>
        )}

        <div className="mini-list">
          {entries === null && <div className="muted small">{t("misc.loading")}</div>}
          {entries !== null && entries.length === 0 && <div className="empty-state">{t("files.emptyDir")}</div>}
          {entries !== null && entries.map((e) => (
            <button key={e.path} className="mini-row" style={{ cursor: "pointer", textAlign: "left" }}
              onClick={() => openPath(e.path, e.is_dir)}>
              <Glyph name={e.is_dir ? "folder" : "file"} />
              <span style={{ flex: 1, minWidth: 0 }} className="mono small">{e.name}{e.is_dir ? "/" : ""}</span>
              {!e.is_dir && e.size != null && <span className="muted small">{fmtSize(e.size)}</span>}
            </button>
          ))}
        </div>

        <div className="inline-form">
          <Input
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder={t("files.newFilePlaceholder")}
            onKeyDown={(e) => e.key === "Enter" && createFile()}
          />
          <Button variant="secondary" size="sm" onClick={createFile}>{t("files.newFile")}</Button>
        </div>
      </div>

      <div className="card" style={{ minWidth: 0 }}>
        {!selected ? (
          <div className="empty-state">{t("files.selectFile")}</div>
        ) : (
          <>
            <div className="inline-form">
              <span style={{ flex: 1, minWidth: 0 }} className="mono small">{selected.path}</span>
              <Button variant="secondary" size="icon-sm" title={t("files.copyPath")} aria-label={t("files.copyPath")}
                onClick={() => navigator.clipboard?.writeText(selected.path).then(
                  () => toast(t("files.copyOk"), "success"),
                  () => toast(t("files.copyFail"), "error"),
                )}>
                <Glyph name="copy" />
              </Button>
              {!editing && (
                <Button variant="secondary" size="sm" onClick={() => { setEditing(true); setDraft(selected.content); }}>
                  {t("files.edit")}
                </Button>
              )}
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button variant="destructive" size="sm">{t("files.delete")}</Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogTitle>{t("files.delete")}</AlertDialogTitle>
                  <AlertDialogDescription>{t("files.deleteConfirm")}</AlertDialogDescription>
                  <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
                    <AlertDialogCancel asChild>
                      <Button variant="secondary" size="sm">{t("files.cancel")}</Button>
                    </AlertDialogCancel>
                    <AlertDialogAction asChild>
                      <Button variant="destructive" size="sm" onClick={deleteSelected}>
                        {t("files.delete")}
                      </Button>
                    </AlertDialogAction>
                  </div>
                </AlertDialogContent>
              </AlertDialog>
            </div>
            <p className="muted small">{t("files.contextGuard")}</p>
            {editing ? (
              <>
                {/* Spec 30.7: CodeMirror editing surface — undo/redo
                    history and the Mod-f find/replace panel. */}
                <Suspense fallback={<div className="muted small">{t("misc.loading")}</div>}>
                  <CodeEditor
                    value={draft}
                    path={selected.path}
                    onChange={setDraft}
                  />
                </Suspense>
                <div className="inline-form">
                  <Button variant="secondary" size="sm" onClick={() => { setEditing(false); setDraft(selected.content); }}>
                    {t("files.cancel")}
                  </Button>
                  <Button size="sm" onClick={saveEdit}>{t("files.save")}</Button>
                </div>
              </>
            ) : previewable ? (
              <DocumentPreview path={selected.path} project={projectId} content={selected.content} />
            ) : (
              <pre className="code-body" style={{ maxHeight: 420, overflow: "auto" }}>{selected.content}</pre>
            )}
          </>
        )}
      </div>
    </div>
  );
}
