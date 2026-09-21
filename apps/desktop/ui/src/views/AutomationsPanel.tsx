import { useCallback, useEffect, useId, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { StatusBadge } from "@/components/ui/badge";
import { t } from "../lib/i18n";
import { toast } from "../lib/ui";
import {
  automationDelete, automationList, automationPreview, automationRunNow, automationSave, automationToggle,
} from "../ipc/engagement";
import { MAX_GOAL_LENGTH, requestedTools } from "../lib/engagement";
import type {
  Automation, AutomationInput, AutomationList, AutomationSeed, ScheduleSpec,
} from "../lib/engagement";

interface Props {
  projectId: string;
  seed: AutomationSeed | null;
  onSeedConsumed: () => void;
  onChanged: () => void;
  onOpenTasks: () => void;
}
const nowSeconds = () => Math.floor(Date.now() / 1000);
function newInput(): AutomationInput {
  return { name: "", goal: "", tools: ["files"], schedule: { kind: "daily", time: "08:00" },
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC", catch_up: "run_once",
    enabled: false, authorization_until: nowSeconds() + 14 * 86400 };
}
function editable(item: Automation): AutomationInput {
  const { automation_id, revision, name, goal, tools, schedule, timezone, catch_up, enabled, authorization_until } = item;
  return { automation_id, revision, name, goal, tools: [...tools], schedule: { ...schedule }, timezone,
    catch_up, enabled, authorization_until };
}
function localInput(epoch: number | null): string {
  if (epoch === null) return "";
  const date = new Date(epoch * 1000);
  return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
}
function displayTime(epoch: number | null, timezone: string): string {
  if (epoch === null) return "—";
  try { return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short", timeZone: timezone }).format(epoch * 1000); }
  catch { return new Date(epoch * 1000).toISOString(); }
}
const controlClass = "min-h-10 w-full rounded-lg border border-input bg-background px-3 py-2 text-base text-foreground";

export function AutomationsPanel({ projectId, seed, onSeedConsumed, onChanged, onOpenTasks }: Props) {
  const id = useId();
  const mounted = useRef(true);
  const busyRef = useRef(false);
  const runRequests = useRef(new Map<string, string>());
  const [data, setData] = useState<AutomationList | null>(null);
  const [form, setForm] = useState<AutomationInput | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<number[]>([]);
  const [previewError, setPreviewError] = useState("");
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const nameInput = useRef<HTMLInputElement>(null);
  const refresh = useCallback(async () => {
    try {
      const next = await automationList(projectId);
      if (mounted.current) setData(next);
    } catch (e) { if (mounted.current) setError(String(e)); }
  }, [projectId]);
  useEffect(() => {
    mounted.current = true;
    void refresh();
    const timer = window.setInterval(() => void refresh(), 15_000);
    return () => { mounted.current = false; window.clearInterval(timer); };
  }, [refresh]);
  useEffect(() => {
    if (!seed) return;
    setForm({ ...newInput(), goal: seed.goal, tools: [...seed.tools] });
    onSeedConsumed();
    requestAnimationFrame(() => nameInput.current?.focus());
  }, [seed, onSeedConsumed]);
  useEffect(() => {
    let cancelled = false;
    setPreview([]);
    setPreviewError("");
    if (!form) return;
    const timer = window.setTimeout(() => {
      automationPreview(form).then((values) => { if (!cancelled) setPreview(values); })
        .catch((e) => { if (!cancelled) setPreviewError(String(e)); });
    }, 300);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [form]);
  const update = (patch: Partial<AutomationInput>) => setForm((current) => current ? { ...current, ...patch } : current);
  const operation = async (action: () => Promise<unknown>) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError("");
    try { await action(); await refresh(); onChanged(); }
    catch (e) { if (mounted.current) setError(String(e)); }
    finally { busyRef.current = false; if (mounted.current) setBusy(false); }
  };
  const save = () => {
    if (!form || !form.name.trim() || !form.goal.trim()) { setError(t("automations.validation")); return; }
    const input = { ...form, tools: requestedTools(form.goal, form.tools) };
    void operation(async () => {
      await automationSave(projectId, input);
      if (mounted.current) { setForm(null); toast(t("automations.saved"), "success"); }
    });
  };
  const setKind = (kind: ScheduleSpec["kind"]) => {
    const schedule: ScheduleSpec = kind === "once" ? { kind, local_datetime: "" }
      : kind === "every" ? { kind, minutes: 240 }
      : kind === "weekly" ? { kind, time: "08:00", weekdays: [1, 2, 3, 4, 5] }
      : { kind, time: "08:00" };
    update({ schedule });
  };

  return (
    <section className="space-y-4" aria-label={t("nav.automations")}>
      <div className="flex flex-wrap items-start gap-3">
        <div className="min-w-0 flex-1"><h2>{t("automations.title")}</h2><p className="muted small">{t("automations.subtitle")}</p></div>
        <Button size="sm" disabled={busy || !data?.scheduler_available} onClick={() => { setForm(newInput()); requestAnimationFrame(() => nameInput.current?.focus()); }}>{t("automations.new")}</Button>
      </div>
      <div className="card space-y-2">
        <p className="small">{t("automations.local")}</p>
        {data && !data.provider_configured && <p role="status" className="small">{t("automations.provider")}</p>}
        {data?.stopped && <p role="status" className="small">{t("automations.stopped")}</p>}
        {data && !data.scheduler_available && <p role="alert">{t("automations.unavailable")}</p>}
      </div>
      {error && <p role="alert" className="rounded-lg border border-destructive/40 p-3 text-sm whitespace-pre-wrap">{error}</p>}
      {form && (
        <form className="card space-y-4" onSubmit={(e) => { e.preventDefault(); save(); }}>
          <div>
            <label htmlFor={`${id}-name`} className="mb-1 block text-sm">{t("automations.name")}</label>
            <Input ref={nameInput} id={`${id}-name`} value={form.name} maxLength={120} required disabled={busy} onChange={(e) => update({ name: e.target.value })} />
          </div>
          <div>
            <label htmlFor={`${id}-goal`} className="mb-1 block text-sm">{t("automations.instructions")}</label>
            <textarea id={`${id}-goal`} rows={5} className={controlClass} value={form.goal} required maxLength={MAX_GOAL_LENGTH}
              disabled={busy} onChange={(e) => update({ goal: e.target.value })} />
          </div>
          <fieldset className="space-y-2" disabled={busy}>
            <legend className="text-sm font-medium">{t("engagement.tools")}</legend>
            <div className="flex flex-wrap gap-4">
              <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked disabled />{t("engagement.files")}</label>
              <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.tools.includes("browser")}
                onChange={(e) => update({ tools: e.target.checked ? [...new Set([...form.tools, "browser" as const])] : form.tools.filter((x) => x !== "browser") })} />{t("engagement.browser")}</label>
              {form.tools.filter((x) => !["files", "browser"].includes(x)).map((tool) => (
                <Button key={tool} type="button" variant="secondary" size="sm" onClick={() => update({ tools: form.tools.filter((x) => x !== tool) })}>{t("engagement.remove")}: @{tool}</Button>
              ))}
            </div>
            <p className="muted small">{t("automations.permissions")}</p>
          </fieldset>
          <div className="grid gap-3 sm:grid-cols-2">
            <div>
              <label htmlFor={`${id}-kind`} className="mb-1 block text-sm">{t("automations.schedule")}</label>
              <select id={`${id}-kind`} className={controlClass} value={form.schedule.kind} disabled={busy} onChange={(e) => setKind(e.target.value as ScheduleSpec["kind"])}>
                {(["once", "every", "daily", "weekly"] as const).map((kind) => <option key={kind} value={kind}>{t(`automations.${kind}`)}</option>)}
              </select>
            </div>
            <div>
              <label htmlFor={`${id}-timezone`} className="mb-1 block text-sm">{t("automations.timezone")}</label>
              <Input id={`${id}-timezone`} value={form.timezone} required disabled={busy} onChange={(e) => update({ timezone: e.target.value })} />
            </div>
            {form.schedule.kind === "once" && <div>
              <label htmlFor={`${id}-once`} className="mb-1 block text-sm">{t("automations.dateTime")}</label>
              <Input id={`${id}-once`} type="datetime-local" value={form.schedule.local_datetime} required disabled={busy}
                onChange={(e) => update({ schedule: { kind: "once", local_datetime: e.target.value } })} />
            </div>}
            {form.schedule.kind === "every" && <div>
              <label htmlFor={`${id}-every`} className="mb-1 block text-sm">{t("automations.minutes")}</label>
              <Input id={`${id}-every`} type="number" min={5} max={525600} value={form.schedule.minutes} required disabled={busy}
                onChange={(e) => update({ schedule: { kind: "every", minutes: Number(e.target.value) } })} />
            </div>}
            {(form.schedule.kind === "daily" || form.schedule.kind === "weekly") && <div>
              <label htmlFor={`${id}-time`} className="mb-1 block text-sm">{t("automations.time")}</label>
              <Input id={`${id}-time`} type="time" value={form.schedule.time} required disabled={busy}
                onChange={(e) => update({ schedule: { ...form.schedule, time: e.target.value } as ScheduleSpec })} />
            </div>}
            <div>
              <label htmlFor={`${id}-catchup`} className="mb-1 block text-sm">{t("automations.catchUp")}</label>
              <select id={`${id}-catchup`} className={controlClass} value={form.catch_up} disabled={busy}
                onChange={(e) => update({ catch_up: e.target.value as "run_once" | "skip" })}>
                <option value="run_once">{t("automations.runOnce")}</option><option value="skip">{t("automations.skip")}</option>
              </select>
            </div>
          </div>
          {form.schedule.kind === "weekly" && <fieldset disabled={busy}>
            <legend className="mb-2 text-sm">{t("automations.weekdays")}</legend>
            <div className="flex flex-wrap gap-4">{[1, 2, 3, 4, 5, 6, 7].map((day) => (
              <label key={day} className="flex items-center gap-2 text-sm"><input type="checkbox"
                checked={form.schedule.kind === "weekly" && form.schedule.weekdays.includes(day)}
                onChange={(e) => {
                  if (form.schedule.kind !== "weekly") return;
                  update({ schedule: { ...form.schedule, weekdays: e.target.checked
                    ? [...new Set([...form.schedule.weekdays, day])].sort() : form.schedule.weekdays.filter((x) => x !== day) } });
                }} />{day}</label>
            ))}</div>
          </fieldset>}
          <div className="space-y-2 rounded-lg border border-border p-3">
            <label className="flex items-start gap-2 text-sm"><input type="checkbox" checked={form.enabled} disabled={busy}
              onChange={(e) => update({ enabled: e.target.checked })} />{t("automations.authorize")}</label>
            <label htmlFor={`${id}-until`} className="block text-sm">{t("automations.until")}</label>
            <Input id={`${id}-until`} type="datetime-local" value={localInput(form.authorization_until)} disabled={busy}
              onChange={(e) => { const epoch = new Date(e.target.value).getTime() / 1000; update({ authorization_until: Number.isFinite(epoch) ? Math.floor(epoch) : null }); }} />
          </div>
          <div className="space-y-1 text-sm" aria-live="polite">
            <strong>{t("automations.preview")}</strong>
            {preview.map((epoch) => <p key={epoch}>{displayTime(epoch, form.timezone)} · {form.timezone}</p>)}
            {previewError && <p>{previewError}</p>}
          </div>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="secondary" size="sm" disabled={busy} onClick={() => setForm(null)}>{t("automations.cancel")}</Button>
            <Button type="submit" size="sm" disabled={busy || !data?.scheduler_available || !!previewError}>{t(busy ? "automations.saving" : "automations.save")}</Button>
          </div>
        </form>
      )}
      {!data && !error && <p role="status">{t("automations.loading")}</p>}
      {data?.items.length === 0 && <div className="card empty-state">{t("automations.empty")}</div>}
      {data?.items.map((item) => {
        const expired = item.authorization_until === null || item.authorization_until <= nowSeconds();
        return (
          <article key={item.automation_id} className="card space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="min-w-0 flex-1">{item.name}</h3>
              <StatusBadge variant={item.enabled && expired ? "status-overdue" : "status-open"}>
                {t(item.enabled ? expired ? "automations.expired" : "automations.enabled" : "automations.paused")}
              </StatusBadge>
            </div>
            <p className="whitespace-pre-wrap text-sm">{item.goal}</p>
            <p className="muted small">{t("automations.next")}: {displayTime(item.next_run_at, item.timezone)} · {item.timezone}</p>
            <div className="flex flex-wrap gap-2">
              <Button size="sm" disabled={busy || !data.provider_configured || data.stopped} onClick={() => void operation(async () => {
                const request = runRequests.current.get(item.automation_id) ?? crypto.randomUUID();
                runRequests.current.set(item.automation_id, request);
                await automationRunNow(projectId, item.automation_id, request);
                runRequests.current.delete(item.automation_id);
              })}>{t("automations.runNow")}</Button>
              <Button variant="secondary" size="sm" disabled={busy} onClick={() => void operation(async () => {
                if (!item.enabled && expired) { setForm(editable(item)); return; }
                await automationToggle(projectId, item.automation_id, item.revision, !item.enabled);
              })}>{t(item.enabled ? "automations.pause" : "automations.enable")}</Button>
              <Button variant="ghost" size="sm" disabled={busy} onClick={() => setForm(editable(item))}>{t("automations.edit")}</Button>
              <Button variant="ghost" size="sm" disabled={busy} onClick={() => setDeleteId(item.automation_id)}>{t("automations.remove")}</Button>
            </div>
            {deleteId === item.automation_id && <div role="group" aria-label={t("automations.remove")} className="space-y-2 rounded-lg border border-border p-3">
              <p className="text-sm">{t("automations.confirmDelete")}</p>
              <div className="flex gap-2"><Button variant="secondary" size="sm" disabled={busy} onClick={() => setDeleteId(null)}>{t("automations.cancel")}</Button>
                <Button variant="destructive" size="sm" disabled={busy} onClick={() => void operation(async () => {
                  await automationDelete(projectId, item.automation_id, item.revision); setDeleteId(null);
                })}>{t("automations.remove")}</Button></div>
            </div>}
            <details><summary className="cursor-pointer text-sm font-medium">{t("automations.history")} ({item.runs.length})</summary>
              {!item.runs.length && <p className="muted small mt-2">{t("automations.noRuns")}</p>}
              <div className="mt-2 space-y-2">{[...item.runs].reverse().map((run) => <div key={run.occurrence_id} className="rounded-lg border border-border p-3 text-sm">
                <div className="flex flex-wrap items-center gap-2"><span className="flex-1">{displayTime(run.scheduled_at, item.timezone)}</span><code>{run.status}</code></div>
                {run.task_id && <Button variant="ghost" size="sm" onClick={onOpenTasks}>{t("automations.task")}: {run.task_id}</Button>}
                {run.note && <p className="muted small">{run.note}</p>}
              </div>)}</div>
            </details>
          </article>
        );
      })}
    </section>
  );
}
