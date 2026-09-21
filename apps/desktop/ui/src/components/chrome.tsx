/* App chrome: borderless-window traffic lights, collapsible sidebar,
   topbar (drag region + search + stop + language switch + avatar).
   Controls use shadcn/ui primitives (Button, AlertDialog, Tooltip)
   themed to the Lumi design system. */
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { RefObject } from "react";
import { Glyph } from "./Icons";
import { t } from "../lib/i18n";
import type { Lang } from "../lib/i18n";
import type { NavKey } from "../lib/navigation";
import { setRoute } from "../lib/ui";
import type { OperationsSnapshot } from "../ipc/types";
import { Button } from "@/components/ui/button";
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent,
  AlertDialogDescription, AlertDialogTitle, AlertDialogTrigger,
} from "@/components/ui/overlays";

const WIN = () => getCurrentWindow();

/* macOS window chrome — bespoke dots, not product buttons (§10.1 does
   not govern titlebar controls). */
export function TrafficLights() {
  return (
    <div className="titlebar-strip">
      <div className="traffic-lights">
        <button className="tl tl-close" title={t("misc.close")} aria-label={t("misc.closeWindow")} onClick={() => void WIN().close()}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2.5} strokeLinecap="round"><path d="M18 6 6 18" /><path d="m6 6 12 12" /></svg>
        </button>
        <button className="tl tl-min" title={t("misc.minimize")} aria-label={t("misc.minimizeWindow")} onClick={() => void WIN().minimize()}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2.5} strokeLinecap="round"><path d="M5 12h14" /></svg>
        </button>
        <button className="tl tl-max" title={t("misc.maximize")} aria-label={t("misc.maximizeWindow")} onClick={() => void WIN().toggleMaximize()}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2.5} strokeLinecap="round"><path d="M5 5h14v14H5z" /></svg>
        </button>
      </div>
    </div>
  );
}

const NAV: { key: NavKey; glyph: string; i18n: string }[] = [
  { key: "projects", glyph: "project", i18n: "nav.projects" },
  { key: "tasks", glyph: "task", i18n: "nav.tasks" },
  { key: "automations", glyph: "task", i18n: "nav.automations" },
  { key: "files", glyph: "folder", i18n: "nav.files" },
  { key: "changes", glyph: "fileDiff", i18n: "nav.changes" },
  { key: "git", glyph: "branch", i18n: "nav.git" },
  { key: "artifacts", glyph: "artifact", i18n: "nav.artifacts" },
  { key: "evidence", glyph: "fileText", i18n: "nav.evidence" },
  { key: "approvals", glyph: "approval", i18n: "nav.approvals" },
];

export function Sidebar({ active, badgeTasks, badgeApprovals, snapshot, collapsed, onToggleCollapse, onNav, onStop }: {
  active: NavKey;
  badgeTasks: number;
  badgeApprovals: number;
  snapshot: OperationsSnapshot | null;
  collapsed: boolean;
  onToggleCollapse: () => void;
  onNav: (key: NavKey) => void;
  onStop: () => void;
}) {
  const ok = !!snapshot;
  return (
    <aside className="sidebar">
      <TrafficLights />
      <div className="logo-row">
        <img className="logo-full" src="assets/brand/lumi-fulltext.svg" alt={t("misc.brandName")} title={t("misc.brandName")} />
        <img className="logo-icon" src="assets/brand/ic_launcher_foreground.png" alt={t("misc.brandName")} title={t("misc.brandName")} />
      </div>
      <nav className="nav">
        {NAV.map(({ key, glyph: g, i18n }) => (
          <button key={key} className={`nav-btn${active === key ? " active" : ""}`} title={t(i18n)} onClick={() => onNav(key)}>
            <span className="nav-ico"><Glyph name={g} /></span>
            <span className="nav-label">{t(i18n)}</span>
            {key === "tasks" && badgeTasks > 0 && <span className="badge">{badgeTasks}</span>}
            {key === "approvals" && badgeApprovals > 0 && <span className="badge">{badgeApprovals}</span>}
          </button>
        ))}
      </nav>
      <div className="spacer" />
      <button className="collapse-toggle" title={collapsed ? t("misc.expandMenu") : t("misc.collapseMenu")} aria-label={t("misc.toggleMenu")} onClick={onToggleCollapse}>
        <Glyph name="chevronLeft" className={collapsed ? "rot-180" : ""} />
      </button>
      <div className="engine-card">
        <div className="engine-row">
          <span className={`dot ${ok ? "ok" : "warn"}`} />
          <div>
            <div className="engine-name">Lumi Engine</div>
            <div className="engine-state">{ok ? t("misc.engineRunning") : t("misc.engineUnavailable")}</div>
          </div>
        </div>
        <button className="btn btn-danger-outline btn-block" onClick={onStop}>
          <Glyph name="stop" /> <span className="btn-label">{t("nav.stop")}</span>
        </button>
      </div>
    </aside>
  );
}

export function LangSwitch({ lang, onSwitch }: { lang: Lang; onSwitch: (lang: Lang) => void }) {
  return (
    <div className="lang-switch" role="group" aria-label={t("misc.language")}>
      {(["en", "vi"] as const).map((code) => (
        <Button
          key={code}
          size="sm"
          variant={lang === code ? "secondary" : "ghost"}
          aria-pressed={lang === code}
          onClick={() => onSwitch(code)}
        >
          {code.toUpperCase()}
        </Button>
      ))}
    </div>
  );
}

/** Stop is consequential: it halts all agent work, so the shadcn
    AlertDialog asks for the explicit decision first (§4: approvals are
    UX). The confirm action keeps the destructive outline weight. */
export function StopButton({ small = false, onStop }: { small?: boolean; onStop: () => void }) {
  return (
    <AlertDialog>
      <AlertDialogTrigger asChild>
        <Button variant="destructive" size={small ? "sm" : "default"}>
          <Glyph name="stop" /> {t("nav.stop")}
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogTitle>{t("nav.stop")}</AlertDialogTitle>
        <AlertDialogDescription>{t("misc.stopConfirm")}</AlertDialogDescription>
        <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <AlertDialogCancel asChild>
            <Button variant="secondary" size="sm">{t("files.cancel")}</Button>
          </AlertDialogCancel>
          <AlertDialogAction asChild>
            <Button variant="destructive" size="sm" onClick={onStop}>
              <Glyph name="stop" /> {t("nav.stop")}
            </Button>
          </AlertDialogAction>
        </div>
      </AlertDialogContent>
    </AlertDialog>
  );
}

export function Topbar({ crumbs, onSearch, onStop, lang, onSwitch, searchRef, right }: {
  crumbs: { label: string; go?: string }[];
  onSearch: () => void;
  onStop: () => void;
  lang: Lang;
  onSwitch: (lang: Lang) => void;
  searchRef?: RefObject<HTMLButtonElement | null>;
  right?: React.ReactNode;
}) {
  return (
    <header className="topbar" data-tauri-drag-region>
      <div className="breadcrumb">
        {crumbs.map((c, i) =>
          i === crumbs.length - 1 ? (
            <span key={i} className="crumb current">{c.label}</span>
          ) : (
            <span key={i} style={{ display: "contents" }}>
              <button className="crumb" onClick={() => c.go && setRoute(c.go)}>{c.label}</button>
              <span className="crumb-sep">›</span>
            </span>
          ),
        )}
      </div>
      <button ref={searchRef} className="searchbox" onClick={onSearch} title={t("misc.searchTitle")}>
        <Glyph name="search" className="search-ico" />
        <span className="search-ph">{t("misc.searchPlaceholder")}</span>
        <span className="kbd">⌘K</span>
      </button>
      <div className="topbar-right">
        <LangSwitch lang={lang} onSwitch={onSwitch} />
        <StopButton small onStop={onStop} />
        <div className="avatar">LU</div>
        {right}
      </div>
    </header>
  );
}
