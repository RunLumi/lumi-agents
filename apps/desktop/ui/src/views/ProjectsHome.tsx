import { Glyph } from "../components/Icons";
import { timeAgo } from "../lib/ui";
import type { ProjectSummary } from "../ipc/types";

interface Props {
  projects: ProjectSummary[];
  onOpen: (projectId: string) => void;
}

export function ProjectsHome({ projects, onOpen }: Props) {
  return (
    <div>
      <div className="section-head"><h2>Recent Projects</h2></div>
      <p className="section-sub">Your recently opened projects.</p>
      {!projects.length ? (
        <div className="empty-state">No projects yet.</div>
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
                <span className={`pill ${p.health === "available" ? "pill-green" : "pill-amber"}`}>
                  {p.health === "available" ? "Available" : "Unavailable"}
                </span>
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
