import { useEffect, useId, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { t } from "../lib/i18n";
import { toast } from "../lib/ui";
import { taskRun } from "../ipc/commands";
import {
  engagementCheckBrowser, engagementCreateTask, engagementSetBrowserOrigins, engagementSnapshot,
} from "../ipc/engagement";
import {
  draftKey, MAX_GOAL_LENGTH, mentionAt, parseDraft, requestedTools, shouldSubmit,
} from "../lib/engagement";
import type { Capability, ComposerDraft, EngagementSnapshot, ToolId } from "../lib/engagement";

interface Props {
  projectId: string;
  onCreated: () => void;
  onSchedule: (goal: string, tools: ToolId[]) => void;
}
function freshDraft(): ComposerDraft {
  return { version: 1, goal: "", tools: ["files"], requestId: crypto.randomUUID() };
}
function readDraft(projectId: string): ComposerDraft {
  try { return parseDraft(localStorage.getItem(draftKey(projectId))) ?? freshDraft(); }
  catch { return freshDraft(); }
}

/** Mount with key={projectId}: drafts and in-flight state cannot cross projects. */
export function TaskComposer({ projectId, onCreated, onSchedule }: Props) {
  const id = useId();
  const textarea = useRef<HTMLTextAreaElement>(null);
  const scopeEditor = useRef<HTMLDetailsElement>(null);
  const scopeInput = useRef<HTMLTextAreaElement>(null);
  const busyRef = useRef(false);
  const mounted = useRef(true);
  const [draft, setDraft] = useState(() => readDraft(projectId));
  const [snapshot, setSnapshot] = useState<EngagementSnapshot | null>(null);
  const [origins, setOrigins] = useState("");
  const [busy, setBusy] = useState(false);
  const [setupBusy, setSetupBusy] = useState(false);
  const [error, setError] = useState("");
  const [draftWarning, setDraftWarning] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [mention, setMention] = useState<{ start: number; query: string } | null>(null);
  const [activeOption, setActiveOption] = useState(0);

  useEffect(() => {
    mounted.current = true;
    let cancelled = false;
    engagementSnapshot(projectId).then((data) => {
      if (cancelled) return;
      setSnapshot(data);
      setOrigins(data.browser_origins.join("\n"));
    }).catch((e) => { if (!cancelled) setError(`${t("engagement.unavailable")} ${String(e)}`); });
    return () => { cancelled = true; mounted.current = false; };
  }, [projectId]);
  useEffect(() => {
    try { localStorage.setItem(draftKey(projectId), JSON.stringify(draft)); setDraftWarning(false); }
    catch { setDraftWarning(true); }
    if (textarea.current) {
      textarea.current.style.height = "auto";
      textarea.current.style.height = `${Math.min(320, Math.max(120, textarea.current.scrollHeight))}px`;
    }
  }, [projectId, draft]);

  const options = (snapshot?.capabilities ?? []).filter((c) => !mention || c.id.startsWith(mention.query));
  const change = (goal: string, tools = draft.tools) => {
    setDraft({ version: 1, goal, tools, requestId: crypto.randomUUID() });
    setError("");
  };
  const updateMention = (goal: string, caret: number) => {
    const next = mentionAt(goal, caret);
    setMention(next);
    setPickerOpen(next !== null);
    setActiveOption(0);
  };
  const selectTool = (capability: Capability) => {
    if (capability.state !== "ready") {
      setError(t(`engagement.reason.${capability.reason}`));
      if (capability.id === "browser") {
        if (scopeEditor.current) scopeEditor.current.open = true;
        scopeInput.current?.focus();
      }
      setPickerOpen(false);
      return;
    }
    const nextTools = requestedTools("", [...draft.tools, capability.id]);
    let goal = draft.goal;
    if (mention && textarea.current) {
      const caret = textarea.current.selectionStart;
      goal = `${goal.slice(0, mention.start)}@${capability.id} ${goal.slice(caret)}`;
    }
    change(goal, nextTools);
    setPickerOpen(false);
    setMention(null);
    requestAnimationFrame(() => textarea.current?.focus());
  };

  const submit = async (run: boolean) => {
    if (busyRef.current) return;
    if (!draft.goal.trim()) { setError(t("engagement.missingGoal")); return; }
    const tools = requestedTools(draft.goal, draft.tools);
    if (!snapshot || tools.some((tool) => !snapshot.capabilities.some((c) => c.id === tool && c.state === "ready"))) {
      setError(t("engagement.toolUnavailable")); return;
    }
    if (run && snapshot.stopped) { setError(t("automations.stopped")); return; }
    busyRef.current = true;
    setBusy(true);
    setError("");
    let createdTaskId = draft.createdTaskId;
    try {
      if (!createdTaskId) {
        const task = await engagementCreateTask(projectId, draft.goal, tools, draft.requestId);
        createdTaskId = task.task_id;
        const saved = { ...draft, tools, createdTaskId };
        // Persist before the separate Run request so a lost response cannot duplicate work.
        try { localStorage.setItem(draftKey(projectId), JSON.stringify(saved)); } catch { /* warning below */ }
        if (mounted.current) setDraft(saved);
        onCreated();
      }
      if (run) await taskRun(createdTaskId);
      if (mounted.current) {
        setDraft(freshDraft());
        toast(t(run ? "engagement.started" : "engagement.saved"), "success");
        onCreated();
        requestAnimationFrame(() => textarea.current?.focus());
      }
    } catch (e) {
      if (mounted.current) setError(`${createdTaskId ? `${t("engagement.createdNotStarted")} ` : ""}${String(e)}`);
    } finally {
      busyRef.current = false;
      if (mounted.current) setBusy(false);
    }
  };
  const browserSetup = async (check: boolean) => {
    if (setupBusy) return;
    setSetupBusy(true);
    setError("");
    try {
      const result = check ? await engagementCheckBrowser(projectId)
        : await engagementSetBrowserOrigins(projectId, origins.split(/[\n,]/).map((s) => s.trim()).filter(Boolean));
      if (mounted.current) { setSnapshot(result); setOrigins(result.browser_origins.join("\n")); }
    } catch (e) { if (mounted.current) setError(String(e)); }
    finally { if (mounted.current) setSetupBusy(false); }
  };

  return (
    <section className="space-y-3" aria-label={t("engagement.goal")}>
      <label className="block font-medium" htmlFor={`${id}-goal`}>{t("engagement.goal")}</label>
      <div className="relative">
        <textarea
          id={`${id}-goal`} ref={textarea} value={draft.goal} rows={4}
          maxLength={MAX_GOAL_LENGTH} disabled={busy}
          className="min-h-30 w-full resize-none overflow-y-auto rounded-xl border border-input bg-background px-4 py-3 text-base leading-6 text-foreground outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30 disabled:opacity-50"
          placeholder={t("engagement.placeholder")}
          role="combobox" aria-autocomplete="list" aria-expanded={pickerOpen}
          aria-controls={pickerOpen ? `${id}-tools` : undefined}
          aria-activedescendant={pickerOpen && options.length ? `${id}-tool-${Math.min(activeOption, options.length - 1)}` : undefined}
          aria-describedby={`${id}-hint`}
          onChange={(e) => { change(e.target.value); updateMention(e.target.value, e.target.selectionStart); }}
          onClick={(e) => updateMention(e.currentTarget.value, e.currentTarget.selectionStart)}
          onKeyDown={(e) => {
            if (e.nativeEvent.isComposing || e.keyCode === 229) return;
            if (pickerOpen && e.key === "Escape") { e.preventDefault(); setPickerOpen(false); return; }
            if (pickerOpen && options.length && ["ArrowDown", "ArrowUp", "Enter"].includes(e.key)) {
              e.preventDefault();
              if (e.key === "Enter") selectTool(options[Math.min(activeOption, options.length - 1)]);
              else setActiveOption((i) => (i + (e.key === "ArrowDown" ? 1 : -1) + options.length) % options.length);
              return;
            }
            if (shouldSubmit({ key: e.key, metaKey: e.metaKey, ctrlKey: e.ctrlKey,
              isComposing: e.nativeEvent.isComposing, pickerOpen })) { e.preventDefault(); void submit(true); }
          }}
        />
        {pickerOpen && (
          <div id={`${id}-tools`} role="listbox" aria-label={t("engagement.tools")}
            className="absolute inset-x-0 top-full z-40 mt-1 max-h-72 overflow-y-auto rounded-xl border border-border bg-popover p-2 shadow-lg">
            {options.map((capability, index) => (
              <button type="button" key={capability.id} id={`${id}-tool-${index}`} role="option"
                aria-selected={index === activeOption} aria-disabled={capability.state !== "ready"}
                className={`block w-full rounded-lg px-3 py-2 text-left ${index === activeOption ? "bg-accent" : ""}`}
                onMouseDown={(e) => e.preventDefault()} onClick={() => selectTool(capability)}>
                <span className="font-medium">@{capability.id} · {t(`engagement.${capability.id}`)}</span>
                <span className="block text-sm text-muted-foreground">{t(`engagement.reason.${capability.reason}`)}</span>
              </button>
            ))}
            {!snapshot && <p className="p-3 text-sm">{t("engagement.wait")}</p>}
          </div>
        )}
      </div>
      <div className="flex flex-wrap gap-2" aria-label={t("engagement.tools")}>
        {draft.tools.map((tool) => (
          <span key={tool} className="chip inline-flex items-center gap-1">
            {t(`engagement.${tool}`)}
            {tool !== "files" && <Button type="button" variant="ghost" size="sm" disabled={busy}
              aria-label={`${t("engagement.remove")}: ${tool}`}
              onClick={() => change(draft.goal, draft.tools.filter((x) => x !== tool))}>×</Button>}
          </span>
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button type="button" variant="secondary" size="sm" disabled={busy}
          onClick={() => { setMention(null); setPickerOpen(!pickerOpen); setActiveOption(0); textarea.current?.focus(); }}>
          @ {t("engagement.tools")}
        </Button>
        <span className="flex-1" />
        <Button type="button" variant="ghost" size="sm" disabled={busy || !draft.goal.trim()} onClick={() => void submit(false)}>{t("engagement.save")}</Button>
        <Button type="button" variant="secondary" size="sm" disabled={busy || !draft.goal.trim()}
          onClick={() => onSchedule(draft.goal, requestedTools(draft.goal, draft.tools))}>{t("engagement.schedule")}</Button>
        <Button type="button" size="sm" disabled={busy || !snapshot || snapshot.stopped || !draft.goal.trim()}
          onClick={() => void submit(true)}>{t(busy ? "engagement.submitting" : "engagement.run")}</Button>
      </div>
      <p id={`${id}-hint`} className="muted small">{t("engagement.hint")}</p>
      {draftWarning && <p role="status" className="small">{t("engagement.draftUnavailable")}</p>}
      {error && <p role="alert" className="rounded-lg border border-destructive/40 p-3 text-sm whitespace-pre-wrap">{error}</p>}
      <details ref={scopeEditor} className="rounded-lg border border-border p-3">
        <summary className="cursor-pointer text-sm font-medium">{t("engagement.scope")}</summary>
        <div className="mt-3 space-y-2">
          <p className="muted small">{t("engagement.scopeHint")}</p>
          <label className="block text-sm" htmlFor={`${id}-origins`}>{t("engagement.origins")}</label>
          <textarea id={`${id}-origins`} ref={scopeInput} rows={2} value={origins} disabled={setupBusy || busy}
            className="w-full rounded-lg border border-input bg-background p-3 text-base"
            placeholder={t("engagement.origins")} onChange={(e) => setOrigins(e.target.value)} />
          <div className="flex flex-wrap gap-2">
            <Button type="button" variant="secondary" size="sm" disabled={setupBusy || busy} onClick={() => void browserSetup(false)}>{t("engagement.saveScope")}</Button>
            <Button type="button" variant="secondary" size="sm" disabled={setupBusy || busy} onClick={() => void browserSetup(true)}>{t("engagement.checkBrowser")}</Button>
          </div>
        </div>
      </details>
    </section>
  );
}
