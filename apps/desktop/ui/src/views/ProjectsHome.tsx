import { Glyph } from "../components/Icons";
import { t } from "../lib/i18n";
import { timeAgo } from "../lib/ui";
import { StatusBadge } from "@/components/ui/badge";
import type { ProjectSummary } from "../ipc/types";

interface Props {
  projects: ProjectSummary[];
  onOpen: (projectId: string) => void;
}

export function ProjectsHome({ projects, onOpen }: Props) {
  return (
    <div>
      <div className="section-head"><h2>{t("home.recentTitle")}</h2></div>
      <p className="section-sub">{t("home.recentSub")}</p>
      {!projects.length ? (
        <div className="empty-state">{t("home.empty")}</div>
      ) : (
        <div className="recent-grid">
          {projects.map((p) => (
            <button key={p.project_id} className="recent-card" onClick={() => onOpen(p.project_id)}>
              <div className="recent-top">
                <span className="recent-ico"><Glyph name="folder" /></span>
                <span style={{ flex: 1, minWidth: 0 }}>
                  <div className="recent-name">{p.display_name}</div>
                  <div className="recent-desc">{p.detected_source} · {timeAgo(p.last_opened_at)}</div>
                </span>
                {p.health === "available"
                  ? <StatusBadge variant="status-done">{t("status.available")}</StatusBadge>
                  : <StatusBadge variant="status-overdue">{t(p.health === "moved" ? "status.moved" : "status.notAvailable")}</StatusBadge>}
              </div>
              <div className="recent-meta">
                <span className="chip">{p.primary_root}</span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
