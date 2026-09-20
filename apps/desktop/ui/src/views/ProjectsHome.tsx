import { useState } from "react";
import { Glyph } from "../components/Icons";
import { timeAgo } from "../lib/ui";
import { t } from "../lib/i18n";
import type { ProjectSummary } from "../ipc/types";

export function StatusPill({ health }: { health: string }) {
  if (health === "available")
    return <span className="pill pill-green"><Glyph name="checkCircle" className="pill-ico" /> {t("status.available")}</span>;
  if (health === "missing")
    return <span className="pill pill-gray"><Glyph name="alert" className="pill-ico" /> {t("status.notAvailable")}</span>;
  return <span className="pill pill-amber"><Glyph name="alert" className="pill-ico" /> {t("status.moved")}</span>;
}

export function ProjectsHome({ projects, onOpenFolder, onClone, onNav }: {
  projects: ProjectSummary[];
  onOpenFolder: () => void;
  onClone: () => void;
  onNav: (projectId: string) => void;
}) {
  const [mode, setMode] = useState<"grid" | "list">("grid");
  return (
    <div>
      <section className="hero">
        <h1>{t("home.title")}</h1>
        <p>{t("home.subtitle")}</p>
      </section>
      <div className="action-cards">
        <button className="action-card" onClick={onOpenFolder}>
          <span className="action-ico"><Glyph name="folderOpen" /></span>
          <span className="action-title">{t("home.openFolder")}</span>
          <span className="action-desc">{t("home.openFolderDesc")}</span>
        </button>
        <button className="action-card" onClick={onClone}>
          <span className="action-ico"><Glyph name="branch" /></span>
          <span className="action-title">{t("home.clone")}</span>
          <span className="action-desc">{t("home.cloneDesc")}</span>
        </button>
        <button className="action-card" onClick={onOpenFolder}>
          <span className="action-ico"><Glyph name="history" /></span>
          <span className="action-title">{t("home.recent")}</span>
          <span className="action-desc">{t("home.recentDesc")}</span>
        </button>
      </div>
      <div className="project-layout">
        <div>
          <div className="section-head"><h2>{t("home.recentTitle")}</h2></div>
          <p className="section-sub">{t("home.recentSub")}</p>
          {!projects.length ? (
            <div className="empty-state">
              <span className="big"><Glyph name="folder" /></span>
              {t("home.empty")}
            </div>
          ) : (
            <div className={mode === "grid" ? "recent-grid" : "recent-list"}>
              {projects.map((p) => (
                <button key={p.project_id} className="recent-card" onClick={() => onNav(p.project_id)}>
                  <div className="recent-top">
                    <span className="recent-ico"><Glyph name="folder" /></span>
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <div className="recent-name">{p.display_name}</div>
                      <div className="recent-desc">{p.detected_source} · {timeAgo(p.last_opened_at)}</div>
                    </span>
                    <StatusPill health={p.health} />
                  </div>
                  {mode === "grid" && (
                    <div className="recent-meta">
                      <span className="chip"><Glyph name="branch" /> {p.primary_root.split(/[\\/]/).pop()}</span>
                      <span className="chip"><Glyph name="clock" /> {timeAgo(p.last_opened_at)}</span>
                      <span className="chip">{p.primary_root}</span>
                    </div>
                  )}
                </button>
              ))}
            </div>
          )}
        </div>
        <aside className="context-panel">
          <div className="card context-card">
            <h4><Glyph name="shield" /> {t("trust.title")}</h4>
            <p className="card-sub">{t("trust.sub")}</p>
            <ul className="check-list">
              <li><span className="check-yes">✓</span> {t("trust.item1")}</li>
              <li><span className="check-yes">✓</span> {t("trust.item2")}</li>
              <li><span className="check-yes">✓</span> {t("trust.item3")}</li>
              <li><span className="check-yes">✓</span> {t("trust.item4")}</li>
              <li><span className="check-yes">✓</span> {t("trust.item5")}</li>
            </ul>
          </div>
          <div className="note-card">
            <h4><Glyph name="star" /> {t("trust.note")}</h4>
            {t("trust.noteBody")}
          </div>
        </aside>
      </div>
    </div>
  );
}
