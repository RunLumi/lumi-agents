"use strict";
/* Lumi Agents desktop app (Folder-as-Project Work mode).
   All state is runtime-derived: empty means empty. No sample data. */

// ---------- IPC ----------
function hasTauri() {
  return typeof window.__TAURI__ !== "undefined" && window.__TAURI__.core;
}
async function invoke(cmd, args = {}) {
  if (!hasTauri()) {
    return { __unavailable: true };
  }
  return window.__TAURI__.core.invoke(cmd, args);
}

// ---------- state ----------
const S = {
  route: { view: "projects" },
  projects: [],
  project: null,      // ProjectOverview for the open project
  summary: null,      // ProjectSummary of the open project
  git: null,
  tasks: [],
  snapshot: null,
  file: null,         // { path, content, sha256 } currently open file
  tree: null,         // root listing cache: { [path]: ListedEntry[] }
  editing: false,
  paletteOpen: false,
};

// ---------- utils ----------
function esc(value) {
  return String(value == null ? "" : value)
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
function parseRoute() {
  const hash = location.hash.replace(/^#\/?/, "");
  const parts = hash.split("/").filter(Boolean);
  if (parts[0] === "p" && parts[1]) {
    S.route = { view: "project", projectId: decodeURIComponent(parts[1]), tab: parts[2] || "home", taskId: parts[3] ? decodeURIComponent(parts[3]) : null };
  } else {
    S.route = { view: parts[0] || "projects" };
  }
}
function statusPill(health) {
  if (health === "available") return '<span class="pill pill-green">● Available</span>';
  if (health === "missing") return '<span class="pill pill-gray">● Not Available</span>';
  return '<span class="pill pill-amber">● Moved — relink needed</span>';
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
const UNAVAILABLE = `<div class="empty-state"><span class="big">◌</span>
  Runtime unavailable. Open the Lumi desktop app to load real project state.</div>`;

// ---------- data loading ----------
async function loadProjects() {
  const result = await invoke("project_list_recent");
  if (result && result.__unavailable) { S.projects = null; return; }
  S.projects = result || [];
}
async function loadProject(projectId) {
  const overview = await invoke("project_overview", { projectId });
  if (overview && overview.__unavailable) { S.project = null; return; }
  S.project = overview;
  S.git = await invoke("git_status", { projectId });
  S.tasks = (await invoke("task_list", { projectId })) || [];
  S.changeSets = (await invoke("change_sets", { projectId })) || [];
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

// ---------- rendering ----------
function render() {
  parseRoute();
  const nav = S.route.view === "project" ? (S.route.tab === "task" ? "tasks" : S.route.tab) : S.route.view;
  document.querySelectorAll(".nav-btn").forEach((b) => {
    b.classList.toggle("active", b.dataset.nav === nav);
  });
  if (S.route.view === "projects") {
    setBreadcrumb([{ label: "Projects" }]);
    content.innerHTML = "";
    content.append(projectsHomeView());
  } else if (S.route.view === "project") {
    renderProject(content).catch((error) => {
      content.innerHTML = `<div class="empty-state unavailable-note"><b>View failed to render.</b><br>${esc(String(error))}</div>`;
    });
  } else {
    setBreadcrumb([{ label: cap(S.route.view) }]);
    content.innerHTML = "";
    content.append(projectRequiredNotice(cap(S.route.view)));
  }
  updateBadges();
}
function cap(text) { return text.charAt(0).toUpperCase() + text.slice(1); }

function setBreadcrumb(parts) {
  const el = document.getElementById("breadcrumb");
  el.innerHTML = parts
    .map((p, i) =>
      i === parts.length - 1
        ? `<span class="crumb current">${esc(p.label)}</span>`
        : `<span class="crumb" data-go="${esc(p.go || "#/projects")}">${esc(p.label)}</span><span class="crumb-sep">›</span>`
    )
    .join("");
  el.querySelectorAll("[data-go]").forEach((c) =>
    c.addEventListener("click", () => setRoute(c.dataset.go))
  );
}
function updateBadges() {
  const taskBadge = document.getElementById("nav-task-badge");
  const approvalBadge = document.getElementById("nav-approval-badge");
  const exceptions = S.snapshot && S.snapshot.exceptions ? S.snapshot.exceptions.length : 0;
  const pending = S.snapshot && S.snapshot.pending_approvals ? S.snapshot.pending_approvals.length : 0;
  const active = S.tasks ? S.tasks.filter((t) => !["COMPLETED", "FAILED", "CANCELLED"].includes(t.status)).length : 0;
  taskBadge.classList.toggle("hidden", !active);
  if (active) taskBadge.textContent = active;
  approvalBadge.classList.toggle("hidden", !pending && !exceptions);
  if (pending || exceptions) approvalBadge.textContent = pending + exceptions;
}

// ----- projects home -----
function projectsHomeView() {
  const wrap = document.createElement("div");
  wrap.innerHTML = `
    <section class="hero">
      <h1>Open a project, start building with Lumi</h1>
      <p>Lumi Agents works with your code directly, in the folder you choose.
         Open a folder, clone a repository, or pick up where you left off.</p>
    </section>
    <div class="action-cards">
      <button class="action-card" id="action-open-folder">
        <span class="action-ico">📁</span>
        <span class="action-title">Open Folder</span>
        <span class="action-desc">Work with an existing codebase on your machine.</span>
      </button>
      <button class="action-card" id="action-clone">
        <span class="action-ico">⑂</span>
        <span class="action-title">Clone Repository</span>
        <span class="action-desc">Start a new project by cloning a repository.</span>
      </button>
      <button class="action-card" id="action-recent">
        <span class="action-ico">🕐</span>
        <span class="action-title">Open Recent</span>
        <span class="action-desc">Quickly reopen a recent project and continue working.</span>
      </button>
    </div>
    <div class="project-layout">
      <div>
        <div class="section-head"><h2>Recent Projects</h2></div>
        <p class="section-sub">Your recently opened projects. Click to open and continue with Lumi.</p>
        <div id="recents"></div>
        <button class="dropzone" id="dropzone">
          <span>📂</span><span><b>Drag and drop a folder here</b> — or click to browse</span>
        </button>
      </div>
      <aside class="context-panel">
        <div class="card context-card">
          <h4>🛡 Project Access &amp; Security</h4>
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
          <h4>★ A more focused way to work</h4>
          Folder-as-project keeps things simple, secure, and productive.
        </div>
      </aside>
    </div>`;

  wrap.querySelector("#action-open-folder").addEventListener("click", pickAndOpenFolder);
  wrap.querySelector("#dropzone").addEventListener("click", pickAndOpenFolder);
  wrap.querySelector("#action-recent").addEventListener("click", () => {
    wrap.querySelector("#recents").scrollIntoView({ behavior: "smooth" });
  });
  wrap.querySelector("#action-clone").addEventListener("click", cloneRepositoryFlow);

  const recents = wrap.querySelector("#recents");
  if (S.projects === null) {
    recents.innerHTML = UNAVAILABLE;
  } else if (!S.projects.length) {
    recents.innerHTML = `<div class="empty-state"><span class="big">📁</span>
      No projects yet. Open a folder to create your first project.</div>`;
  } else {
    const grid = document.createElement("div");
    grid.className = "recent-grid";
    for (const project of S.projects) {
      grid.append(recentCard(project));
    }
    recents.append(grid);
  }
  return wrap;
}
function recentCard(project) {
  const card = document.createElement("button");
  card.className = "recent-card";
  card.innerHTML = `
    <div class="recent-top">
      <span class="recent-ico">🗂</span>
      <span style="flex:1">
        <div class="recent-name">${esc(project.display_name)}</div>
        <div class="recent-desc">${esc(project.detected_source)} project</div>
      </span>
      ${statusPill(project.health)}
    </div>
    <div class="recent-meta">
      <span class="chip">⑂ ${esc(project.primary_root.split(/[\\/]/).pop())}</span>
      <span class="chip">🕒 ${esc(timeAgo(project.last_opened_at))}</span>
      <span class="chip mono">${esc(project.primary_root)}</span>
    </div>`;
  card.addEventListener("click", () => {
    if (project.health !== "available") { openProjectMenu(project); return; }
    setRoute(`#/p/${encodeURIComponent(project.project_id)}/home`);
  });
  return card;
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
      source: source.trim(),
      destinationParent: parent,
      displayName: name || null,
    });
    toast("Repository cloned and opened as a project.", "success");
    refresh();
    setRoute(`#/p/${encodeURIComponent(opened.project.project_id)}/home`);
  } catch (error) {
    toast(String(error), "error");
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

// ----- project shell -----
async function renderProject(content) {
  const projectId = S.route.projectId;
  try {
    await loadProject(projectId);
  } catch (error) {
    content.innerHTML = `<div class="empty-state unavailable-note"><b>Project failed to load.</b><br>${esc(String(error))}</div>`;
    return;
  }
  if (!S.project) {
    content.innerHTML = UNAVAILABLE;
    return;
  }
  setBreadcrumb([
    { label: "Projects", go: "#/projects" },
    { label: S.project.display_name },
    ...(S.route.tab === "task" ? [{ label: "Tasks", go: `#/p/${projectId}/tasks` }, { label: "Task" }] : [{ label: cap(S.route.tab) }]),
  ]);
  content.innerHTML = "";
  content.append(projectHeaderView());
  content.append(tabsView());
  const layout = document.createElement("div");
  layout.className = "project-layout";
  const main = document.createElement("div");
  main.style.minWidth = "0";
  main.append(tabContent());
  layout.append(main, contextPanel());
  content.append(layout);
}
function projectHeaderView() {
  const head = document.createElement("div");
  head.className = "project-head";
  const branch = S.git ? S.git.branch : null;
  head.innerHTML = `
    <div class="project-ico">🗂</div>
    <div style="flex:1">
      <h1 class="project-title">${esc(S.project.display_name)}</h1>
      <p class="project-desc">${esc(S.project.detected_source)} project</p>
      <div class="project-chips">
        <span class="chip">📁 ${esc(S.project.primary_root)}</span>
        ${branch ? `<span class="chip">⑂ ${esc(branch)}</span>` : ""}
        <span class="pill pill-green">● Local • Safe</span>
      </div>
    </div>
    <div class="project-head-right">
      <button class="btn btn-sm" id="open-in-finder">📂 Open in Finder</button>
      <button class="btn btn-primary btn-sm" id="new-task-btn">＋ New Task</button>
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
  const defs = [
    ["home", "Home"], ["tasks", "Tasks"], ["files", "Files"], ["changes", "Changes"],
    ["git", "Git"], ["artifacts", "Artifacts"], ["evidence", "Evidence"],
    ["approvals", "Approvals"], ["settings", "Settings"],
  ];
  for (const [key, label] of defs) {
    const b = document.createElement("button");
    b.className = `tab ${S.route.tab === key ? "active" : ""}`;
    b.textContent = label;
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

// ----- home tab -----
function homeTabView() {
  const wrap = document.createElement("div");
  wrap.append(workingTreeCard(), validationCard(), recentChangesCard(), taskHistoryCard());
  wrap.querySelector(".stat-cards") || wrap.classList.add("card-grid");
  return wrap;
}
function workingTreeCard() {
  const card = document.createElement("div");
  card.className = "card";
  card.style.marginBottom = "14px";
  const git = S.git;
  const clean = git && git.is_clean;
  card.innerHTML = `
    <h3>⑂ Working Tree Status
      ${git ? (clean ? '<span class="pill pill-green">Clean</span>' : '<span class="pill pill-amber">Dirty</span>') : '<span class="pill pill-gray">No Git</span>'}
    </h3>
    ${git ? `
      <div class="kv"><span class="kv-key">Branch</span><span class="kv-val mono">${esc(git.branch || "detached")}</span></div>
      <div class="kv"><span class="kv-key">Staged files</span><span class="kv-val">${git.staged.length}</span></div>
      <div class="kv"><span class="kv-key">Modified files</span><span class="kv-val">${git.unstaged.length}</span></div>
      <div class="kv"><span class="kv-key">Untracked files</span><span class="kv-val">${git.untracked.length}</span></div>` :
      '<p class="card-sub">This project has no Git repository, so working-tree state is unavailable.</p>'}
    <button class="card-link" id="wt-link">View in Git →</button>`;
  card.querySelector("#wt-link").addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/git`));
  return card;
}
function validationCard() {
  const card = document.createElement("div");
  card.className = "card";
  card.style.marginBottom = "14px";
  const proposals = (S.project.discovery && S.project.discovery.proposed_commands) || [];
  const latest = latestValidations();
  const rows = latest.length
    ? latest.map((v) => `
        <div class="mini-row">
          <span class="${v.status === "passed" ? "check-yes" : "check-no"}">${v.status === "passed" ? "✓" : "✗"}</span>
          <span style="flex:1"><b>${esc(v.role)}</b> · <span class="mono">${esc(v.command)}</span></span>
          <span class="pill ${v.status === "passed" ? "pill-green" : v.status === "failed" ? "pill-red" : "pill-amber"}">${esc(v.status)}</span>
        </div>`).join("")
    : `<div class="empty-state">No validations recorded yet.</div>`;
  card.innerHTML = `
    <h3>🧪 Validation Status ${latest.length ? `<span class="pill ${allPassed(latest) ? "pill-green" : "pill-red"}">${allPassed(latest) ? "All passing" : "Attention needed"}</span>` : ""}</h3>
    <p class="card-sub">Run the project's real checks. "Files edited" is never "done".</p>
    ${rows}
    <div class="inline-form" style="margin-top:10px">
      <input class="input" id="val-command" list="val-proposals" placeholder="${esc(proposals[0] ? proposals[0].command : "cargo test / npm test …")}">
      <datalist id="val-proposals">${proposals.map((p) => `<option value="${esc(p.command)}">${esc(p.role)}</option>`).join("")}</datalist>
      <button class="btn btn-primary btn-sm" id="val-run">Run</button>
    </div>`;
  card.querySelector("#val-run").addEventListener("click", async () => {
    const command = card.querySelector("#val-command").value.trim()
      || (proposals[0] ? proposals[0].command : "");
    if (!command) { toast("Nothing to run.", "error"); return; }
    const task = await ensureManualTask();
    try {
      const record = await invoke("validation_run", { projectId: S.route.projectId, taskId: task, command });
      toast(`Validation ${record.status}.`, record.status === "passed" ? "success" : "error");
      refresh();
    } catch (e) { toast(String(e), "error"); }
  });
  return card;
}
function latestValidations() {
  // Validations from the manual + project task sets, newest task set first.
  if (!S.changeSets) return [];
  return S.changeSets.flatMap((set) => set.validations || []);
}
function allPassed(validations) {
  return validations.length && validations.every((v) => v.status === "passed");
}
async function ensureManualTask() {
  // Validations not tied to a runtime task are recorded on the reserved
  // manual task set so history stays attributable.
  return "task-manual-local";
}
function recentChangesCard() {
  const card = document.createElement("div");
  card.className = "card";
  const sets = S.changeSets || [];
  const entries = sets.flatMap((set) => set.entries || []).slice(0, 6);
  card.innerHTML = `
    <h3>📝 Recent Lumi Changes <button class="card-link" id="see-changes" style="margin-left:auto">View in Changes →</button></h3>
    <div class="mini-list">
      ${entries.length ? entries.map((e) => `
        <div class="mini-row">
          <span>📄</span>
          <span style="flex:1" class="mono">${esc(e.path)}</span>
          <span class="pill pill-blue">${esc(e.kind)}${e.source === "external_conflict" ? " · external" : ""}</span>
          <span class="muted small">${esc(timeAgo(e.recorded_at))}</span>
        </div>`).join("") : `<div class="empty-state">Lumi has not changed anything in this project yet.</div>`}
    </div>`;
  card.querySelector("#see-changes").addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/changes`));
  return card;
}
function taskHistoryCard() {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `
    <h3>🧭 Task History <button class="card-link" id="see-tasks" style="margin-left:auto">View All →</button></h3>
    ${S.tasks && S.tasks.length ? `<div class="mini-list">
      ${S.tasks.slice(0, 6).map((t) => `
        <div class="task-row" data-task="${esc(t.task_id)}">
          <span class="task-row-goal" style="flex:1">${esc(t.goal)}</span>
          ${taskStatusPill(t.status)}
          <span class="muted small">${esc(timeAgo(t.created_at))}</span>
        </div>`).join("")}
    </div>` : `<div class="empty-state">No tasks yet. Create one to delegate real work.</div>`}`;
  card.querySelector("#see-tasks").addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/tasks`));
  card.querySelectorAll("[data-task]").forEach((row) =>
    row.addEventListener("click", () => setRoute(`#/p/${S.route.projectId}/task/${row.dataset.task}`)));
  return card;
}

// ----- tasks -----
function tasksView() {
  const wrap = document.createElement("div");
  wrap.className = "card";
  wrap.innerHTML = `
    <h3>New Task</h3>
    <p class="card-sub">Give Lumi a goal for this project. The task is durable and resumes after restart.</p>
    <div class="field"><textarea class="textarea" id="task-goal" placeholder="e.g. Implement OAuth login and make all tests pass"></textarea></div>
    <button class="btn btn-primary" id="task-create">Create Task</button>
    <h3 style="margin-top:18px">Tasks</h3>
    <div id="task-rows"></div>`;
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
    rows.innerHTML = `<div class="empty-state">No tasks yet.</div>`;
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
  let changeSet;
  try {
    changeSet = await invoke("change_set", { projectId: S.route.projectId, taskId });
    if (changeSet && changeSet.__unavailable) changeSet = null;
  } catch { changeSet = null; }
  const wrap = document.createElement("div");
  const steps = task
    ? [["Create", task.status !== "CREATED"], ["Work", ["RUNNING", "COMPLETED", "WAITING_APPROVAL", "WAITING_USER", "PAUSED"].includes(task.status)], ["Validate", changeSet && changeSet.validations && changeSet.validations.length], ["Review", task.status === "COMPLETED"]]
    : [];
  wrap.className = "card";
  wrap.innerHTML = `
    ${!task ? '<div class="empty-state">Task not found in this project\'s durable state.</div>' : `
      <div class="project-head" style="margin-bottom:8px">
        <div style="flex:1">
          <h1 class="project-title" style="font-size:18px">${esc(task.goal)}</h1>
          <div class="project-chips" style="margin-top:8px">
            ${taskStatusPill(task.status)}
            <span class="chip">${esc((task.project_binding && task.project_binding.workspace_kind) || "")}</span>
            <span class="chip mono">${esc((task.project_binding && task.project_binding.workspace_root) || "")}</span>
          </div>
        </div>
      </div>
      <div class="stepper">
        ${steps.map(([label, done]) => `
          <div class="step ${done ? "done" : ""}">
            <div class="step-dot">${done ? "✓" : ""}</div>
            <div class="step-label">${esc(label)}</div>
          </div>`).join("")}
      </div>
      <h3 style="margin:16px 0 6px">Validations</h3>
      ${changeSet && changeSet.validations && changeSet.validations.length ? changeSet.validations.map((v) => `
        <div class="mini-row">
          <span class="${v.status === "passed" ? "check-yes" : "check-no"}">${v.status === "passed" ? "✓" : "✗"}</span>
          <span style="flex:1" class="mono">${esc(v.command)}</span>
          <span class="pill ${v.status === "passed" ? "pill-green" : v.status === "failed" ? "pill-red" : "pill-amber"}">${esc(v.status)}</span>
        </div>`).join("") : '<div class="empty-state">No validations run for this task yet.</div>'}
    `}`;
  return wrap;
}

// ----- files -----
async function filesView() {
  const wrap = document.createElement("div");
  wrap.className = "files-layout";
  const treePanel = document.createElement("div");
  treePanel.className = "card tree-panel";
  treePanel.innerHTML = `<h3>Files</h3><div id="tree"></div>`;
  const viewer = document.createElement("div");
  viewer.className = "file-viewer";
  const context = document.createElement("div");
  context.className = "card context-card";
  context.innerHTML = `<h4>📄 File Context</h4><div class="empty-state">Select a file.</div>`;
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
    item.innerHTML = `<span>${entry.is_dir ? "📁" : "📄"}</span><span style="flex:1;overflow:hidden;text-overflow:ellipsis">${esc(entry.name)}</span>`;
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
async function openFile(projectId, path, viewer, context) {
  let file;
  try {
    file = await invoke("file_read", { projectId, path });
    if (file && file.__unavailable) return;
  } catch (error) {
    viewer.innerHTML = `<div class="empty-state" style="margin:14px">${esc(String(error))}</div>`;
    return;
  }
  S.file = file;
  S.editing = false;
  const lines = file.content.split("\n");
  viewer.innerHTML = `
    <div class="file-viewer-head">
      <span>📄</span><b>${esc(path)}</b>
      <span style="flex:1"></span>
      <button class="btn btn-sm" id="file-edit-toggle">Edit</button>
      <button class="btn btn-primary btn-sm" id="file-save" disabled>Save</button>
    </div>
    <div class="code-body">
      <div class="code-lines">${lines.map((_, i) => `${i + 1}`).join("<br>")}</div>
      <div class="code-content" id="code-content">${esc(file.content)}</div>
    </div>`;
  context.innerHTML = `
    <h4>📄 File Context</h4>
    <div class="kv"><span class="kv-key">Path</span><span class="kv-val mono">${esc(path)}</span></div>
    <div class="kv"><span class="kv-key">Lines</span><span class="kv-val">${lines.length}</span></div>
    <div class="kv"><span class="kv-key">Size</span><span class="kv-val">${new Blob([file.content]).size} B</span></div>
    <div class="kv"><span class="kv-key">SHA-256</span><span class="kv-val mono small">${esc(file.sha256.slice(0, 16))}…</span></div>
    <p class="card-sub" style="margin-top:10px">Edits are checksum-guarded: if the file changes on disk before you save, Lumi refuses and preserves the newer version.</p>`;
  viewer.querySelector("#file-edit-toggle").addEventListener("click", () => {
    S.editing = !S.editing;
    const body = viewer.querySelector(".code-body");
    if (S.editing) {
      const editor = document.createElement("textarea");
      editor.className = "editor-area";
      editor.value = file.content;
      editor.id = "editor";
      body.replaceWith(editor);
      viewer.querySelector("#file-edit-toggle").textContent = "Cancel";
      viewer.querySelector("#file-save").disabled = false;
    } else {
      refresh();
    }
  });
  viewer.querySelector("#file-save").addEventListener("click", async () => {
    const editor = viewer.querySelector("#editor");
    try {
      await invoke("file_edit", { projectId, path, expectedSha256: file.sha256, content: editor.value });
      toast("Saved.", "success");
      await openFile(projectId, path, viewer, context);
    } catch (error) {
      toast(String(error), "error");
    }
  });
}

// ----- changes -----
async function changesView() {
  const sets = await invoke("change_sets", { projectId: S.route.projectId });
  const wrap = document.createElement("div");
  wrap.innerHTML = `
    <div class="card">
      <h3>⇄ Lumi Change Sets</h3>
      <p class="card-sub">Exactly what Lumi changed — never raw tool-call telemetry. Pre-existing edits in your working tree are shown in Git, not here.</p>
      <div id="sets"></div>
    </div>`;
  const container = wrap.querySelector("#sets");
  if (!sets || sets.__unavailable) { container.innerHTML = UNAVAILABLE; return wrap; }
  if (!sets.length) {
    container.innerHTML = `<div class="empty-state"><span class="big">✎</span>No changes yet. Changes appear here as tasks and edits modify project files.</div>`;
    return wrap;
  }
  for (const set of sets) {
    const card = document.createElement("div");
    card.className = "card";
    card.style.marginBottom = "12px";
    const validations = set.validations || [];
    card.innerHTML = `
      <h3>Task <span class="mono">${esc(set.task_id)}</span>
        ${allPassed(validations) ? '<span class="pill pill-green">Validated</span>' : validations.length ? '<span class="pill pill-red">Validation failed</span>' : ""}
      </h3>
      <p class="card-sub">${(set.entries || []).length} file change(s) · ${validations.length} validation(s) · updated ${esc(timeAgo(set.updated_at))}</p>
      ${(set.entries || []).map((e) => renderChangeEntry(e)).join("")}
      ${validations.map((v) => `
        <div class="mini-row">
          <span class="${v.status === "passed" ? "check-yes" : "check-no"}">${v.status === "passed" ? "✓" : "✗"}</span>
          <span style="flex:1" class="mono">${esc(v.command)}</span>
          <span class="pill ${v.status === "passed" ? "pill-green" : v.status === "failed" ? "pill-red" : "pill-amber"}">${esc(v.status)}</span>
        </div>`).join("")}
    `;
    container.append(card);
  }
  return wrap;
}
function renderChangeEntry(entry) {
  let html = `
    <div class="mini-row">
      <span>📄</span>
      <span style="flex:1" class="mono">${esc(entry.path)}</span>
      <span class="pill pill-blue">${esc(entry.kind)}</span>
      ${entry.sha256_before && entry.sha256_after && entry.sha256_before !== entry.sha256_after
        ? `<button class="card-link" data-patch="${esc(entry.path)}">View diff</button>` : ""}
    </div>
    <div id="patch-${esc(entry.path).replace(/[^a-z0-9]/gi, "")}"></div>`;
  return html;
}

// ----- git -----
async function gitView() {
  const wrap = document.createElement("div");
  wrap.innerHTML = `
    <h2 style="margin:0 0 4px">Git Workspace</h2>
    <p class="section-sub">Manage branches, review changes, and collaborate with confidence.</p>
    <div id="git-body"></div>
    <div class="project-layout" style="margin-top:14px">
      <div class="card">
        <h3>🔒 Git Policy &amp; Safety</h3>
        <p class="card-sub"><b>Auto-approved (safe):</b> view history, create branches, read diffs, switch branches.</p>
        <p class="card-sub" style="margin:0"><b>Approval required:</b> push to remote, force push, history rewrite, remote branch deletion. These never run without explicit authority — they are not exposed to the local runtime at all.</p>
      </div>
    </div>`;
  const body = wrap.querySelector("#git-body");
  if (!S.git) {
    body.innerHTML = `<div class="empty-state"><span class="big">⑂</span>This project has no Git repository, so Git tools are unavailable.</div>`;
    return wrap;
  }
  const git = S.git;
  const log = await invoke("git_log", { projectId: S.route.projectId, limit: 8 });
  const branches = await invoke("git_branches", { projectId: S.route.projectId });
  body.innerHTML = `
    <div class="stat-cards">
      <div class="card">
        <h3>⑂ Current Branch ${git.is_clean ? '<span class="pill pill-green">Clean</span>' : '<span class="pill pill-amber">Dirty</span>'}</h3>
        <p style="font-size:16px;font-weight:700;margin:6px 0" class="mono">${esc(git.branch || "detached HEAD")}</p>
        <div class="kv"><span class="kv-key">Staged</span><span class="kv-val">${git.staged.length}</span></div>
        <div class="kv"><span class="kv-key">Unstaged</span><span class="kv-val">${git.unstaged.length}</span></div>
        <div class="kv"><span class="kv-key">Untracked</span><span class="kv-val">${git.untracked.length}</span></div>
      </div>
      <div class="card" style="grid-column: span 2">
        <h3>Changes in Working Directory</h3>
        ${git.staged.length + git.unstaged.length + git.untracked.length ? `
          <table class="status-table">
            <tr><th></th><th>File</th><th>Status</th></tr>
            ${git.staged.map((f) => gitRow(f.path, f.index_state, "staged")).join("")}
            ${git.unstaged.map((f) => gitRow(f.path, f.worktree_state, "unstaged")).join("")}
            ${git.untracked.map((p) => gitRow(p, "?", "untracked")).join("")}
          </table>` : '<div class="empty-state">Working tree is clean.</div>'}
        <div class="inline-form" style="margin-top:12px">
          <input class="input" id="commit-msg" placeholder="Commit message (commits exactly the files you ticked)">
          <button class="btn btn-primary btn-sm" id="commit-btn">Commit Selected</button>
        </div>
        <p class="card-sub" style="margin-top:6px">Commits are path-scoped: Lumi never sweeps in your other staged work.</p>
      </div>
    </div>
    <div class="two-cards">
      <div class="card">
        <h3>Branches</h3>
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
        <h3>Recent Commits</h3>
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

// ----- artifacts / evidence / approvals (honest states) -----
function artifactsView() {
  const wrap = document.createElement("div");
  wrap.className = "card";
  wrap.innerHTML = `
    <h3>◇ Artifacts</h3>
    <p class="card-sub">Generated outputs, reports, and exports — checksummed and provenance-linked.</p>
    <div class="empty-state"><span class="big">◇</span>No artifacts yet. Run a task that produces reports or exports and they will appear here with SHA-256 integrity refs.</div>`;
  return wrap;
}
function evidenceView() {
  const wrap = document.createElement("div");
  wrap.className = "card";
  const evidence = S.snapshot && S.snapshot.evidence;
  wrap.innerHTML = `
    <h3>≡ Evidence</h3>
    <p class="card-sub">Trust-labeled action history. "Attempted" is not "done".</p>
    ${evidence && evidence.length ? `<div class="mini-list">
      ${evidence.map((e) => `
        <div class="mini-row">
          <span style="flex:1">${esc(e.operation || e.action_id || "action")}</span>
          <span class="pill ${e.trust_label === "VERIFIED" ? "pill-green" : "pill-amber"}">${esc(e.trust_label || "unknown")}</span>
        </div>`).join("")}
    </div>` : `<div class="empty-state">No runtime evidence recorded yet.</div>`}`;
  return wrap;
}
function approvalsView() {
  const wrap = document.createElement("div");
  const exceptions = S.snapshot && S.snapshot.exceptions;
  const pending = S.snapshot && S.snapshot.pending_approvals;
  wrap.innerHTML = `
    <div class="card">
      <h3>✓ Approvals &amp; Exceptions</h3>
      <p class="card-sub">Consequential actions pause here for your explicit decision.</p>
      ${pending && pending.length ? pending.map((p) => `
        <div class="card" style="margin-bottom:10px">
          <b>${esc(p.card ? p.card.business_effect : p.action_digest)}</b>
          <div class="kv"><span class="kv-key">Digest</span><span class="kv-val mono small">${esc(p.action_digest)}</span></div>
        </div>`).join("") : `<div class="empty-state"><span class="big">✓</span>Nothing needs your approval right now.</div>`}
      ${exceptions && exceptions.length ? exceptions.map((x) => `
        <div class="card" style="margin-top:10px">
          <h3><span class="pill pill-red">Exception</span> ${esc(x.what_blocked || "")}</h3>
          <p class="card-sub">${esc(x.why || "")}</p>
          ${(x.safe_choices || []).map((c) => `<div class="mini-row"><b>${esc(c.label)}</b><span class="muted">${esc(c.description)}</span></div>`).join("")}
        </div>`).join("") : ""}
    </div>`;
  return wrap;
}

// ----- settings -----
function settingsView() {
  const wrap = document.createElement("div");
  const p = S.project;
  wrap.innerHTML = `
    <div class="project-layout">
      <div>
        <div class="card" style="margin-bottom:14px">
          <h3>⚙ Project Settings</h3>
          <p class="card-sub">Real configuration for this project. Org policy and provider routing are governed by policy — a project may only narrow, never widen, authority.</p>
          <div class="kv"><span class="kv-key">Project ID</span><span class="kv-val mono">${esc(p.project_id)}</span></div>
          <div class="kv"><span class="kv-key">Project root</span><span class="kv-val mono">${esc(p.primary_root)}</span></div>
          <div class="kv"><span class="kv-key">Environment</span><span class="kv-val mono">${esc(p.environment_id)}</span></div>
          <div class="kv"><span class="kv-key">Detected source</span><span class="kv-val">${esc(p.detected_source)}</span></div>
          <div class="kv"><span class="kv-key">Instruction sources</span><span class="kv-val">${p.instructions.length ? esc(p.instructions.join(", ")) : "none found"}</span></div>
        </div>
        <div class="card">
          <h3>🔐 Capabilities (negotiated)</h3>
          <p class="card-sub">Only capabilities the runtime actually provides are advertised.</p>
          <div class="recent-meta">
            ${(p.capabilities || []).map((c) => `<span class="chip">${esc(c)}</span>`).join("") || '<span class="muted">none</span>'}
          </div>
          <div class="inline-form" style="margin-top:14px">
            <button class="btn" id="btn-relink">Relink root…</button>
            <button class="btn btn-danger-outline" id="btn-remove">Remove project</button>
          </div>
        </div>
        <div class="card" id="memory-panel">
          <h3>💡 Project Memory</h3>
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
          <h4>🛡 Security &amp; Trust Posture <span class="pill pill-green">Strict</span></h4>
          <ul class="check-list">
            <li><span class="check-yes">✓</span> Strict project boundary — only the authorized roots</li>
            <li><span class="check-yes">✓</span> Local execution — files stay on your machine</li>
            <li><span class="check-yes">✓</span> Identity proven by a project marker on disk</li>
            <li><span class="check-no">○</span> Push / force-push / cleanup need external authority (not exposed)</li>
          </ul>
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

function contextPanel() {
  const panel = document.createElement("aside");
  panel.className = "context-panel";
  const p = S.project;
  const git = S.git;
  panel.innerHTML = `
    <div class="card context-card">
      <h4>🗂 Project Context</h4>
      <div class="kv"><span class="kv-key">Project Root</span></div>
      <div class="chip mono" style="width:100%;justify-content:space-between">${esc(p.primary_root)}</div>
      <div class="kv" style="margin-top:8px"><span class="kv-key">Environment</span></div>
      <div class="kv"><span class="kv-val mono">${esc(p.environment_id)}</span></div>
      <div class="kv"><span class="kv-key">Branch</span><span class="kv-val mono">${esc((git && git.branch) || "—")}</span></div>
      <div class="kv"><span class="kv-key">Permissions Boundary</span>
        <span class="pill pill-green">Strict (Project Only)</span></div>
      <p class="card-sub" style="margin:8px 0 0">Lumi can only access files within this project's authorized roots. No access to your home directory or other projects.</p>
    </div>
    <div class="card context-card">
      <h4>✓ What Lumi can access</h4>
      <ul class="check-list">
        <li><span class="check-yes">✓</span> Read and edit files in this project</li>
        <li><span class="check-yes">✓</span> Run project commands (via bounded shell)</li>
        <li><span class="check-yes">✓</span> Read git history and create local branches</li>
        <li><span class="check-no">✕</span> No access to personal files or external networks</li>
      </ul>
    </div>
    <div class="note-card">
      <h4>💡 Working in a safe, local environment</h4>
      Your code stays on your machine. Capabilities shown are exactly what the runtime provides — nothing more.
    </div>`;
  return panel;
}

function projectRequiredNotice(label) {
  const el = document.createElement("div");
  if (S.projects && S.projects.length) {
    el.innerHTML = `<div class="empty-state"><span class="big">🗂</span>Open a project first to use ${esc(label)}.<div style="margin-top:10px">
      <button class="btn btn-primary" id="go-projects">Go to Projects</button></div></div>`;
    el.querySelector("#go-projects").addEventListener("click", () => setRoute("#/projects"));
  } else {
    el.innerHTML = UNAVAILABLE;
  }
  return el;
}

// ----- command palette -----
let paletteProjects = [];
async function openPalette() {
  S.paletteOpen = true;
  document.getElementById("palette").classList.remove("hidden");
  const input = document.getElementById("palette-input");
  input.value = "";
  input.focus();
  paletteProjects = S.projects || [];
  await renderPaletteResults("");
}
function closePalette() {
  S.paletteOpen = false;
  document.getElementById("palette").classList.add("hidden");
}
async function renderPaletteResults(query) {
  const results = document.getElementById("palette-results");
  const chips = document.getElementById("palette-chips");
  const inProject = S.route.view === "project";
  chips.innerHTML = ["review Lumi changes", "open approval queue", "show failing tests"]
    .map((c) => `<button class="palette-chip">${esc(c)}</button>`).join("");
  let html = "";
  const q = query.trim().toLowerCase();
  const projects = paletteProjects.filter((p) => !q || p.display_name.toLowerCase().includes(q));
  if (projects.length) {
    html += '<div class="palette-group">Projects</div>';
    html += projects.slice(0, 5).map((p) => `
      <button class="palette-item" data-go="#/p/${encodeURIComponent(p.project_id)}/home">
        <span>🗂</span><span style="flex:1">${esc(p.display_name)}</span><span class="mono">${esc(p.health)}</span>
      </button>`).join("");
  }
  if (inProject && q) {
    const files = await invoke("file_search", { projectId: S.route.projectId, query: q, mode: "filename" });
    if (files && files.length) {
      html += '<div class="palette-group">Files</div>';
      html += files.slice(0, 8).map((f) => `
        <button class="palette-item" data-file="${esc(f.path)}">
          <span>📄</span><span style="flex:1" class="mono">${esc(f.path)}</span>
        </button>`).join("");
    }
  }
  results.innerHTML = html || '<div class="empty-state" style="margin:10px">No matches.</div>';
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
}

// ---------- refresh + events ----------
let refreshSeq = 0;
async function refresh() {
  const seq = ++refreshSeq;
  await loadSnapshot();
  await loadProjects();
  if (seq !== refreshSeq) return; // a newer refresh supersedes this one
  await render();
}
window.addEventListener("hashchange", refresh);
function boot() {
  refresh();
  setInterval(loadSnapshot, 5000);

  document.querySelectorAll(".nav-btn").forEach((b) =>
    b.addEventListener("click", () => {
      const nav = b.dataset.nav;
      if (nav === "projects" || !S.route.view || S.route.view === "projects") {
        setRoute("#/projects");
        if (nav !== "projects") render();
        S.route = { view: nav };
        document.getElementById("content").innerHTML = "";
        document.getElementById("content").append(projectRequiredNotice(cap(nav)));
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
    if (e.key === "Escape") closePalette();
  });
  document.getElementById("palette-input").addEventListener("input", (e) => renderPaletteResults(e.target.value));
  document.getElementById("palette").addEventListener("click", (e) => {
    if (e.target.id === "palette") closePalette();
  });
  document.getElementById("palette-ask-send").addEventListener("click", () => {
    toast("Model delegation arrives with a wired runtime. File and project actions work today.");
  });
}
if (document.readyState === "loading") {
  window.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
