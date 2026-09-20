/* App chrome: borderless-window traffic lights, collapsible sidebar,
   topbar (drag region + search + stop + language switch + avatar). */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Glyph } from "./Icons";
import { t } from "../lib/i18n";
import type { Lang } from "../lib/i18n";
import { setRoute } from "../lib/ui";
import type { OperationsSnapshot } from "../ipc/types";

const WIN = () => getCurrentWindow();

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

export type NavKey =
  | "projects" | "tasks" | "files" | "changes" | "git"
  | "artifacts" | "evidence" | "approvals";

const NAV: { key: NavKey; glyph: string; i18n: string }[] = [
  { key: "projects", glyph: "project", i18n: "nav.projects" },
  { key: "tasks", glyph: "task", i18n: "nav.tasks" },
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
          <Glyph name="stop" /> {t("nav.stop")}
        </button>
      </div>
    </aside>
  );
}

export function LangSwitch({ lang, onSwitch }: { lang: Lang; onSwitch: (lang: Lang) => void }) {
  return (
    <div className="lang-switch" role="group" aria-label={t("misc.language")}>
      <button className={`lang-opt${lang === "en" ? " active" : ""}`} onClick={() => onSwitch("en")}>EN</button>
      <button className={`lang-opt${lang === "vi" ? " active" : ""}`} onClick={() => onSwitch("vi")}>VI</button>
    </div>
  );
}

export function StopButton({ small = false, onStop }: { small?: boolean; onStop: () => void }) {
  return (
    <button className={`btn btn-danger-outline${small ? " btn-sm" : ""}`} onClick={onStop}>
      <Glyph name="stop" /> {t("nav.stop")}
    </button>
  );
}

export function Topbar({ crumbs, onSearch, onStop, lang, onSwitch, right }: {
  crumbs: { label: string; go?: string }[];
  onSearch: () => void;
  onStop: () => void;
  lang: Lang;
  onSwitch: (lang: Lang) => void;
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
      <button className="searchbox" onClick={onSearch} title={t("misc.searchTitle")}>
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
