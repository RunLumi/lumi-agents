// Lumi desktop frontend: calls Tauri IPC commands to the backend.
// The UI is a VIEWPORT onto the orchestrator's durable state — it does
// not own policy, credentials, or executor state (spec 23 §23.17).

// Tauri v2 injects window.__TAURI__ in the webview.
// In a browser (for dev), we fall back to mock data.

const isTauri = typeof window.__TAURI__ !== 'undefined';

async function invoke(cmd, args) {
  if (isTauri) {
    return window.__TAURI__.core.invoke(cmd, args);
  }
  // Mock responses for browser-only development.
  return mockInvoke(cmd, args);
}

function mockInvoke(cmd) {
  const mocks = {
    get_kill_switch: { running: false },
    emergency_stop: { running: false },
    get_connection_status: true,
    get_task_progress: {
      task_id: 'task-demo',
      goal: 'Reconcile Q3 invoices',
      phase: 'EXECUTING',
      steps_completed: 3,
      steps_total: 5,
      pending_approvals: 1,
      exceptions: 0,
      elapsed_minutes: 3,
      budget_used: 4,
      budget_total: 20,
    },
    get_pending_approvals: [
      {
        action_digest: 'abc123def456',
        business_effect: 'Send quote to customer@example.com for 12,500,000 VND',
        target_description: 'Email via CRM connector',
        reversible: false,
        policy_reason: 'COMMUNICATION requires approval',
        expires_in: '60 minutes',
      },
    ],
    get_exceptions: [],
    get_evidence: [
      { action_id: 'a-1', operation: 'fetch_open_invoices', trust_label: 'Verified', verified: true },
      { action_id: 'a-2', operation: 'send_customer_email', trust_label: 'Attempted', verified: false },
    ],
    get_permissions: [
      { capability: 'accessibility', description: 'Control applications via Accessibility', state: 'GRANTED' },
      { capability: 'screen_recording', description: 'Selective screenshot evidence', state: 'NOT_GRANTED' },
    ],
  };
  return Promise.resolve(mocks[cmd] ?? null);
}

// ---------------------------------------------------------------------------
// View management
// ---------------------------------------------------------------------------

function showView(name) {
  document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
  document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
  document.getElementById(`view-${name}`).classList.add('active');
  document.querySelector(`.nav-btn[data-view="${name}"]`).classList.add('active');
  refresh();
}

// ---------------------------------------------------------------------------
// Data loading
// ---------------------------------------------------------------------------

async function refresh() {
  const active = document.querySelector('.nav-btn.active')?.dataset.view ?? 'dashboard';
  switch (active) {
    case 'dashboard': await loadDashboard(); break;
    case 'approvals': await loadApprovals(); break;
    case 'exceptions': await loadExceptions(); break;
    case 'evidence': await loadEvidence(); break;
    case 'permissions': await loadPermissions(); break;
  }
}

async function loadDashboard() {
  const progress = await invoke('get_task_progress');
  const kill = await invoke('get_kill_switch');
  const container = document.getElementById('task-cards');

  const pct = progress.steps_total > 0
    ? Math.round((progress.steps_completed / progress.steps_total) * 100)
    : 0;

  container.innerHTML = `
    <div class="task-card">
      <h3>${esc(progress.goal)}</h3>
      <div class="task-meta">
        <span>Phase: ${esc(progress.phase)}</span>
        <span>Steps: ${progress.steps_completed}/${progress.steps_total}</span>
        <span>Time: ${progress.elapsed_minutes} min</span>
        <span>Budget: ${progress.budget_used}/${progress.budget_total}</span>
      </div>
      <div class="progress-bar"><div class="fill" style="width:${pct}%"></div></div>
    </div>
  `;

  // Kill switch reflects the live state.
  const btn = document.getElementById('kill-switch');
  btn.textContent = kill.running ? '⏹ STOP ALL' : '◼ STOPPED';
  btn.disabled = !kill.running;
  btn.style.background = kill.running ? 'var(--danger)' : 'var(--border)';

  // Update badges.
  updateBadge('approval-badge', progress.pending_approvals);
  updateBadge('exception-badge', progress.exceptions);
}

async function loadApprovals() {
  const approvals = await invoke('get_pending_approvals');
  const container = document.getElementById('approval-cards');
  container.innerHTML = approvals.map(a => `
    <div class="card">
      <h3>${esc(a.business_effect)}</h3>
      <div class="detail">Target: ${esc(a.target_description)}</div>
      <div class="detail risk">${esc(a.policy_reason)}</div>
      <div class="detail">Reversible: ${a.reversible ? 'Yes' : 'No'}</div>
      <div class="detail">Expires in: ${esc(a.expires_in)}</div>
      <div class="card-actions">
        <button class="btn btn-approve" onclick="approveAction('${esc(a.action_digest)}')">Approve</button>
        <button class="btn btn-reject" onclick="rejectAction('${esc(a.action_digest)}')">Reject</button>
      </div>
    </div>
  `).join('');
  updateBadge('approval-badge', approvals.length);
}

async function loadExceptions() {
  const exceptions = await invoke('get_exceptions');
  const container = document.getElementById('exception-cards');
  container.innerHTML = exceptions.map(e => `
    <div class="card">
      <h3>${esc(e.what_blocked)}</h3>
      <div class="detail">${esc(e.why)}</div>
      <div class="detail risk">${esc(e.consequences)}</div>
      <div class="detail">Next step: ${esc(e.suggested_next_step)}</div>
      <div class="card-actions">
        ${e.safe_choices.map(c =>
          `<button class="btn btn-secondary" title="${esc(c.description)}">${esc(c.label)}</button>`
        ).join('')}
      </div>
    </div>
  `).join('');
  updateBadge('exception-badge', exceptions.length);
}

async function loadEvidence() {
  const evidence = await invoke('get_evidence');
  const container = document.getElementById('evidence-list');
  container.innerHTML = evidence.map(e => `
    <div class="evidence-item">
      <div>
        <div class="operation">${esc(e.operation)}</div>
        <div class="action-id">${esc(e.action_id)}</div>
      </div>
      <span class="trust-label trust-${e.trust_label.toLowerCase()}">${esc(e.trust_label)}</span>
    </div>
  `).join('');
}

async function loadPermissions() {
  const permissions = await invoke('get_permissions');
  const container = document.getElementById('permission-list');
  container.innerHTML = permissions.map(p => `
    <div class="permission-item">
      <div>
        <div class="name">${esc(p.capability)}</div>
        <div class="description">${esc(p.description)}</div>
      </div>
      <span class="permission-state ${p.state === 'GRANTED' ? 'permission-granted' : 'permission-not-granted'}">
        ${esc(p.state)}
      </span>
    </div>
  `).join('');
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

async function approveAction(digest) {
  await invoke('approve_action', { digest });
  await refresh();
}

async function rejectAction(digest) {
  await invoke('reject_action', { digest });
  await refresh();
}

async function emergencyStop() {
  await invoke('emergency_stop');
  await refresh();
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

function esc(text) {
  const div = document.createElement('div');
  div.textContent = text ?? '';
  return div.innerHTML;
}

function updateBadge(id, count) {
  const el = document.getElementById(id);
  if (!el) return;
  if (count > 0) {
    el.textContent = count;
    el.classList.remove('hidden');
  } else {
    el.classList.add('hidden');
  }
}

// Auto-refresh every 5 seconds.
setInterval(refresh, 5000);

// Initial load.
document.addEventListener('DOMContentLoaded', refresh);
