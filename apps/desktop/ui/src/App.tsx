/* Foundation proof surface (PR 1): tokens, glass recipes, glyphs, and
   the mock IPC chain exercised end to end. The full Work-mode views are
   ported in the PR 2 cutover — this screen is not production IA. */
import { useEffect, useState } from "react";
import { Glyph } from "./components/glyphs";
import { Mark } from "./components/glyphs";
import { projectListRecent } from "./ipc/commands";
import type { ProjectSummary } from "./ipc/types";

export function App() {
  const [projects, setProjects] = useState<ProjectSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    projectListRecent()
      .then(setProjects)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="min-h-screen bg-paper-white p-8">
      <div className="glass-chrome mb-6 max-w-xl rounded-xl p-6">
        <h1 className="font-display text-2xl font-semibold tracking-tight text-civic-navy">
          Lumi — React foundation
        </h1>
        <p className="mt-2 text-sm text-soft-slate">
          Tokens, glass recipes, glyphs, and the typed IPC chain are live.
          Work-mode views arrive with the PR 2 cutover.
        </p>
        <div className="mt-4 flex items-center gap-3 text-civic-navy">
          <Glyph name="project" />
          <Glyph name="task" />
          <Glyph name="evidence" />
          <Glyph name="approval" />
          <Mark name="clarity" />
        </div>
      </div>
      <div className="glass-chrome max-w-xl rounded-xl p-6">
        <h2 className="text-sm font-semibold text-civic-navy">IPC chain (mock or runtime)</h2>
        {error != null && (
          <p className="mt-2 text-sm text-risk-red">{error}</p>
        )}
        {projects == null && error == null && (
          <p className="mt-2 text-sm text-soft-slate">Loading…</p>
        )}
        {projects != null && projects.length === 0 && (
          <p className="mt-2 text-sm text-soft-slate">No projects yet.</p>
        )}
        {projects != null && projects.length > 0 && (
          <ul className="mt-2 text-sm text-ink">
            {projects.map((p) => (
              <li key={p.project_id}>
                {p.display_name} · {p.health} · {p.detected_source}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

export default App;
