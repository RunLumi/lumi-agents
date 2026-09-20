"use strict";
/* Lumi Agents desktop app — Folder-as-Project Work mode.
   Built on the Lumi Design System (DESIGN.md). All state is
   runtime-derived: empty means empty. No sample data in production. */

// ================= icons (line, 1.75px stroke — DESIGN.md §9) =================
/* ==================================================================
   Lumi Glyph System (ICON.md) — three layers, one 20×20 construction.
   utility/  : normalized Tabler subset for ordinary actions/objects
   product/  : Lumi-native semantic glyphs (L-form derived)
   marks/    : clarity star, annotation dot, L-corner, diagonal channel
   Monochrome; stroke 1.75 at the 20-grid; angles 0/45/90 preferred.
   ================================================================== */
const UTILITY_GLYPHS = {
  folder: '<path d="M2.75 5.25A1.5 1.5 0 0 1 4.25 3.75h3.4l1.9 2.5h6.2a1.5 1.5 0 0 1 1.5 1.5v7.5a1.5 1.5 0 0 1-1.5 1.5H4.25a1.5 1.5 0 0 1-1.5-1.5z"/>',
  folderOpen: '<path d="M2.75 5.25A1.5 1.5 0 0 1 4.25 3.75h3.4l1.9 2.5h6.2a1.5 1.5 0 0 1 1.5 1.5v1"/><path d="m2.75 14.75 1.6-4h12.4l-1.7 4z"/>',
  file: '<path d="M11.25 2.75H5.25A1.5 1.5 0 0 0 3.75 4.25v11.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V6.75z"/><path d="M11.25 2.75v4h4"/>',
  fileText: '<path d="M11.25 2.75H5.25A1.5 1.5 0 0 0 3.75 4.25v11.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V6.75z"/><path d="M11.25 2.75v4h4"/><path d="M6.75 10.25h6.5"/><path d="M6.75 13.25h4.5"/>',
  fileDiff: '<path d="M11.25 2.75H5.25A1.5 1.5 0 0 0 3.75 4.25v11.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V6.75z"/><path d="M11.25 2.75v4h4"/><path d="M10 10.25v4.5"/><path d="M7.75 12.5h4.5"/><path d="M7.75 15.5h4.5" stroke-width="1"/>',
  search: '<circle cx="8.75" cy="8.75" r="5.25"/><path d="m12.75 12.75 4.25 4.25"/>',
  plus: '<path d="M10 4.25v11.5"/><path d="M4.25 10h11.5"/>',
  close: '<path d="m5 5 10 10"/><path d="M15 5 5 15"/>',
  chevronRight: '<path d="M7.5 4.5 13 10l-5.5 5.5"/>',
  chevronDown: '<path d="M4.5 7.5 10 13l5.5-5.5"/>',
  copy: '<rect x="7.25" y="7.25" width="9" height="9" rx="1.5"/><path d="M12.75 4.75h-6a2 2 0 0 0-2 2v6"/>',
  refresh: '<path d="M16.75 10a6.75 6.75 0 0 0-13-2.5"/><path d="M3.75 3.25v3.5h3.5"/><path d="M3.25 10a6.75 6.75 0 0 0 13 2.5"/><path d="M16.25 16.75v-3.5h-3.5"/>',
  clock: '<circle cx="10" cy="10" r="6.75"/><path d="M10 6.25V10l2.5 1.75"/>',
  arrowRight: '<path d="M3.25 10h12.5"/><path d="m10.5 4.75 5.25 5.25-5.25 5.25"/>',
  external: '<path d="M11.25 3.25h5.5v5.5"/><path d="M15.75 4.25 8.25 11.75"/><path d="M16.75 11.75v3.5a1.5 1.5 0 0 1-1.5 1.5h-9.5a1.5 1.5 0 0 1-1.5-1.5v-9.5a1.5 1.5 0 0 1 1.5-1.5h3.5"/>',
  settings: '<path d="M17.25 4.5h-6"/><path d="M7.25 4.5H2.75"/><path d="M17.25 10h-4"/><path d="M9 10H2.75"/><path d="M17.25 15.5h-2.5"/><path d="M10.5 15.5H2.75"/><path d="M13.25 2.5v4"/><path d="M11 8v4"/><path d="M14.75 13.5v4"/>',
  list: '<path d="M6.75 4.5h10.5"/><path d="M6.75 10h10.5"/><path d="M6.75 15.5h10.5"/><path d="M3 4.5h.01"/><path d="M3 10h.01"/><path d="M3 15.5h.01"/>',
  edit: '<path d="m13.25 3 3.75 3.75L7.25 16.5 3 17.5l1-4.25z"/>',
  trash: '<path d="M3 4.75h14"/><path d="M6.75 4.75V3.25h6.5v1.5"/><path d="m5.25 4.75.7 11.25a1.5 1.5 0 0 0 1.5 1.4h5.1a1.5 1.5 0 0 0 1.5-1.4l.7-11.25"/><path d="M8.25 8v6"/><path d="M11.75 8v6"/>',
  eye: '<path d="M2 10s3-5.75 8-5.75 8 5.75 8 5.75-3 5.75-8 5.75S2 10 2 10z"/><circle cx="10" cy="10" r="2.5"/>',
  download: '<path d="M17.25 13.5v2.75a1.5 1.5 0 0 1-1.5 1.5h-11.5a1.5 1.5 0 0 1-1.5-1.5v-2.75"/><path d="m6 9.25 4 4 4-4"/><path d="M10 13.25V3"/>',
  inbox: '<path d="M17.5 10.5H13l-1.5 2.25h-3L7 10.5H2.5"/><path d="M4.75 3.75h10.5l2.25 6.75v4.75a1.5 1.5 0 0 1-1.5 1.5H4a1.5 1.5 0 0 1-1.5-1.5v-4.75z"/>',
  play: '<path d="m5.5 3.5 11 6.5-11 6.5z"/>',
  stop: '<rect x="4.75" y="4.75" width="10.5" height="10.5" rx="1.5"/>',
  lock: '<rect x="3.75" y="8.75" width="12.5" height="8" rx="1.5"/><path d="M6.75 8.75V6a3.25 3.25 0 0 1 6.5 0v2.75"/>',
  branch: '<circle cx="14.75" cy="5" r="2.25"/><circle cx="5.25" cy="15" r="2.25"/><path d="M5.25 12.75V6a3.5 3.5 0 0 1 3.5-3.5h1.5"/><path d="M12.25 6.75a6 6 0 0 1-4.9 5.9"/>',
  commit: '<circle cx="10" cy="10" r="2.75"/><path d="M2.75 10h4.5"/><path d="M12.75 10h4.5"/>',
  home: '<path d="M3 8.25 10 2.75l7 5.5v7.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 3 15.75z"/><path d="M7.5 17.5v-5h5v5"/>',
  history: '<path d="M3.25 10a6.75 6.75 0 1 0 2-4.75L2.75 7.75"/><path d="M2.75 3.25v4.5h4.5"/><path d="M10 6.75V10l2.5 1.5"/>',
  hash: '<path d="M3.5 7.5h13"/><path d="M3.5 12.5h13"/><path d="M8.5 3.5 7 16.5"/><path d="M13 3.5l-1.5 13"/>',
  layers: '<path d="M10 2.75 3 6.25l7 3.5 7-3.5z"/><path d="m3 10 7 3.5 7-3.5"/><path d="m3 13.75 7 3.5 7-3.5"/>',
  message: '<path d="M17.25 12.5a1.5 1.5 0 0 1-1.5 1.5H5.75l-3.5 3.5v-11.5a1.5 1.5 0 0 1 1.5-1.5h12a1.5 1.5 0 0 1 1.5 1.5z"/>',
  terminal: '<rect x="2.75" y="3.75" width="14.5" height="12.5" rx="1.5"/><path d="m5.75 7.25 2.25 2.25-2.25 2.25"/><path d="M10.25 12h4"/>',
  verified: '<circle cx="10" cy="10" r="6.75"/><path d="m7.25 10.25 1.9 1.9 3.85-4.4"/>',
  insight: '<path d="M10 2.75a5.5 5.5 0 0 0-3.25 9.94c.75.56 1.25 1.31 1.25 2.31h4c0-1 .5-1.75 1.25-2.31A5.5 5.5 0 0 0 10 2.75z"/><path d="M8 17.75h4"/><path d="M8.75 20h2.5"/>',
  activity: '<path d="M2.75 10h3.5l2.25-6.25 3.5 12 2.25-5.75h3.5"/>',
};

/* Lumi product glyphs — drawn from the folded L-form; annotation dot and
   L-corner carry provenance and structure. */
const PRODUCT_GLYPHS = {
  project: '<path d="M3.25 17V7.25"/><path d="M3.25 17h10.5"/><rect x="8.25" y="4.75" width="8.5" height="8" rx="1"/>',
  task: '<path d="m3.25 10.25 3.25 3.25 7-7.25"/><circle cx="16.25" cy="14" r="1.75" fill="currentColor" stroke="none"/>',
  agent: '<path d="M3.5 16.5V8"/><path d="M3.5 16.5H11"/><circle cx="13.75" cy="6.25" r="2.25"/><path d="m8.5 12.5 3.4-3.9"/>',
  evidence: '<path d="M11.25 2.75H5.25A1.5 1.5 0 0 0 3.75 4.25v11.5a1.5 1.5 0 0 0 1.5 1.5h10a1.5 1.5 0 0 0 1.5-1.5V6.75z"/><path d="M11.25 2.75v4h4"/><circle cx="13.75" cy="14" r="1.5" fill="currentColor" stroke="none"/>',
  approval: '<circle cx="10" cy="10" r="6.75"/><path d="m7.25 10.25 1.9 1.9 3.85-4.4"/>',
  artifact: '<path d="M17.25 6.5 10 2.75 2.75 6.5v7L10 17.25l7.25-3.75z"/><path d="m2.75 6.5 7.25 3.75 7.25-3.75"/><path d="M10 10.25v7"/>',
  verification: '<circle cx="8.75" cy="8.75" r="5.25"/><path d="m12.5 12.5 4.25 4.25"/><path d="m6.75 8.75 1.5 1.5 2.75-3"/>',
  recovery: '<path d="M16.5 10a6.5 6.5 0 1 1-1.9-4.6"/><path d="M16.5 2.75v3.5H13"/><circle cx="10" cy="10" r="1.4" fill="currentColor" stroke="none"/>',
  exception: '<path d="M10 3 2.5 16.25h15z"/><path d="M10 8v3.25"/><circle cx="10" cy="13.75" r="1" fill="currentColor" stroke="none"/>',
  authority: '<path d="M10 2.5 16.5 5v5.25c0 4.1-2.9 6.75-6.5 8-3.6-1.25-6.5-3.9-6.5-8V5z"/><path d="M7.25 12V8.75h5.25"/>',
  guardrail: '<path d="M10 2.5 16.5 5v5.25c0 4.1-2.9 6.75-6.5 8-3.6-1.25-6.5-3.9-6.5-8V5z"/><path d="M7.25 10h5.25"/>',
  workflow: '<circle cx="4.75" cy="4.75" r="2"/><circle cx="15.25" cy="4.75" r="2"/><circle cx="10" cy="15.25" r="2"/><path d="M6.5 6.25 8.9 13.5"/><path d="M13.5 6.25 11.1 13.5"/><path d="M6.75 4.75h6.5"/>',
};

/* Semantic marks — state attaches to an object glyph, never a new icon. */
const MARKS = {
  clarity: '<path d="M10 2.25 11.8 8.2 17.75 10 11.8 11.8 10 17.75 8.2 11.8 2.25 10l5.95-1.8z"/>',
  annotationDot: '<circle cx="10" cy="10" r="3.5" fill="currentColor" stroke="none"/>',
  lCorner: '<path d="M3.75 3.75v12.5h12.5"/>',
  diagonal: '<path d="M3.25 16.75 16.75 3.25"/>',
};

/* Canonical glyph table + legacy aliases for existing call sites. */
const GLYPHS = {
  ...UTILITY_GLYPHS,
  ...PRODUCT_GLYPHS,
  folderOpen: UTILITY_GLYPHS.folderOpen,
  checkCircle: UTILITY_GLYPHS.verified,
  check: UTILITY_GLYPHS.check,
  star: MARKS.clarity,
  zap: PRODUCT_GLYPHS.agent,
  shield: PRODUCT_GLYPHS.authority,
  alert: PRODUCT_GLYPHS.exception,
  box: PRODUCT_GLYPHS.artifact,
  bulb: UTILITY_GLYPHS.insight,
  diff: UTILITY_GLYPHS.fileDiff,
  activity: UTILITY_GLYPHS.activity,
  eye: UTILITY_GLYPHS.eye,
  hash: UTILITY_GLYPHS.hash,
  message: UTILITY_GLYPHS.message,
  play: UTILITY_GLYPHS.play,
  stop: UTILITY_GLYPHS.stop,
  external: UTILITY_GLYPHS.external,
  download: UTILITY_GLYPHS.download,
  inbox: UTILITY_GLYPHS.inbox,
  history: UTILITY_GLYPHS.history,
  home: UTILITY_GLYPHS.home,
};

function glyph(name, cls = "") {
  const body = GLYPHS[name] || UTILITY_GLYPHS.file;
  return `<svg class="${cls}" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
}

function mark(name, cls = "") {
  const body = MARKS[name] || MARKS.annotationDot;
  return `<svg class="${cls}" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
}

/* Legacy call-site alias — all geometry flows through the glyph system. */
const icon = glyph;
// Window-control glyphs, injected once (hover-revealed traffic lights).
function injectWindowGlyphs() {
  const close = document.getElementById("win-close");
  const min = document.getElementById("win-min");
  const max = document.getElementById("win-max");
  if (close) close.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>`;
  if (min) min.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M5 12h14"/></svg>`;
  if (max) max.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M5 5h14v14H5z"/></svg>`;
}
// ================= IPC =================
function hasTauri() {
  return typeof window.__TAURI__ !== "undefined" && window.__TAURI__.core;
}
async function invoke(cmd, args = {}) {
  if (!hasTauri()) return { __unavailable: true };
  return window.__TAURI__.core.invoke(cmd, args);
}

// ================= state =================
const S = {
  route: { view: "projects" },
  projects: [],
  project: null,
  summary: null,
  git: null,
  tasks: [],
  changeSets: [],
  artifacts: [],
  snapshot: null,
  openFiles: [],      // [{path, content, sha256}]
  activeFile: null,
  editing: false,
  diffMode: "unified",
  changesView: "entries",
  taskSubTab: "overview",
  paletteOpen: false,
  paletteSection: "all",
};

// ================= utils =================
function esc(v) {
  return String(v == null ? "" : v)
    .replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}
function timeAgo(rfc3339) {
  if (!rfc3339) return "";
  const then = new Date(rfc3339).getTime();
  if (Number.isNaN(then)) return "";
  const mins = Math.max(0, Math.round((Date.now() - then) / 60000));
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  const days = Math.round(hours / 24);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}
function toast(message, kind = "") {
  const el = document.createElement("div");
  el.className = `toast ${kind}`;
  el.textContent = message;
  document.getElementById("toasts").appendChild(el);
  setTimeout(() => el.remove(), 4200);
}
function setRoute(hash) { location.hash = hash; }
function cap(t) { return t.charAt(0).toUpperCase() + t.slice(1); }
function fmtSize(bytes) {
  if (bytes == null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}
function gitChanged(git) {
  if (!git) return 0;
  return git.staged.length + git.unstaged.length + git.untracked.length;
}
function gitIsClean(git) {
  return !!git && gitChanged(git) === 0;
}
function langOf(path) {
  const ext = path.split(".").pop().toLowerCase();
  const map = { ts: "TypeScript", tsx: "TypeScript", js: "JavaScript", rs: "Rust", py: "Python", go: "Go", json: "JSON", md: "Markdown", toml: "TOML", yml: "YAML", yaml: "YAML", html: "HTML", css: "CSS", sh: "Shell" };
  return map[ext] || "Text";
}

// ================= data =================
async function loadProjects() {
  const result = await invoke("project_list_recent");
  S.projects = result && result.__unavailable ? null : (result || []);
}
async function loadProject(projectId) {
  const overview = await invoke("project_overview", { projectId });
  if (overview && overview.__unavailable) { S.project = null; return; }
  S.project = overview;
  S.git = await invoke("git_status", { projectId });
  if (S.git && S.git.__unavailable) S.git = null;
  S.tasks = (await invoke("task_list", { projectId })) || [];
  S.changeSets = (await invoke("change_sets", { projectId })) || [];
  S.artifacts = (await invoke("artifacts_list", { projectId })) || [];
}
async function loadSnapshot() {
  const snapshot = await invoke("get_operations_snapshot");
  S.snapshot = snapshot && snapshot.__unavailable ? null : snapshot;
  const dot = document.getElementById("engine-dot");
  const label = document.getElementById("engine-state");
  if (S.snapshot) {
    dot.className = "dot ok";
    label.textContent = "Running locally";
  } else {
    dot.className = "dot warn";
    label.textContent = "Unavailable";
  }
}

// ================= routing =================
function parseRoute() {
  const hash = location.hash.replace(/^#\/?/, "");
  const parts = hash.split("/").filter(Boolean);
  if (parts[0] === "p" && parts[1]) {
    S.route = { view: "project", projectId: decodeURIComponent(parts[1]), tab: parts[2] || "home", taskId: parts[3] ? decodeURIComponent(parts[3]) : null };
  } else {
    S.route = { view: parts[0] || "projects" };
  }
}
function setBreadcrumb(parts) {
  const el = document.getElementById("breadcrumb");
  el.innerHTML = parts
    .map((p, i) =>
      i === parts.length - 1
        ? `<span class="crumb current">${esc(p.label)}</span>`
        : `<button class="crumb" data-go="${esc(p.go || "#/projects")}">${esc(p.label)}</button><span class="crumb-sep">›</span>`
    ).join("");
  el.querySelectorAll("[data-go]").forEach((c) =>
    c.addEventListener("click", () => setRoute(c.dataset.go)));
}
function updateBadges() {
  const taskBadge = document.getElementById("nav-task-badge");
  const approvalBadge = document.getElementById("nav-approval-badge");
  const exceptions = S.snapshot && S.snapshot.exceptions ? S.snapshot.exceptions.length : 0;
  const pending = S.snapshot && S.snapshot.pending_approvals ? S.snapshot.pending_approvals.length : 0;
  const active = S.tasks && S.route.view === "project"
    ? S.tasks.filter((t) => !["COMPLETED", "FAILED", "CANCELLED"].includes(t.status)).length
    : 0;
  taskBadge.classList.toggle("hidden", !active);
  if (active) taskBadge.textContent = active;
  approvalBadge.classList.toggle("hidden", !pending && !exceptions);
  if (pending || exceptions) approvalBadge.textContent = pending + exceptions;
}

function render() {
  parseRoute();
  const navKey = S.route.view === "project" ? (["task"].includes(S.route.tab) ? "tasks" : S.route.tab) : S.route.view;
  document.querySelectorAll(".nav-btn").forEach((b) => {
    b.classList.toggle("active", b.dataset.nav === navKey);
  });
  const content = document.getElementById("content");
  if (S.route.view === "projects") {
    setBreadcrumb([{ label: "Projects" }]);
    content.replaceChildren(projectsHomeView());
  } else if (S.route.view === "project") {
    setBreadcrumb([
      { label: "Projects", go: "#/projects" },
      ...(S.project ? [{ label: S.project.display_name }] : []),
      ...(S.route.tab === "task"
        ? [{ label: "Tasks", go: `#/p/${S.route.projectId}/tasks` }, { label: "Task" }]
        : [{ label: cap(S.route.tab) }]),
    ]);
    renderProject(content);
  } else {
    setBreadcrumb([{ label: cap(S.route.view) }]);
    content.replaceChildren(projectRequiredNotice(cap(S.route.view)));
  }
  updateBadges();
}

// ================= projects home =================
function projectsHomeView() {
  const wrap = document.createElement("div");
  wrap.innerHTML = `
    <section class="hero">
      <h1>Open a project, start building with Lumi</h1>
      <p>Lumi works with your code directly, in the folder you choose.
         Open a folder, clone a repository, or pick up where you left off —
         with every change inspected, validated, and yours.</p>
    </section>
    <div class="action-cards">
      <button class="action-card" id="action-open-folder">
        <span class="action-ico">${icon("folderOpen")}</span>
        <span class="action-title">Open Folder</span>
        <span class="action-desc">Work with an existing codebase on your machine.</span>
      </button>
      <button class="action-card" id="action-clone">
        <span class="action-ico">${icon("branch")}</span>
        <span class="action-title">Clone Repository</span>
        <span class="action-desc">Start a new project by cloning a repository.</span>
      </button>
      <button class="action-card" id="action-recent">
        <span class="action-ico">${icon("history")}</span>
        <span class="action-title">Open Recent</span>
        <span class="action-desc">Quickly reopen a recent project and continue working.</span>
      </button>
    </div>
    <div class="project-layout">
      <div>
        <div class="section-head">
          <h2>Recent Projects</h2>
          <div class="section-tools">
            <div class="view-toggle">
              <button id="view-grid" class="active" title="Grid">${icon("layers")}</button>
              <button id="view-list" title="List">${icon("list")}</button>
            </div>
          </div>
        </div>
        <p class="section-sub">Your recently opened projects. Click to open and continue with Lumi.</p>
        <div id="recents"></div>
        <button class="dropzone" id="dropzone">
          ${icon("folder")}<span><b>Drag and drop a folder here</b> — or click to browse</span>
        </button>
      </div>
      <aside class="context-panel">
        <div class="card context-card">
          <h4>${icon("shield")} Project Access &amp; Security</h4>
          <p class="card-sub">You're in control</p>
          <ul class="check-list">
            <li><span class="check-yes">✓</span> Opening a project gives Lumi access only to the folder you choose</li>
            <li><span class="check-yes">✓</span> No access to your home directory or other projects</li>
            <li><span class="check-yes">✓</span> Cannot read or modify files outside the project</li>
            <li><span class="check-yes">✓</span> Works entirely on your local machine</li>
            <li><span class="check-yes">✓</span> You can change or close the project at any time</li>
          </ul>
        </div>
        <div class="note-card">
          <h4>${icon("star")} A more focused way to work</h4>
          Folder-as-project keeps things simple, secure, and productive.
        </div>
      </aside>
    </div>`;

  wrap.querySelector("#action-open-folder").addEventListener("click", pickAndOpenFolder);
  wrap.querySelector("#dropzone").addEventListener("click", pickAndOpenFolder);
  wrap.querySelector("#action-recent").addEventListener("click", () =>
    wrap.querySelector("#recents").scrollIntoView({ behavior: "smooth" }));
  wrap.querySelector("#action-clone").addEventListener("click", cloneRepositoryFlow);
  wrap.querySelector("#view-grid").addEventListener("click", () => setRecentsView(wrap, "grid"));
  wrap.querySelector("#view-list").addEventListener("click", () => setRecentsView(wrap, "list"));

  const recents = wrap.querySelector("#recents");
  recents.dataset.mode = "grid";
  if (S.projects === null) {
    recents.innerHTML = UNAVAILABLE;
  } else if (!S.projects.length) {
    recents.innerHTML = `<div class="empty-state"><span class="big">${icon("folder")}</span>No projects yet. Open a folder to create your first project.</div>`;
  } else {
    renderRecentsInto(recents);
  }
  return wrap;
}
function setRecentsView(wrap, mode) {
  const recents = wrap.querySelector("#recents");
  recents.dataset.mode = mode;
  wrap.querySelector("#view-grid").classList.toggle("active", mode === "grid");
  wrap.querySelector("#view-list").classList.toggle("active", mode === "list");
  renderRecentsInto(recents);
}
function renderRecentsInto(recents) {
  const mode = recents.dataset.mode || "grid";
  recents.innerHTML = "";
  const container = document.createElement("div");
  container.className = mode === "grid" ? "recent-grid" : "recent-list";
  for (const project of S.projects) container.append(recentCard(project, mode));
  recents.append(container);
  // Lazy branch chips: real git data per available project.
  for (const project of S.projects.filter((p) => p.health === "available")) {
    invoke("git_status", { projectId: project.project_id }).then((git) => {
      if (!git || git.__unavailable || !git.branch) return;
      const chip = container.querySelector(`[data-branch-for="${CSS.escape(project.project_id)}"]`);
      if (chip) chip.innerHTML = `${icon("branch")} ${esc(git.branch)}`;
    }).catch(() => {});
  }
}
function recentCard(project, mode) {
  const card = document.createElement("button");
  card.className = "recent-card";
  const isList = mode === "list";
  if (isList) card.classList.add("card-hover");
  card.innerHTML = `
    <div class="recent-top">
      <span class="recent-ico">${icon("folder")}</span>
      <span style="flex:1;min-width:0">
        <div class="recent-name">${esc(project.display_name)}</div>
        <div class="recent-desc">${esc(project.detected_source)} project · ${esc(timeAgo(project.last_opened_at))}</div>
      </span>
      ${statusPill(project.health)}
    </div>
    ${isList ? "" : `
    <div class="recent-meta">
      <span class="chip" data-branch-for="${esc(project.project_id)}">${icon("branch")} …</span>
      <span class="chip">${icon("clock")} ${esc(timeAgo(project.last_opened_at))}</span>
      <span class="chip">${esc(project.primary_root)}</span>
    </div>`}`;
  card.addEventListener("click", () => {
    if (project.health !== "available") { openProjectMenu(project); return; }
    setRoute(`#/p/${encodeURIComponent(project.project_id)}/home`);
  });
  return card;
}
function statusPill(health) {
  if (health === "available") return `<span class="pill pill-green">${icon("checkCircle")} Available</span>`;
  if (health === "missing") return `<span class="pill pill-gray">${icon("alert")} Not Available</span>`;
  return `<span class="pill pill-amber">${icon("alert")} Moved — relink needed</span>`;
}
function taskStatusPill(status) {
  const map = {
    CREATED: ["pill-blue", "Created"], QUEUED: ["pill-blue", "Queued"],
    RUNNING: ["pill-green", "Running"], WAITING_APPROVAL: ["pill-amber", "Waiting approval"],
    WAITING_USER: ["pill-amber", "Waiting for you"], WAITING_EXTERNAL: ["pill-amber", "Waiting external"],
    PAUSED: ["pill-amber", "Paused"], COMPLETED: ["pill-green", "Completed"],
    FAILED: ["pill-red", "Failed"], AMBIGUOUS: ["pill-amber", "Ambiguous"],
    CANCELLED: ["pill-gray", "Cancelled"],
  };
  const [cls, label] = map[status] || ["pill-gray", status || "Unknown"];
  return `<span class="pill ${cls}">${esc(label)}</span>`;
}
function openProjectMenu(project) {
  if (project.health === "missing") {
    if (confirm(`Project root unavailable:\n${project.primary_root}\n\nRemove this project from Recent Projects? (The folder on disk is not touched.)`)) {
      invoke("project_remove", { projectId: project.project_id }).then(refresh);
    }
  } else {
    const newPath = prompt(`The folder at ${project.primary_root} was moved or replaced.\nEnter the project's current folder to relink:`);
    if (newPath) {
      invoke("project_relink", { projectId: project.project_id, path: newPath })
        .then(refresh).catch((e) => toast(String(e), "error"));
    }
  }
}
async function pickAndOpenFolder() {
  if (!hasTauri()) { toast("Folder picking needs the desktop app.", "error"); return; }
  try {
    const selected = await window.__TAURI__.dialog.open({ directory: true, multiple: false, title: "Open project folder" });
    if (!selected) return;
    const opened = await invoke("project_open_folder", { path: selected });
    toast(opened.created ? "Project created." : "Project reopened.", "success");
    refresh();
    setRoute(`#/p/${encodeURIComponent(opened.project.project_id)}/home`);
  } catch (error) {
    toast(String(error), "error");
  }
}
async function cloneRepositoryFlow() {
  const source = prompt("Repository URL or local path to clone:");
  if (!source) return;
  if (!hasTauri()) { toast("Folder picking needs the desktop app.", "error"); return; }
  try {
    const parent = await window.__TAURI__.dialog.open({
      directory: true, multiple: false, title: "Choose the parent folder for the clone",
    });
    if (!parent) return;
    const name = prompt("Display name for the new project (optional):");
    const opened = await invoke("project_clone", {
      source: source.trim(), destinationParent: parent, displayName: name || null,
    });
    toast("Repository cloned and opened as a project.", "success");
    refresh();
    setRoute(`#/p/${encodeURIComponent(opened.project.project_id)}/home`);
  } catch (error) {
    toast(String(error), "error");
  }
}

// ================= project shell =================
async function renderProject(content) {
  const projectId = S.route.projectId;
  try {
    await loadProject(projectId);
  } catch (error) {
    content.innerHTML = `<div class="empty-state unavailable-note"><b>Project failed to load.</b><br>${esc(String(error))}</div>`;
    return;
  }
  if (!S.project) { content.innerHTML = UNAVAILABLE; return; }
  setBreadcrumb([
    { label: "Projects", go: "#/projects" },
    { label: S.project.display_name },
    ...(S.route.tab === "task"
      ? [{ label: "Tasks", go: `#/p/${S.route.projectId}/tasks` }, { label: "Task" }]
      : [{ label: cap(S.route.tab) }]),
  ]);
  content.replaceChildren();
  content.append(projectHeaderView());
  content.append(tabsView());
  const layout = document.createElement("div");
  layout.className = "project-layout";
  const main = document.createElement("div");
  main.style.minWidth = "0";
  const view = tabContent();
  if (view instanceof Promise) {
    const skeleton = document.createElement("div");
    skeleton.className = "skeleton";
    main.append(skeleton);
    const atTab = S.route.tab;
    const atTask = S.route.taskId;
    view.then((el) => {
      // Drop stale renders if the user navigated while loading.
      if (S.route.tab !== atTab || S.route.taskId !== atTask) return;
      main.replaceChildren(el);
    }).catch((error) => {
      main.innerHTML = `<div class="empty-state unavailable-note"><b>View failed to render.</b><br>${esc(String(error))}</div>`;
    });
  } else {
    main.append(view);
  }
  layout.append(main, contextPanel());
  content.append(layout);
}
function projectHeaderView() {
  const head = document.createElement("div");
  head.className = "project-head";
  const branch = S.git ? S.git.branch : null;
  head.innerHTML = `
    <div class="project-ico">${icon("folder")}</div>
    <div style="flex:1;min-width:0">
      <h1 class="project-title">${esc(S.project.display_name)}</h1>
      <p class="project-desc">${esc(S.project.detected_source)} project</p>
      <div class="project-chips">
        <span class="chip">${icon("folder")} ${esc(S.project.primary_root)}</span>
        ${branch ? `<span class="chip">${icon("branch")} ${esc(branch)}</span>` : ""}
        <span class="pill pill-green">${icon("lock")} Local • Safe</span>
      </div>
    </div>
    <div class="project-head-right">
      <span class="sync-note">${icon("clock")} Opened ${esc(timeAgo(S.project.last_opened_at || new Date().toISOString()))}</span>
      <div style="display:flex;gap:8px">
        <button class="btn btn-sm" id="open-in-finder">${icon("external")} Open in Finder</button>
        <button class="btn btn-primary btn-sm" id="new-task-btn">${icon("plus")} New Task</button>
      </div>
    </div>`;
  head.querySelector("#open-in-finder").addEventListener("click", async () => {
    if (hasTauri() && window.__TAURI__.opener) {
      await window.__TAURI__.opener.revealItemInDir(S.project.primary_root);
    } else { toast("Available in the desktop app."); }
  });
  head.querySelector("#new-task-btn").addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/tasks`));
  return head;
}
function tabsView() {
  const tabs = document.createElement("div");
  tabs.className = "tabs";
  const counts = {
    tasks: (S.tasks || []).length,
    changes: (S.changeSets || []).length,
    artifacts: (S.artifacts || []).length,
  };
  const defs = [
    ["home", "Home"], ["tasks", "Tasks", counts.tasks], ["files", "Files"],
    ["changes", "Changes", counts.changes], ["git", "Git"],
    ["artifacts", "Artifacts", counts.artifacts], ["evidence", "Evidence"],
    ["approvals", "Approvals"], ["settings", "Settings"],
  ];
  for (const [key, label, count] of defs) {
    const b = document.createElement("button");
    b.className = `tab ${S.route.tab === key || (key === "tasks" && S.route.tab === "task") ? "active" : ""}`;
    b.innerHTML = `${esc(label)}${count != null ? ` <span class="count">${count}</span>` : ""}`;
    b.addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/${key}`));
    tabs.append(b);
  }
  return tabs;
}
function tabContent() {
  switch (S.route.tab) {
    case "tasks": return tasksView();
    case "task": return taskDetailView(S.route.taskId);
    case "files": return filesView();
    case "changes": return changesView();
    case "git": return gitView();
    case "artifacts": return artifactsView();
    case "evidence": return evidenceView();
    case "approvals": return approvalsView();
    case "settings": return settingsView();
    default: return homeTabView();
  }
}
function contextPanel() {
  const panel = document.createElement("aside");
  panel.className = "context-panel";
  const p = S.project;
  const git = S.git;
  panel.innerHTML = `
    <div class="card context-card">
      <h4>${icon("folder")} Project Context</h4>
      <div class="kv"><span class="kv-key">Project Root</span></div>
      <div class="chip mono" style="width:100%;justify-content:space-between">${esc(p.primary_root)}</div>
      <div class="kv" style="margin-top:8px"><span class="kv-key">Environment</span><span class="kv-val mono">${esc(p.environment_id)}</span></div>
      <div class="kv"><span class="kv-key">Branch</span><span class="kv-val mono">${esc((git && git.branch) || "—")}</span></div>
      <div class="kv"><span class="kv-key">Permissions Boundary</span><span class="pill pill-green">Strict (Project Only)</span></div>
      <p class="card-sub" style="margin:8px 0 0">Lumi can only access files within this project's authorized roots. No access to your home directory or other projects.</p>
    </div>
    <div class="card context-card">
      <h4>${icon("checkCircle")} What Lumi can access</h4>
      <ul class="check-list">
        <li><span class="check-yes">✓</span> Read and edit files in this project</li>
        <li><span class="check-yes">✓</span> Run project commands (via bounded shell)</li>
        <li><span class="check-yes">✓</span> Read git history and create local branches</li>
        <li><span class="check-no">✕</span> No access to personal files or external networks</li>
      </ul>
    </div>
    <div class="note-card">
      <h4>${icon("lock")} Working in a safe, local environment</h4>
      Your code stays on your machine. Capabilities shown are exactly what the runtime provides — nothing more.
    </div>`;
  return panel;
}
function projectRequiredNotice(label) {
  const el = document.createElement("div");
  if (S.projects && S.projects.length) {
    el.innerHTML = `<div class="empty-state"><span class="big">${icon("folder")}</span>Open a project first to use ${esc(label)}.<div style="margin-top:10px">
      <button class="btn btn-primary" id="go-projects">Go to Projects</button></div></div>`;
    el.querySelector("#go-projects").addEventListener("click", () => setRoute("#/projects"));
  } else {
    el.innerHTML = UNAVAILABLE;
  }
  return el;
}

// ================= home tab =================
function homeTabView() {
  const wrap = document.createElement("div");
  const activeTask = (S.tasks || []).find((t) => t.status === "RUNNING" || t.status === "WAITING_APPROVAL" || t.status === "WAITING_USER");
  if (activeTask) wrap.append(activeTaskCard(activeTask));
  const stats = document.createElement("div");
  stats.className = "stat-cards";
  stats.append(workingTreeCard(), validationCard(), recentArtifactsCard());
  wrap.append(stats);
  const two = document.createElement("div");
  two.className = "two-cards";
  two.append(recentChangesCard(), taskHistoryCard());
  wrap.append(two);
  return wrap;
}
function activeTaskCard(task) {
  const card = document.createElement("div");
  card.className = "card";
  card.style.marginBottom = "13px";
  const steps = [
    ["Understand", task.status !== "CREATED"],
    ["Work", ["RUNNING", "WAITING_APPROVAL", "WAITING_USER", "PAUSED"].includes(task.status)],
    ["Validate", true], ["Review", ["COMPLETED"].includes(task.status)],
  ];
  const doneCount = steps.filter(([, d]) => d).length;
  const pct = Math.round((doneCount / steps.length) * 100);
  card.innerHTML = `
    <h3>${icon("activity")} Active Task
      <span class="pill pill-blue">${icon("agent")} Agent registered</span>
      <span style="margin-left:auto"><button class="card-link" data-go-task="${esc(task.task_id)}">Open task ${icon("arrowRight")}</button></span>
    </h3>
    <p style="margin:4px 0 0;font-weight:600;color:var(--color-civic-navy)">${esc(task.goal)}</p>
    <div class="progress-track"><div class="progress-fill" style="width:${pct}%"></div></div>
    <div class="muted small tabular">${doneCount} of ${steps.length} stages · ${pct}%</div>
    <div class="stepper">
      ${steps.map(([label, done]) => `
        <div class="step ${done ? "done" : ""}">
          <div class="step-dot">${done ? icon("check") : ""}</div>
          <div class="step-label">${esc(label)}</div>
        </div>`).join("")}
    </div>`;
  card.querySelector("[data-go-task]").addEventListener("click", () =>
    setRoute(`#/p/${S.route.projectId}/task/${task.task_id}`));
  return card;
}
function workingTreeCard() {
  const card = document.createElement("div");
  card.className = "card";
  const git = S.git;
  const clean = gitIsClean(git);
  card.innerHTML = `
    <h3>${icon("branch")} Working Tree Status
      ${git ? (clean ? '<span class="pill pill-green">Clean</span>' : '<span class="pill pill-amber">Dirty</span>') : '<span class="pill pill-gray">No Git</span>'}
    </h3>
    ${git ? `
      <div class="kv"><span class="kv-key">Branch</span><span class="kv-val mono">${esc(git.branch || "detached")}</span></div>
      <div class="kv"><span class="kv-key">Staged files</span><span class="kv-val tabular">${git.staged.length}</span></div>
      <div class="kv"><span class="kv-key">Modified files</span><span class="kv-val tabular">${git.unstaged.length}</span></div>
      <div class="kv"><span class="kv-key">Untracked files</span><span class="kv-val tabular">${git.untracked.length}</span></div>`
      : '<p class="card-sub">This project has no Git repository, so working-tree state is unavailable.</p>'}
    <button class="card-link" id="wt-link">View in Git ${icon("arrowRight")}</button>`;
  card.querySelector("#wt-link").addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/git`));
  return card;
}
function allValidations() {
  return (S.changeSets || []).flatMap((set) => set.validations || []);
}
function validationCard() {
  const card = document.createElement("div");
  card.className = "card";
  const proposals = (S.project.discovery && S.project.discovery.proposed_commands) || [];
  const latest = allValidations().slice(0, 4);
  const allPass = latest.length && latest.every((v) => v.status === "passed");
  card.innerHTML = `
    <h3>${icon("verification")} Validation Status
      ${latest.length ? `<span class="pill ${allPass ? "pill-green" : "pill-red"}">${allPass ? icon("checkCircle") : ""} All passing</span>` : ""}</h3>
    <p class="card-sub">Run the project's real checks. "Files edited" is never "done".</p>
    <div class="mini-list">
      ${latest.length ? latest.map((v) => `
        <div class="mini-row">
          <span class="${v.status === "passed" ? "check-yes" : "check-no"}">${v.status === "passed" ? "✓" : "✗"}</span>
          <span style="flex:1"><b>${esc(v.role)}</b> · <span class="mono">${esc(v.command)}</span></span>
          <span class="pill ${v.status === "passed" ? "pill-green" : v.status === "failed" ? "pill-red" : "pill-amber"}">${esc(v.status)}</span>
        </div>`).join("") : '<div class="empty-state">No validations recorded yet.</div>'}
    </div>
    <div class="inline-form" style="margin-top:10px">
      <input class="input" id="val-command" list="val-proposals" placeholder="${esc(proposals[0] ? proposals[0].command : "cargo test / npm test …")}">
      <datalist id="val-proposals">${proposals.map((p) => `<option value="${esc(p.command)}">${esc(p.role)}</option>`).join("")}</datalist>
      <button class="btn btn-primary btn-sm" id="val-run">${icon("play")} Run</button>
    </div>`;
  card.querySelector("#val-run").addEventListener("click", async () => {
    const command = card.querySelector("#val-command").value.trim()
      || (proposals[0] ? proposals[0].command : "");
    if (!command) { toast("Nothing to run.", "error"); return; }
    try {
      const record = await invoke("validation_run", {
        projectId: S.route.projectId, taskId: "task-manual-local", command,
      });
      toast(`Validation ${record.status}.`, record.status === "passed" ? "success" : "error");
      refresh();
    } catch (e) { toast(String(e), "error"); }
  });
  return card;
}
function recentArtifactsCard() {
  const card = document.createElement("div");
  card.className = "card";
  const artifacts = (S.artifacts || []).slice(0, 4);
  card.innerHTML = `
    <h3>${icon("artifact")} Recent Artifacts
      ${(S.artifacts || []).length ? `<span class="pill pill-blue tabular">${S.artifacts.length} new</span>` : ""}</h3>
    <div class="mini-list">
      ${artifacts.length ? artifacts.map((a) => `
        <div class="mini-row">
          ${icon("fileText")}
          <span style="flex:1" class="mono">${esc(a.name)}</span>
          <span class="muted small">${esc(fmtSize(a.size))}</span>
        </div>`).join("") : '<div class="empty-state">No artifacts yet.</div>'}
    </div>
    <button class="card-link" id="see-artifacts">View All Artifacts ${icon("arrowRight")}</button>`;
  card.querySelector("#see-artifacts").addEventListener("click", () =>
    setRoute(`#/p/${S.route.projectId}/artifacts`));
  return card;
}
function recentChangesCard() {
  const card = document.createElement("div");
  card.className = "card";
  const entries = (S.changeSets || []).flatMap((set) => set.entries || []).slice(0, 6);
  card.innerHTML = `
    <h3>${icon("edit")} Recent Lumi Changes
      <button class="card-link" id="see-changes" style="margin-left:auto">View in Changes ${icon("arrowRight")}</button></h3>
    <div class="mini-list">
      ${entries.length ? entries.map((e) => `
        <div class="mini-row">
          ${icon("file")}
          <span style="flex:1" class="mono">${esc(e.path)}</span>
          <span class="pill pill-blue">${esc(e.kind)}</span>
          <span class="muted small">${esc(timeAgo(e.recorded_at))}</span>
        </div>`).join("") : '<div class="empty-state">Lumi has not changed anything in this project yet.</div>'}
    </div>`;
  card.querySelector("#see-changes").addEventListener("click", () =>
    setRoute(`#/p/${S.route.projectId}/changes`));
  return card;
}
function taskHistoryCard() {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `
    <h3>${icon("list")} Task History
      <button class="card-link" id="see-tasks" style="margin-left:auto">View All ${icon("arrowRight")}</button></h3>
    ${(S.tasks || []).length ? `<div class="mini-list">
      ${S.tasks.slice(0, 6).map((t) => `
        <div class="task-row" data-task="${esc(t.task_id)}">
          <span class="task-row-goal" style="flex:1">${esc(t.goal)}</span>
          ${taskStatusPill(t.status)}
          <span class="muted small">${esc(timeAgo(t.created_at))}</span>
        </div>`).join("")}
    </div>` : '<div class="empty-state">No tasks yet. Create one to delegate real work.</div>'}`;
  card.querySelector("#see-tasks").addEventListener("click", () =>
    setRoute(`#/p/${S.route.projectId}/tasks`));
  card.querySelectorAll("[data-task]").forEach((row) =>
    row.addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/task/${row.dataset.task}`)));
  return card;
}

// ================= tasks =================
function tasksView() {
  const wrap = document.createElement("div");
  wrap.innerHTML = `
    <div class="card">
      <h3>${icon("task")} New Task</h3>
      <p class="card-sub">Give Lumi a goal for this project. The task is durable and resumes after restart.</p>
      <div class="field"><textarea class="textarea" id="task-goal" placeholder="e.g. Implement OAuth login and make all tests pass"></textarea></div>
      <button class="btn btn-primary" id="task-create">${icon("plus")} Create Task</button>
    </div>
    <div class="card" style="margin-top:13px">
      <h3>${icon("list")} Tasks</h3>
      <div id="task-rows"></div>
    </div>`;
  wrap.querySelector("#task-create").addEventListener("click", async () => {
    const goal = wrap.querySelector("#task-goal").value.trim();
    if (!goal) { toast("Describe the goal first.", "error"); return; }
    try {
      const task = await invoke("task_create", { projectId: S.route.projectId, goal });
      toast("Task created.", "success");
      setRoute(`#/p/${S.route.projectId}/task/${encodeURIComponent(task.task_id)}`);
    } catch (e) { toast(String(e), "error"); }
  });
  const rows = wrap.querySelector("#task-rows");
  if (!S.tasks || !S.tasks.length) {
    rows.innerHTML = `<div class="empty-state">${icon("inbox") && ""}No tasks yet.</div>`;
  } else {
    for (const t of S.tasks) {
      const row = document.createElement("div");
      row.className = "task-row";
      row.innerHTML = `
        <span class="task-row-goal" style="flex:1">${esc(t.goal)}</span>
        ${taskStatusPill(t.status)}
        <span class="muted small">${esc(timeAgo(t.created_at))}</span>`;
      row.addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/task/${encodeURIComponent(t.task_id)}`));
      rows.append(row);
    }
  }
  return wrap;
}
async function taskDetailView(taskId) {
  let task = (S.tasks || []).find((t) => t.task_id === taskId);
  let changeSet = null;
  try {
    changeSet = await invoke("change_set", { projectId: S.route.projectId, taskId });
    if (changeSet && changeSet.__unavailable) changeSet = null;
  } catch { /* no change set yet */ }

  const wrap = document.createElement("div");
  if (!task) {
    wrap.className = "card";
    wrap.innerHTML = `<div class="empty-state">Task not found in this project's durable state.</div>`;
    return wrap;
  }
  const stageDefs = [
    ["Understand", task.status !== "CREATED"],
    ["Work", ["RUNNING", "WAITING_APPROVAL", "WAITING_USER", "PAUSED", "COMPLETED"].includes(task.status)],
    ["Validate", !!(changeSet && changeSet.validations && changeSet.validations.length)],
    ["Review", task.status === "COMPLETED"],
  ];
  const doneCount = stageDefs.filter(([, d]) => d).length;
  const pct = Math.round((doneCount / stageDefs.length) * 100);
  const entries = (changeSet && changeSet.entries) || [];
  const validations = (changeSet && changeSet.validations) || [];
  const commands = (changeSet && changeSet.commands) || [];

  wrap.innerHTML = `
    <div class="card" style="margin-bottom:13px">
      <div style="display:flex;align-items:flex-start;gap:12px">
        <div style="flex:1">
          <h1 class="project-title" style="font-size:17px">${esc(task.goal)}</h1>
          <div class="project-chips" style="margin-top:8px">
            ${taskStatusPill(task.status)}
            <span class="chip">${icon("layers")} ${esc((task.project_binding && task.project_binding.workspace_kind) || "")}</span>
            <span class="chip">${icon("clock")} ${esc(timeAgo(task.created_at))}</span>
          </div>
        </div>
        <div class="context-card" style="min-width:130px">
          <div class="muted small tabular" style="text-align:right">${doneCount} of ${stageDefs.length} · <b style="font-size:16px;color:var(--color-civic-navy)">${pct}%</b></div>
          <div class="progress-track"><div class="progress-fill" style="width:${pct}%"></div></div>
        </div>
      </div>
      <div class="stepper">
        ${stageDefs.map(([label, done]) => `
          <div class="step ${done ? "done" : ""}">
            <div class="step-dot">${done ? icon("check") : ""}</div>
            <div class="step-label">${esc(label)}</div>
          </div>`).join("")}
      </div>
    </div>
    <div class="card">
      <div class="subtabs">
        <button class="subtab active" data-sub="timeline">Timeline</button>
        <button class="subtab" data-sub="files">Changed Files <span class="tabular">${entries.length}</span></button>
        <button class="subtab" data-sub="validations">Validations <span class="tabular">${validations.length}</span></button>
        <button class="subtab" data-sub="commands">Commands <span class="tabular">${commands.length}</span></button>
      </div>
      <div id="task-sub-content"></div>
    </div>`;

  const subContent = wrap.querySelector("#task-sub-content");
  const renderSub = (sub) => {
    wrap.querySelectorAll(".subtab").forEach((b) => b.classList.toggle("active", b.dataset.sub === sub));
    if (sub === "files") {
      subContent.innerHTML = entries.length ? `<div class="mini-list">${entries.map((e) => `
        <div class="mini-row">${icon("file")}
          <span style="flex:1" class="mono">${esc(e.path)}</span>
          <span class="pill pill-blue">${esc(e.kind)}</span>
          <span class="muted small">${esc(timeAgo(e.recorded_at))}</span>
        </div>`).join("")}</div>` : '<div class="empty-state">No file changes recorded.</div>';
    } else if (sub === "validations") {
      subContent.innerHTML = validations.length ? `<div class="mini-list">${validations.map((v) => `
        <div class="mini-row">
          <span class="${v.status === "passed" ? "check-yes" : "check-no"}">${v.status === "passed" ? "✓" : "✗"}</span>
          <span style="flex:1" class="mono">${esc(v.command)}</span>
          <span class="pill ${v.status === "passed" ? "pill-green" : v.status === "failed" ? "pill-red" : "pill-amber"}">${esc(v.status)}</span>
        </div>`).join("")}</div>` : '<div class="empty-state">No validations run for this task yet.</div>';
    } else if (sub === "commands") {
      subContent.innerHTML = commands.length ? `<div class="mini-list">${commands.map((c) => `
        <div class="mini-row">${icon("play")}
          <span style="flex:1" class="mono">${esc(c.command)}</span>
          <span class="muted small">${esc(c.purpose || "")}</span>
        </div>`).join("")}</div>` : '<div class="empty-state">No commands recorded.</div>';
    } else {
      subContent.innerHTML = `<div class="terminal" id="task-console">
        ${consoleTimeline(task, entries, validations, commands)}
      </div>`;
    }
  };
  wrap.querySelectorAll(".subtab").forEach((b) =>
    b.addEventListener("click", () => renderSub(b.dataset.sub)));
  renderSub("timeline");
  return wrap;
}
function consoleTimeline(task, entries, validations, commands) {
  const t = task.created_at || TimestampNow();
  const line = (time, cls, text) =>
    `<div class="t-line"><span class="t-time">${esc((time || "").slice(11, 19) || "--:--:--")}</span><span class="${cls}">${esc(text)}</span></div>`;
  let html = "";
  html += line(t, "t-info", `> Task registered: ${task.goal}`);
  html += line(t, "t-dim", `> Workspace: ${(task.project_binding && task.project_binding.workspace_kind) || "unbound"}`);
  for (const e of entries) {
    html += line(e.recorded_at, "t-ok", `✓ ${cap(e.kind)}: ${e.path}${e.source === "external_conflict" ? " (external conflict preserved)" : ""}`);
  }
  for (const c of commands) {
    html += line(c.recorded_at, "t-cmd", `$ ${c.command}`);
  }
  for (const v of validations) {
    const cls = v.status === "passed" ? "t-ok" : v.status === "failed" ? "t-err" : "t-info";
    html += line(v.recorded_at, cls, `${v.status === "passed" ? "✓" : "✗"} ${v.command} → ${v.status}`);
  }
  html += line(task.created_at, "t-dim", "> Progress is durable — this task survives restart and resumes here.");
  return html;
}
function TimestampNow() { return new Date().toISOString(); }

// ================= files =================
async function filesView() {
  const wrap = document.createElement("div");
  wrap.className = "files-layout";
  const treePanel = document.createElement("div");
  treePanel.className = "card tree-panel";
  treePanel.innerHTML = `<h3>${icon("layers")} Files</h3><div id="tree"></div>`;
  const viewer = document.createElement("div");
  viewer.className = "file-viewer";
  const context = document.createElement("div");
  context.className = "card context-card";
  context.innerHTML = `<h4>${icon("file")} File Context</h4><div class="empty-state">Select a file.</div>`;
  wrap.append(treePanel, viewer, context);
  await loadTreeNode(S.route.projectId, ".", treePanel.querySelector("#tree"), viewer, context);
  return wrap;
}
async function loadTreeNode(projectId, relPath, container, viewer, context) {
  const entries = await invoke("file_list", { projectId, path: relPath });
  container.innerHTML = "";
  if (entries && entries.__unavailable) { container.innerHTML = UNAVAILABLE; return; }
  if (!entries || !entries.length) {
    container.innerHTML = '<div class="empty-state">Empty folder.</div>';
    return;
  }
  for (const entry of entries) {
    const item = document.createElement("button");
    item.className = "tree-item";
    item.innerHTML = `${entry.is_dir ? icon("folder") : icon("file")}<span style="flex:1;overflow:hidden;text-overflow:ellipsis">${esc(entry.name)}</span>`;
    item.addEventListener("click", async () => {
      if (entry.is_dir) {
        const sub = document.createElement("div");
        sub.style.paddingLeft = "14px";
        item.after(sub);
        item.disabled = true;
        await loadTreeNode(projectId, entry.path, sub, viewer, context);
        item.disabled = false;
      } else {
        container.querySelectorAll(".tree-item").forEach((i) => i.classList.remove("active"));
        item.classList.add("active");
        await openFile(projectId, entry.path, viewer, context);
      }
    });
    container.append(item);
  }
}
function highlight(content, lang) {
  const kw = {
    JavaScript: "const|let|var|function|return|if|else|for|while|import|export|from|class|new|await|async|try|catch|throw",
    TypeScript: "const|let|var|function|return|if|else|for|while|import|export|from|class|new|await|async|try|catch|throw|interface|type|implements|extends",
    Rust: "fn|let|mut|pub|struct|enum|impl|match|if|else|for|while|loop|return|use|mod|crate|self|Self|where|async|await|trait",
    Python: "def|class|return|if|elif|else|for|while|import|from|as|with|try|except|raise|lambda|pass|yield|global|assert",
    Go: "func|package|import|return|if|else|for|range|go|defer|var|const|type|struct|interface|map|chan",
  }[lang];
  let html = esc(content);
  if (kw) {
    html = html
      .replace(/(&quot;[^&]*?&quot;|'[^']*?'|`[^`]*?`)/g, '<span class="tok-str">$1</span>')
      .replace(/(^|\n)(\s*)(\/\/[^\n]*|#(?![0-9])[^*\n]*)/g, '$1$2<span class="tok-com">$3</span>')
      .replace(new RegExp(`\\b(${kw})\\b`, "g"), '<span class="tok-kw">$1</span>')
      .replace(/\b(\d+(\.\d+)?)\b/g, '<span class="tok-num">$1</span>');
  } else if (lang === "Markdown") {
    html = html
      .replace(/^(#{1,6} .*)$/gm, '<span class="tok-kw">$1</span>')
      .replace(/(\*\*[^*]+\*\*)/g, '<span class="tok-kw">$1</span>');
  }
  return html;
}
async function openFile(projectId, path, viewer, context) {
  let file;
  try {
    file = await invoke("file_read", { projectId, path });
    if (file && file.__unavailable) return;
  } catch (error) {
    viewer.innerHTML = `<div class="empty-state" style="margin:14px">${esc(String(error))}</div>`;
    return;
  }
  // Tab registry: keep open files, activate the clicked one.
  const existing = S.openFiles.find((f) => f.path === path);
  if (existing) { existing.content = file.content; existing.sha256 = file.sha256; }
  else S.openFiles.push({ path, content: file.content, sha256: file.sha256 });
  S.activeFile = path;
  S.editing = false;
  renderFileArea(projectId, viewer, context);
}
function renderFileArea(projectId, viewer, context) {
  const file = S.openFiles.find((f) => f.path === S.activeFile);
  if (!file) return;
  const lines = file.content.split("\n");
  const lang = langOf(file.path);
  const segments = file.path.split("/");
  const crumbs = segments
    .map((s, i) => (i === segments.length - 1
      ? `<b style="color:var(--color-civic-navy)">${esc(s)}</b>`
      : `<span>${esc(s)}</span><span>›</span>`))
    .join("");

  viewer.innerHTML = `
    <div class="file-tabs" id="file-tabs">
      ${S.openFiles.map((f) => `
        <button class="file-tab ${f.path === S.activeFile ? "active" : ""}" data-tab-path="${esc(f.path)}">
          ${icon("file")} ${esc(f.path.split("/").pop())}
          <span class="ft-close" data-close="${esc(f.path)}">×</span>
        </button>`).join("")}
    </div>
    <div class="file-viewer-head">
      <span class="crumbs">${icon("folder")} ${crumbs}</span>
      <span style="flex:1"></span>
      <button class="btn btn-sm" id="file-copy">${icon("copy")} Copy path</button>
      <button class="btn btn-sm" id="file-edit-toggle">${S.editing ? "Cancel" : icon("edit") + " Edit"}</button>
      <button class="btn btn-primary btn-sm" id="file-save" disabled>${icon("check")} Save</button>
    </div>
    <div id="file-body">
      ${S.editing
        ? `<textarea class="editor-area" id="editor" spellcheck="false">${esc(file.content)}</textarea>`
        : `<div class="code-body">
            <div class="code-lines">${lines.map((_, i) => i + 1).join("<br>")}</div>
            <div class="code-content">${highlight(file.content, lang)}</div>
          </div>`}
    </div>
    <div class="status-bar">
      <span>${esc(file.path.split("/").pop())}</span>
      <span class="sb-right">
        <span class="tabular">${lines.length} lines</span>
        <span>${esc(fmtSize(new Blob([file.content]).size))}</span>
        <span>${esc(lang)}</span><span>UTF-8</span><span>LF</span>
      </span>
    </div>`;

  const related = (S.changeSets || [])
    .flatMap((set) => (set.entries || []))
    .filter((e) => e.path === file.path)
    .slice(0, 4);
  context.innerHTML = `
    <h4>${icon("file")} File Context</h4>
    <button class="card-link" id="ctx-editor" style="margin-bottom:8px">${icon("external")} Open in Editor</button>
    <div class="kv"><span class="kv-key">Lines</span><span class="kv-val tabular">${lines.length}</span></div>
    <div class="kv"><span class="kv-key">Size</span><span class="kv-val tabular">${esc(fmtSize(new Blob([file.content]).size))}</span></div>
    <div class="kv"><span class="kv-key">Language</span><span class="kv-val">${esc(lang)}</span></div>
    <div class="kv"><span class="kv-key">SHA-256</span><span class="kv-val mono">${esc(file.sha256.slice(0, 12))}…</span></div>
    <h4 style="margin-top:12px">${icon("history")} Recent changes to this file</h4>
    ${related.length ? `<div class="mini-list">${related.map((e) => `
      <div class="mini-row"><span class="pill pill-blue">${esc(e.kind)}</span>
      <span class="muted small">${esc(timeAgo(e.recorded_at))}</span></div>`).join("")}</div>`
      : '<div class="muted small">No Lumi changes recorded for this file.</div>'}
    <p class="card-sub" style="margin-top:10px">Edits are checksum-guarded: if the file changes on disk before you save, Lumi refuses and preserves the newer version.</p>`;

  viewer.querySelector("#file-copy").addEventListener("click", async () => {
    try { await navigator.clipboard.writeText(file.path); toast("Path copied.", "success"); }
    catch { toast("Copy failed.", "error"); }
  });
  viewer.querySelector("#ctx-editor").addEventListener("click", () => {
    if (hasTauri() && window.__TAURI__.opener) {
      window.__TAURI__.opener.revealItemInDir(`${S.project.primary_root}/${file.path}`);
    } else toast("Available in the desktop app.");
  });
  viewer.querySelector("#file-edit-toggle").addEventListener("click", () => {
    S.editing = !S.editing;
    renderFileArea(projectId, viewer, context);
  });
  const saveBtn = viewer.querySelector("#file-save");
  if (saveBtn) {
    saveBtn.disabled = !S.editing;
    saveBtn.addEventListener("click", async () => {
      const editor = viewer.querySelector("#editor");
      try {
        await invoke("file_edit", {
          projectId, path: file.path, expectedSha256: file.sha256, content: editor.value,
        });
        toast("Saved.", "success");
        S.editing = false;
        const fresh = await invoke("file_read", { projectId, path: file.path });
        const at = S.openFiles.findIndex((f) => f.path === file.path);
        if (at >= 0 && fresh && !fresh.__unavailable) S.openFiles[at] = { path: file.path, ...fresh };
        renderFileArea(projectId, viewer, context);
      } catch (error) {
        toast(String(error), "error");
      }
    });
  }
  viewer.querySelectorAll("[data-tab-path]").forEach((tab) => {
    tab.addEventListener("click", (e) => {
      if (e.target.dataset.close !== undefined) {
        S.openFiles = S.openFiles.filter((f) => f.path !== e.target.dataset.close);
        if (S.activeFile === e.target.dataset.close) {
          S.activeFile = S.openFiles.length ? S.openFiles[S.openFiles.length - 1].path : null;
        }
        if (!S.activeFile) { refresh(); return; }
      } else {
        S.activeFile = e.currentTarget.dataset.tabPath;
      }
      S.editing = false;
      renderFileArea(projectId, viewer, context);
    });
  });
}

// ================= changes =================
async function changesView() {
  const sets = await invoke("change_sets", { projectId: S.route.projectId });
  const wrap = document.createElement("div");
  wrap.innerHTML = `<div id="changes-body"></div>`;
  const body = wrap.querySelector("#changes-body");
  if (!sets || sets.__unavailable) { body.innerHTML = UNAVAILABLE; return wrap; }
  if (!sets.length) {
    body.innerHTML = `<div class="card"><div class="empty-state"><span class="big">${icon("edit")}</span>
      No changes yet. Changes appear here as tasks and edits modify project files —
      with checksums, patches, and validations attached.</div></div>`;
    return wrap;
  }
  const totals = { created: 0, modified: 0, moved: 0, deleted: 0 };
  for (const set of sets) for (const e of set.entries || []) {
    if (totals[e.kind] != null) totals[e.kind] += 1;
  }
  body.innerHTML = `
    <div class="stats-row">
      <div class="stat-tile"><span class="tone-green">${icon("plus")}</span><div><div class="num">${totals.created}</div><div class="lbl">Files created</div></div></div>
      <div class="stat-tile"><span class="tone-blue">${icon("edit")}</span><div><div class="num">${totals.modified}</div><div class="lbl">Files modified</div></div></div>
      <div class="stat-tile"><span class="tone-amber">${icon("arrowRight")}</span><div><div class="num">${totals.moved}</div><div class="lbl">Files moved</div></div></div>
      <div class="stat-tile"><span class="tone-red">${icon("trash")}</span><div><div class="num">${totals.deleted}</div><div class="lbl">Files deleted</div></div></div>
    </div>
    <div class="diff-toolbar">
      <div class="diff-toggle">
        <button id="diff-unified" class="active">Unified</button>
        <button id="diff-split">Side by Side</button>
      </div>
      <span class="muted small">Pre-existing user edits live in Git, never here. Lumi's change set answers: what did Lumi change, and is it correct?</span>
    </div>
    <div id="sets"></div>`;
  const container = body.querySelector("#sets");
  const renderSets = () => {
    container.innerHTML = "";
    for (const set of sets) container.append(changeSetCard(set));
  };
  body.querySelector("#diff-unified").addEventListener("click", () => {
    S.diffMode = "unified";
    body.querySelector("#diff-unified").classList.add("active");
    body.querySelector("#diff-split").classList.remove("active");
    renderSets();
  });
  body.querySelector("#diff-split").addEventListener("click", () => {
    S.diffMode = "split";
    body.querySelector("#diff-split").classList.add("active");
    body.querySelector("#diff-unified").classList.remove("active");
    renderSets();
  });

  const changeSetCard = (set) => {
    const card = document.createElement("div");
    card.className = "card";
    card.style.marginBottom = "12px";
    const validations = set.validations || [];
    const allPass = validations.length && validations.every((v) => v.status === "passed");
    card.innerHTML = `
      <h3>Task <span class="mono">${esc(set.task_id)}</span>
        ${allPass ? '<span class="pill pill-green">Validated</span>' : validations.length ? '<span class="pill pill-red">Validation failed</span>' : ""}
      </h3>
      <p class="card-sub tabular">${(set.entries || []).length} file change(s) · ${validations.length} validation(s) · updated ${esc(timeAgo(set.updated_at))}</p>
      ${(set.entries || []).map((e) => renderChangeEntry(e, S.diffMode)).join("")}
      ${validations.map((v) => `
        <div class="mini-row">
          <span class="${v.status === "passed" ? "check-yes" : "check-no"}">${v.status === "passed" ? "✓" : "✗"}</span>
          <span style="flex:1" class="mono">${esc(v.command)}</span>
          <span class="pill ${v.status === "passed" ? "pill-green" : v.status === "failed" ? "pill-red" : "pill-amber"}">${esc(v.status)}</span>
        </div>`).join("")}
    `;
    return card;
  };
  renderSets();
  container.querySelectorAll("[data-toggle-diff]").forEach((b) =>
    b.addEventListener("click", () => {
      const block = document.getElementById(b.dataset.toggleDiff);
      if (block) block.hidden = !block.hidden;
    }));
  return wrap;
}
function renderChangeEntry(entry, mode) {
  const anchor = `patch-${entry.path.replace(/[^a-z0-9]/gi, "")}`;
  const hasPatch = !!entry.patch;
  return `
    <div class="mini-row">
      ${icon("file")}
      <span style="flex:1" class="mono">${esc(entry.path)}</span>
      <span class="pill pill-blue">${esc(entry.kind)}</span>
      ${hasPatch ? `<button class="card-link" data-toggle-diff="${anchor}">${icon("eye")} Diff</button>` : ""}
    </div>
    <div id="${anchor}" class="diff-block" ${entry === visiblePatchEntry ? "" : "hidden"}>${hasPatch ? renderPatch(entry.patch, S.diffMode) : ""}</div>`;
}
function renderPatch(patch, mode) {
  const lines = patch.split("\n");
  if (mode === "split") {
    const rows = [];
    let pendingDel = [];
    for (const line of lines) {
      if (line.startsWith("@@")) {
        flushPair();
        rows.push(`<div class="line meta">${esc(line)}</div>`);
      } else if (line.startsWith("-")) pendingDel.push(line);
      else if (line.startsWith("+")) {
        flushPair();
        rows.push(splitRow("", `<div class="line add">${esc(line)}</div>`));
      } else {
        flushPair();
        rows.push(splitRow(line, line));
      }
    }
    function flushPair() {
      if (pendingDel.length) {
        for (const d of pendingDel) rows.push(splitRow(`<div class="line del">${esc(d)}</div>`, ""));
        pendingDel = [];
      }
    }
    function splitRow(before, after) {
      return `<div style="display:contents">
        <div class="pane"><div class="line ${before ? "del" : "ghost"}">${typeof before === "string" && before.startsWith("<") ? before : esc(before || " ")}</div></div>
        <div class="pane">${after || '<div class="line ghost"> </div>'}</div>
      </div>`;
    }
    return `<div class="diff-split">
      <div class="pane"><div class="line meta">Before</div></div>
      <div class="pane"><div class="line meta">After (Lumi changes)</div></div>
      ${rows.join("")}
    </div>`;
  }
  return lines.map((line) => {
    const cls = line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : "meta";
    return `<div class="line ${cls}">${esc(line)}</div>`;
  }).join("");
}

// ================= git =================
async function gitView() {
  const wrap = document.createElement("div");
  wrap.innerHTML = `
    <div class="section-head"><h2>Git Workspace</h2></div>
    <p class="section-sub">Manage branches, review changes, and collaborate with confidence.</p>
    <div id="git-body"></div>
    <div class="project-layout" style="margin-top:13px">
      <div class="card">
        <h3>${icon("shield")} Git Policy &amp; Safety</h3>
        <p class="card-sub"><b>Auto-approved (safe):</b> view history, create branches, read diffs, switch branches.</p>
        <p class="card-sub" style="margin:0"><b>Approval required:</b> push to remote, force push, history rewrite, remote branch deletion. These never run without explicit authority — they are not exposed to the local runtime at all.</p>
      </div>
    </div>`;
  const body = wrap.querySelector("#git-body");
  if (!S.git) {
    body.innerHTML = `<div class="empty-state"><span class="big">${icon("branch")}</span>This project has no Git repository, so Git tools are unavailable.</div>`;
    return wrap;
  }
  const git = S.git;
  const log = await invoke("git_log", { projectId: S.route.projectId, limit: 8 });
  const branches = await invoke("git_branches", { projectId: S.route.projectId });
  const validations = allValidations();
  const dirty = !gitIsClean(git);
  const readiness = [
    { label: "Working tree", detail: dirty ? `${gitChanged(git)} change(s)` : "clean", ok: !dirty },
    { label: "Validations", detail: validations.length ? (validations.every((v) => v.status === "passed") ? "all passing" : "attention needed") : "none recorded", ok: validations.length && validations.every((v) => v.status === "passed") },
    { label: "Branch", detail: git.branch || "detached", ok: !!git.branch },
    { label: "Push", detail: "requires approval — not exposed locally", ok: null },
  ];
  body.innerHTML = `
    <div class="stat-cards" style="grid-template-columns: 1fr 2fr">
      <div class="card">
        <h3>${icon("branch")} Current Branch ${gitIsClean(git) ? '<span class="pill pill-green">Clean</span>' : '<span class="pill pill-amber">Dirty</span>'}</h3>
        <p style="font-size:15px;font-weight:700;margin:6px 0" class="mono">${esc(git.branch || "detached HEAD")}</p>
        <div class="kv"><span class="kv-key">Staged</span><span class="kv-val tabular">${git.staged.length}</span></div>
        <div class="kv"><span class="kv-key">Unstaged</span><span class="kv-val tabular">${git.unstaged.length}</span></div>
        <div class="kv"><span class="kv-key">Untracked</span><span class="kv-val tabular">${git.untracked.length}</span></div>
      </div>
      <div class="card">
        <h3>${icon("diff")} Changes in Working Directory</h3>
        ${dirty ? `
          <table class="status-table">
            <tr><th></th><th>File</th><th>Status</th></tr>
            ${git.staged.map((f) => gitRow(f.path, f.index_state, "staged")).join("")}
            ${git.unstaged.map((f) => gitRow(f.path, f.worktree_state, "unstaged")).join("")}
            ${git.untracked.map((p) => gitRow(p, "?", "untracked")).join("")}
          </table>` : '<div class="empty-state">Working tree is clean.</div>'}
        <div class="inline-form" style="margin-top:12px">
          <input class="input" id="commit-msg" placeholder="Commit message (commits exactly the files you ticked)">
          <button class="btn btn-primary btn-sm" id="commit-btn">${icon("commit")} Commit Selected</button>
        </div>
        <p class="card-sub" style="margin-top:6px">Commits are path-scoped: Lumi never sweeps in your other staged work.</p>
      </div>
    </div>
    <div class="card" style="margin-bottom:13px">
      <h3>${icon("checkCircle")} Readiness</h3>
      <div class="mini-list">
        ${readiness.map((r) => `
          <div class="mini-row">
            <span class="${r.ok == null ? "check-no" : r.ok ? "check-yes" : "check-no"}">${r.ok == null ? "●" : r.ok ? "✓" : "○"}</span>
            <span style="flex:1"><b>${esc(r.label)}</b></span>
            <span class="muted small">${esc(r.detail)}</span>
          </div>`).join("")}
      </div>
    </div>
    <div class="two-cards">
      <div class="card">
        <h3>${icon("branch")} Branches</h3>
        <div class="inline-form" style="margin-bottom:10px">
          <input class="input" id="new-branch" placeholder="feature/my-branch">
          <button class="btn btn-sm" id="create-branch">Create</button>
        </div>
        <div class="mini-list">
          ${(branches || []).map((b) => `
            <div class="mini-row">
              <span class="mono" style="flex:1">${esc(b)}</span>
              ${b === git.branch ? '<span class="pill pill-green">Current</span>' : `<button class="card-link" data-switch="${esc(b)}">Switch</button>`}
            </div>`).join("")}
        </div>
      </div>
      <div class="card">
        <h3>${icon("commit")} Recent Commits</h3>
        <div class="mini-list">
          ${(log || []).map((c) => `
            <div class="mini-row">
              <span class="mono">${esc(c.short_hash)}</span>
              <span style="flex:1">${esc(c.subject)}</span>
              <span class="muted small">${esc(c.author)}</span>
            </div>`).join("") || '<div class="empty-state">No commits yet.</div>'}
        </div>
      </div>
    </div>`;

  const selected = new Set();
  body.querySelectorAll("input[data-gitted]").forEach((box) =>
    box.addEventListener("change", () => {
      if (box.checked) selected.add(box.dataset.gitted); else selected.delete(box.dataset.gitted);
    }));
  body.querySelector("#commit-btn").addEventListener("click", async () => {
    const message = body.querySelector("#commit-msg").value.trim();
    if (!message || !selected.size) { toast("Tick files and write a message first.", "error"); return; }
    try {
      await invoke("git_commit", { projectId: S.route.projectId, message, paths: [...selected] });
      toast("Committed.", "success");
      refresh();
    } catch (e) { toast(String(e), "error"); }
  });
  body.querySelector("#create-branch").addEventListener("click", async () => {
    const name = body.querySelector("#new-branch").value.trim();
    if (!name) return;
    try { await invoke("git_create_branch", { projectId: S.route.projectId, name }); toast("Branch created.", "success"); refresh(); }
    catch (e) { toast(String(e), "error"); }
  });
  body.querySelectorAll("[data-switch]").forEach((b) =>
    b.addEventListener("click", async () => {
      try { await invoke("git_switch", { projectId: S.route.projectId, name: b.dataset.switch }); refresh(); }
      catch (e) { toast(String(e), "error"); }
    }));
  return wrap;
}
function gitRow(path, state, bucket) {
  const cls = state === "A" ? "added" : state === "D" ? "deleted" : state === "?" ? "untracked" : "";
  return `<tr>
    <td><input type="checkbox" data-gitted="${esc(path)}"></td>
    <td class="mono">${esc(path)}</td>
    <td><span class="state-chip ${cls}">${esc(state || "M")}</span> <span class="muted small">${esc(bucket)}</span></td>
  </tr>`;
}

// ================= artifacts =================
function artifactsView() {
  const wrap = document.createElement("div");
  const artifacts = S.artifacts || [];
  wrap.innerHTML = `
    <div class="section-head"><h2>Artifacts</h2></div>
    <p class="section-sub">Generated outputs under <span class="mono">.lumi/artifacts</span> — checksummed, provenance-linked, stored locally.</p>
    ${artifacts.length ? `
      <div class="card">
        <table class="status-table">
          <tr><th>Name</th><th>Type</th><th>Lifecycle</th><th>Size</th><th>Modified</th></tr>
          ${artifacts.map((a) => `
            <tr>
              <td class="mono">${icon("fileText")} ${esc(a.name)}</td>
              <td>${a.artifact_type ? `<span class="pill pill-blue">${esc(a.artifact_type)}</span>` : '<span class="muted">—</span>'}</td>
              <td>${a.lifecycle ? `<span class="pill ${a.lifecycle.toLowerCase() === "ready" || a.lifecycle.toLowerCase() === "published" ? "pill-green" : "pill-gray"}">${esc(a.lifecycle)}</span>` : '<span class="muted">—</span>'}</td>
              <td class="tabular">${esc(fmtSize(a.size))}</td>
              <td class="muted small">${esc(timeAgo(new Date(a.modified_at * 1000).toISOString()))}</td>
            </tr>`).join("")}
        </table>
      </div>`
      : `<div class="empty-state"><span class="big">${icon("artifact")}</span>
          No artifacts yet. Run a task that produces reports or exports and they will appear here with SHA-256 integrity refs.</div>`}`;
  return wrap;
}

// ================= evidence / approvals =================
function evidenceView() {
  const wrap = document.createElement("div");
  wrap.className = "card";
  const evidence = S.snapshot && S.snapshot.evidence;
  wrap.innerHTML = `
    <h3>${icon("list")} Evidence</h3>
    <p class="card-sub">Trust-labeled action history. "Attempted" is not "done".</p>
    ${evidence && evidence.length ? `<div class="mini-list">
      ${evidence.map((e) => `
        <div class="mini-row">
          <span style="flex:1">${esc(e.operation || e.action_id || "action")}</span>
          <span class="pill ${e.trust_label === "VERIFIED" ? "pill-green" : "pill-amber"}">${esc(e.trust_label || "unknown")}</span>
        </div>`).join("")}
    </div>` : `<div class="empty-state"><span class="big">${icon("inbox")}</span>No runtime evidence recorded yet.</div>`}`;
  return wrap;
}
function approvalsView() {
  const wrap = document.createElement("div");
  const exceptions = S.snapshot && S.snapshot.exceptions;
  const pending = S.snapshot && S.snapshot.pending_approvals;
  wrap.innerHTML = `
    <div class="card">
      <h3>${icon("checkCircle")} Approvals &amp; Exceptions</h3>
      <p class="card-sub">Consequential actions pause here for your explicit decision.</p>
      ${pending && pending.length ? pending.map((p) => `
        <div class="card" style="margin-bottom:10px">
          <b>${esc(p.card ? p.card.business_effect : p.action_digest)}</b>
          <div class="kv"><span class="kv-key">Digest</span><span class="kv-val mono small">${esc(p.action_digest)}</span></div>
        </div>`).join("") : `<div class="empty-state"><span class="big">${icon("checkCircle")}</span>Nothing needs your approval right now.</div>`}
      ${exceptions && exceptions.length ? exceptions.map((x) => `
        <div class="card" style="margin-top:10px">
          <h3><span class="pill pill-red">Exception</span> ${esc(x.what_blocked || "")}</h3>
          <p class="card-sub">${esc(x.why || "")}</p>
          ${(x.safe_choices || []).map((c) => `<div class="mini-row"><b>${esc(c.label)}</b><span class="muted">${esc(c.description)}</span></div>`).join("")}
        </div>`).join("") : ""}
    </div>`;
  return wrap;
}

// ================= settings =================
function settingsView() {
  const wrap = document.createElement("div");
  const p = S.project;
  wrap.innerHTML = `
    <div class="project-layout">
      <div>
        <div class="card" style="margin-bottom:13px">
          <h3>${icon("settings")} Project Settings</h3>
          <p class="card-sub">Real configuration for this project. Org policy and provider routing are governed by policy — a project may only narrow, never widen, authority.</p>
          <div class="kv"><span class="kv-key">Project ID</span><span class="kv-val mono">${esc(p.project_id)}</span></div>
          <div class="kv"><span class="kv-key">Project root</span><span class="kv-val mono">${esc(p.primary_root)}</span></div>
          <div class="kv"><span class="kv-key">Environment</span><span class="kv-val mono">${esc(p.environment_id)}</span></div>
          <div class="kv"><span class="kv-key">Detected source</span><span class="kv-val">${esc(p.detected_source)}</span></div>
          <div class="kv"><span class="kv-key">Instruction sources</span><span class="kv-val">${p.instructions.length ? esc(p.instructions.join(", ")) : "none found"}</span></div>
        </div>
        <div class="card" style="margin-bottom:13px">
          <h3>${icon("hash")} Capabilities (negotiated)</h3>
          <p class="card-sub">Only capabilities the runtime actually provides are advertised.</p>
          <div class="recent-meta">
            ${(p.capabilities || []).map((c) => `<span class="chip">${esc(c)}</span>`).join("") || '<span class="muted">none</span>'}
          </div>
        </div>
        <div class="card">
          <div class="inline-form">
            <button class="btn" id="btn-relink">${icon("refresh")} Relink root…</button>
            <button class="btn btn-danger-outline" id="btn-remove">${icon("trash")} Remove project</button>
          </div>
        </div>
        <div class="card" style="margin-top:13px" id="memory-panel">
          <h3>${icon("insight")} Project Memory</h3>
          <p class="card-sub">Validated knowledge with provenance. Records tied to a Git HEAD go stale when HEAD moves; invalidation is reasoned and audit-retained.</p>
          <div id="memory-rows"></div>
          <div class="inline-form" style="margin-top:10px">
            <input class="input" id="memory-content" placeholder="e.g. npm test validates the sync module">
            <input class="input" id="memory-command" placeholder="command (required for validated_command)">
            <select class="select" id="memory-kind" style="max-width:180px">
              <option value="validated_command">validated_command</option>
              <option value="convention">convention</option>
              <option value="environment_requirement">environment_requirement</option>
              <option value="recovery_procedure">recovery_procedure</option>
            </select>
            <button class="btn btn-primary btn-sm" id="memory-save">Remember</button>
          </div>
        </div>
      </div>
      <aside class="context-panel">
        <div class="card context-card">
          <h4>${icon("shield")} Security &amp; Trust Posture <span class="pill pill-green">Strict</span></h4>
          <ul class="check-list">
            <li><span class="check-yes">✓</span> Strict project boundary — only the authorized roots</li>
            <li><span class="check-yes">✓</span> Local execution — files stay on your machine</li>
            <li><span class="check-yes">✓</span> Identity proven by a project marker on disk</li>
            <li><span class="check-no">✕</span> Push / force-push / cleanup need external authority (not exposed)</li>
          </ul>
        </div>
        <div class="note-card">
          <h4>${icon("star")} Your work stays with you</h4>
          Local by design. Secure by default. In your control.
        </div>
      </aside>
    </div>`;
  wrap.querySelector("#btn-relink").addEventListener("click", () => {
    const path = prompt("New absolute path of the project root:");
    if (path) invoke("project_relink", { projectId: p.project_id, path }).then(refresh).catch((e) => toast(String(e), "error"));
  });
  wrap.querySelector("#btn-remove").addEventListener("click", () => {
    if (confirm("Remove this project from Lumi? The folder on disk is not touched.")) {
      invoke("project_remove", { projectId: p.project_id }).then(() => setRoute("#/projects")).catch((e) => toast(String(e), "error"));
    }
  });
  const memoryRows = wrap.querySelector("#memory-rows");
  invoke("memory_list", { projectId: p.project_id }).then((records) => {
    if (!records || records.__unavailable || !records.length) {
      memoryRows.innerHTML = '<div class="empty-state">No project memory yet. Validated commands and conventions appear here with provenance.</div>';
      return;
    }
    memoryRows.innerHTML = records.map(([record, trusted]) => `
      <div class="mini-row">
        <span class="${trusted ? "check-yes" : "check-no"}">${trusted ? "✓" : "✗"}</span>
        <span style="flex:1">${esc(record.content)}
          <span class="muted small mono">· ${esc(record.kind)} · ${esc(record.provenance.task_id)}</span></span>
        <span class="pill ${trusted ? "pill-green" : "pill-gray"}">${trusted ? "trusted" : "stale"}</span>
        ${trusted ? `<button class="card-link" data-invalidate="${esc(record.memory_id)}">Invalidate</button>` : ""}
      </div>`).join("");
    memoryRows.querySelectorAll("[data-invalidate]").forEach((b) =>
      b.addEventListener("click", () => {
        const reason = prompt("Reason for invalidating this memory:");
        if (!reason) return;
        invoke("memory_invalidate", { projectId: p.project_id, memoryId: b.dataset.invalidate, reason })
          .then(refresh).catch((e) => toast(String(e), "error"));
      }));
  }).catch((e) => { memoryRows.innerHTML = `<div class="empty-state">${esc(String(e))}</div>`; });
  wrap.querySelector("#memory-save").addEventListener("click", async () => {
    const content = wrap.querySelector("#memory-content").value.trim();
    const command = wrap.querySelector("#memory-command").value.trim() || null;
    const kind = wrap.querySelector("#memory-kind").value;
    if (!content) { toast("Describe the knowledge first.", "error"); return; }
    try {
      await invoke("memory_remember", {
        projectId: p.project_id,
        submission: {
          memory_id: `mem-${Date.now()}`,
          kind, content, task_id: "task-manual-local",
          command, evidence: null,
        },
      });
      toast("Remembered.", "success");
      refresh();
    } catch (e) { toast(String(e), "error"); }
  });
  return wrap;
}

// ================= command palette =================
let paletteProjects = [];
let paletteSelection = 0;
async function openPalette() {
  S.paletteOpen = true;
  document.getElementById("palette").classList.remove("hidden");
  const input = document.getElementById("palette-input");
  input.value = "";
  input.focus();
  paletteProjects = S.projects || [];
  S.paletteSection = "all";
  await renderPaletteResults("");
}
function closePalette() {
  S.paletteOpen = false;
  document.getElementById("palette").classList.add("hidden");
}
async function renderPaletteResults(query) {
  const results = document.getElementById("palette-results");
  const rail = document.getElementById("palette-rail");
  const inProject = S.route.view === "project";
  const q = query.trim().toLowerCase();

  const projects = paletteProjects.filter((p) => !q || p.display_name.toLowerCase().includes(q));
  let files = [];
  let tasks = [];
  if (inProject && q) {
    files = (await invoke("file_search", { projectId: S.route.projectId, query: q, mode: "filename" })) || [];
    if (files.__unavailable) files = [];
    tasks = (S.tasks || []).filter((t) => t.goal.toLowerCase().includes(q));
  }
  const groups = { Projects: projects, Files: files.slice(0, 6), Tasks: tasks.slice(0, 5) };
  if (rail.childElementCount === 0 || S.paletteSection) {
    rail.innerHTML = `
      <button class="rail-item ${S.paletteSection === "all" ? "active" : ""}" data-section="all">${icon("search")} All results</button>
      <div class="rail-section">Jump to</div>
      <button class="rail-item ${S.paletteSection === "projects" ? "active" : ""}" data-section="projects">${icon("folder")} Projects <span class="rail-count tabular">${projects.length}</span></button>
      ${inProject ? `
      <button class="rail-item ${S.paletteSection === "files" ? "active" : ""}" data-section="files">${icon("file")} Files <span class="rail-count tabular">${files.length}</span></button>
      <button class="rail-item ${S.paletteSection === "tasks" ? "active" : ""}" data-section="tasks">${icon("check")} Tasks <span class="rail-count tabular">${tasks.length}</span></button>` : ""}
      <div class="rail-section">Pro tip</div>
      <div class="muted small" style="padding:4px 9px;line-height:1.5">Type to search across ${inProject ? "files, tasks, and " : ""}projects.</div>`;
    rail.querySelectorAll("[data-section]").forEach((b) =>
      b.addEventListener("click", () => { S.paletteSection = b.dataset.section; renderPaletteResults(query); }));
  }

  let html = "";
  const sections = S.paletteSection === "all" ? Object.keys(groups) : [cap(S.paletteSection)];
  for (const key of sections) {
    const list = key === "Projects" ? projects : key === "Files" ? files.slice(0, 6) : tasks.slice(0, 5);
    if (!list.length) continue;
    html += `<div class="palette-group">${key} <span class="tabular">${list.length}</span></div>`;
    html += list.map((item) => {
      if (key === "Projects") {
        return `<button class="palette-item" data-go="#/p/${encodeURIComponent(item.project_id)}/home">
          ${icon("folder")}<span style="flex:1">${esc(item.display_name)}</span><span class="p-meta">${esc(item.health)}</span></button>`;
      }
      if (key === "Files") {
        return `<button class="palette-item" data-file="${esc(item.path)}">
          ${icon("file")}<span style="flex:1" class="mono">${esc(item.path)}</span><span class="p-meta">${esc(timeAgo(new Date((item.modified_at || 0) * 1000).toISOString()))}</span></button>`;
      }
      return `<button class="palette-item" data-task="${esc(item.task_id)}">
        ${icon("check")}<span style="flex:1">${esc(item.goal)}</span><span class="p-meta">${esc(item.status)}</span></button>`;
    }).join("");
  }
  results.innerHTML = html || `<div class="empty-state" style="margin:10px">No matches.</div>`;
  paletteSelection = 0;
  markSelected();
  results.querySelectorAll("[data-go]").forEach((b) => b.addEventListener("click", () => { closePalette(); setRoute(b.dataset.go); }));
  results.querySelectorAll("[data-file]").forEach((b) =>
    b.addEventListener("click", () => {
      closePalette();
      setRoute(`#/p/${S.route.projectId}/files`);
      setTimeout(async () => {
        const viewer = document.querySelector(".file-viewer");
        const context = document.querySelector(".context-card");
        if (viewer && context) await openFile(S.route.projectId, b.dataset.file, viewer, context);
      }, 300);
    }));
  results.querySelectorAll("[data-task]").forEach((b) =>
    b.addEventListener("click", () => {
      closePalette();
      setRoute(`#/p/${S.route.projectId}/task/${encodeURIComponent(b.dataset.task)}`);
    }));
}
function markSelected() {
  const items = document.querySelectorAll(".palette-item");
  items.forEach((el, i) => el.classList.toggle("selected", i === paletteSelection));
  if (items[paletteSelection]) items[paletteSelection].scrollIntoView({ block: "nearest" });
}

// ================= refresh + boot =================
let refreshSeq = 0;
async function refresh() {
  const seq = ++refreshSeq;
  await loadSnapshot();
  await loadProjects();
  if (seq !== refreshSeq) return;
  await render();
}
window.addEventListener("hashchange", refresh);
function boot() {
  injectWindowGlyphs();
  const askIco = document.getElementById("palette-ask-ico");
  if (askIco) askIco.innerHTML = mark("clarity");
  refresh();
  setInterval(loadSnapshot, 5000);

  document.querySelectorAll(".nav-btn").forEach((b) =>
    b.addEventListener("click", () => {
      const nav = b.dataset.nav;
      if (nav === "projects" || !S.route.view || S.route.view === "projects") {
        setRoute("#/projects");
        if (nav !== "projects") {
          S.route = { view: nav };
          document.getElementById("content").replaceChildren(projectRequiredNotice(cap(nav)));
        }
      } else {
        setRoute(`#/p/${S.route.projectId}/${nav}`);
      }
    }));

  document.getElementById("kill-switch").addEventListener("click", async () => {
    if (!hasTauri()) { toast("Available in the desktop app.", "error"); return; }
    if (!confirm("Stop all agent work?")) return;
    await invoke("emergency_stop");
    toast("Agent stopped.", "success");
    loadSnapshot();
  });
  document.getElementById("stop-agent").addEventListener("click", () =>
    document.getElementById("kill-switch").click());

  document.getElementById("search-open").addEventListener("click", openPalette);
  document.addEventListener("keydown", (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      S.paletteOpen ? closePalette() : openPalette();
    }
    if (S.paletteOpen) {
      if (e.key === "Escape") closePalette();
      const items = document.querySelectorAll(".palette-item");
      if (e.key === "ArrowDown") { e.preventDefault(); paletteSelection = Math.min(paletteSelection + 1, items.length - 1); markSelected(); }
      if (e.key === "ArrowUp") { e.preventDefault(); paletteSelection = Math.max(paletteSelection - 1, 0); markSelected(); }
      if (e.key === "Enter" && items[paletteSelection]) items[paletteSelection].click();
    }
  });
  document.getElementById("palette-input").addEventListener("input", (e) => renderPaletteResults(e.target.value));
  document.getElementById("palette").addEventListener("click", (e) => {
    if (e.target.id === "palette") closePalette();
  });
  document.getElementById("palette-ask-send").addEventListener("click", () => {
    toast("Model delegation arrives with a wired runtime. File and project actions work today.");
  });

  // Window controls (tauri-ui style borderless chrome). Inert in a plain
  // browser; fully functional in the packaged app.
  const currentWindow = () =>
    hasTauri() && window.__TAURI__.window ? window.__TAURI__.window.getCurrent() : null;
  const winClose = document.getElementById("win-close");
  const winMin = document.getElementById("win-min");
  const winMax = document.getElementById("win-max");
  if (winClose) winClose.addEventListener("click", () => currentWindow()?.close());
  if (winMin) winMin.addEventListener("click", () => currentWindow()?.minimize());
  if (winMax)
    winMax.addEventListener("click", () => {
      const win = currentWindow();
      // Double-click on the titlebar maximizes too; the green light toggles.
      win?.toggleMaximize();
    });
}
if (document.readyState === "loading") {
  window.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
