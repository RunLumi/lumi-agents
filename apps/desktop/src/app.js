// Lumi desktop frontend: a presentation layer over runtime-owned state.
//
// The browser fallback is deliberately empty. It exists for layout work and
// never pretends that fixture data is a live task, approval, permission, or
// verified outcome.

const isTauri = typeof window.__TAURI__ !== 'undefined';

function unknownSnapshot(connection = 'unknown') {
  return {
    connection,
    execution: 'unavailable',
    queue: null,
    progress: null,
    pending_approvals: null,
    exceptions: null,
    evidence: null,
    economics: null,
    permissions: null,
  };
}

async function invoke(cmd, args) {
  if (isTauri) {
    return window.__TAURI__.core.invoke(cmd, args);
  }
  return mockInvoke(cmd, args);
}

function mockInvoke(cmd) {
  switch (cmd) {
    case 'get_operations_snapshot':
      return Promise.resolve(unknownSnapshot());
    case 'get_kill_switch':
    case 'emergency_stop':
      return Promise.resolve({ state: 'unknown', running: null });
    default:
      return Promise.resolve(null);
  }
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

async function loadSnapshot() {
  try {
    const snapshot = await invoke('get_operations_snapshot');
    return snapshot ?? unknownSnapshot();
  } catch (_error) {
    return unknownSnapshot('disconnected');
  }
}

async function refresh() {
  const snapshot = await loadSnapshot();
  updateConnection(snapshot.connection, snapshot.execution);
  const active = document.querySelector('.nav-btn.active')?.dataset.view ?? 'dashboard';
  switch (active) {
    case 'dashboard': await loadDashboard(snapshot); break;
    case 'approvals': await loadApprovals(snapshot); break;
    case 'exceptions': await loadExceptions(snapshot); break;
    case 'evidence': await loadEvidence(snapshot); break;
    case 'permissions': await loadPermissions(snapshot); break;
  }
}

async function loadDashboard(snapshot) {
  const kill = await invoke('get_kill_switch').catch(() => ({ state: 'unknown', running: null }));
  const container = document.getElementById('task-cards');
  const progress = snapshot?.progress;

  if (!progress) {
    const queue = Array.isArray(snapshot?.queue)
      ? `Queue items: ${snapshot.queue.length}`
      : 'Queue state unavailable.';
    container.innerHTML = `
      <div class="empty-state unavailable">Workload state is unavailable until a Lumi runtime is connected.</div>
      <div class="detail">${esc(queue)}</div>
    `;
  } else {
    const steps = Array.isArray(progress.steps) ? progress.steps : [];
    const verified = steps.filter(step => step.trust === 'VERIFIED').length;
    const activeSystem = progress.active_system
      ? `<span>System: ${esc(progress.active_system)}</span>`
      : '';
    const economics = renderEconomics(snapshot.economics);
    const queueSummary = Array.isArray(snapshot.queue)
      ? `<div class="detail">Queue items: ${snapshot.queue.length}</div>`
      : '<div class="detail unavailable">Queue state unavailable.</div>';
    container.innerHTML = `
      <div class="task-card">
        <h3>${esc(progress.goal)}</h3>
        <div class="task-meta">
          <span>Task: ${esc(progress.task_id)}</span>
          <span>Phase: ${esc(progress.phase)}</span>
          <span>Verified evidence records: ${verified}</span>
          <span>Time: ${progress.elapsed_minutes} min</span>
          <span>Budget: ${progress.budget_used}/${progress.budget_total}</span>
          ${activeSystem}
        </div>
        <div class="detail">${esc(progress.evidence_summary || 'Evidence summary unavailable')}</div>
        ${queueSummary}
      </div>
      ${economics}
    `;
  }

  const btn = document.getElementById('kill-switch');
  const killKnown = kill && typeof kill.running === 'boolean';
  btn.textContent = killKnown ? (kill.running ? '⏹ STOP ALL' : '◼ STOPPED') : '◼ UNAVAILABLE';
  btn.disabled = !killKnown || !kill.running;
  btn.style.background = killKnown && kill.running ? 'var(--danger)' : 'var(--border)';

  updateBadge('approval-badge', countIfKnown(snapshot.pending_approvals));
  updateBadge('exception-badge', countIfKnown(snapshot.exceptions));
}

async function loadApprovals(snapshot) {
  const approvals = snapshot?.pending_approvals;
  const container = document.getElementById('approval-cards');
  if (!Array.isArray(approvals)) {
    renderUnavailable(container, 'Approval queue is unavailable until the runtime supplies normalized actions.');
  } else if (approvals.length === 0) {
    renderEmpty(container, 'No pending approvals.');
  } else {
    container.innerHTML = approvals.map(a => {
      const card = a.card || {};
      const action = '<div class="detail unavailable">Approval issuance is unavailable until an authenticated runtime issuer is connected.</div>';
      return `
        <div class="card">
          <h3>${esc(card.business_effect)}</h3>
          <div class="detail">Target: ${esc(card.target_description)}</div>
          <div class="detail risk">${esc(card.policy_reason)}</div>
          <div class="detail">Reversible: ${card.reversible ? 'Yes' : 'No'}</div>
          <div class="detail">Expires: ${esc(card.expires_at)}</div>
          <div class="detail">Evidence: ${esc(card.evidence_summary)}</div>
          <div class="detail action-id">Digest: ${esc(a.action_digest)}</div>
          ${action}
        </div>
      `;
    }).join('');
  }
  updateBadge('approval-badge', countIfKnown(approvals));
}

async function loadExceptions(snapshot) {
  const exceptions = snapshot?.exceptions;
  const container = document.getElementById('exception-cards');
  if (!Array.isArray(exceptions)) {
    renderUnavailable(container, 'Exception state is unavailable until the runtime connects.');
  } else if (exceptions.length === 0) {
    renderEmpty(container, 'No active exceptions.');
  } else {
    container.innerHTML = exceptions.map(e => `
      <div class="card">
        <h3>${esc(e.what_blocked)}</h3>
        <div class="detail">${esc(e.why)}</div>
        <div class="detail risk">${esc(e.consequences)}</div>
        <div class="detail">Next step: ${esc(e.suggested_next_step)}</div>
        <div class="detail">Evidence: ${esc((e.evidence_refs || []).join(', ') || 'Unavailable')}</div>
        <div class="safe-choices">
          ${(e.safe_choices || []).map(c => `<div class="safe-choice"><strong>${esc(c.label)}</strong><span>${esc(c.description)}</span></div>`).join('')}
        </div>
      </div>
    `).join('');
  }
  updateBadge('exception-badge', countIfKnown(exceptions));
}

async function loadEvidence(snapshot) {
  const evidence = snapshot?.evidence;
  const container = document.getElementById('evidence-list');
  if (!Array.isArray(evidence)) {
    renderUnavailable(container, 'Evidence is unavailable until the runtime supplies audit/evidence records.');
  } else if (evidence.length === 0) {
    renderEmpty(container, 'No evidence records.');
  } else {
    container.innerHTML = evidence.map(e => `
      <div class="evidence-item">
        <div>
          <div class="operation">${esc(e.operation)}</div>
          <div class="action-id">${esc(e.action_id)}${e.target ? ` · ${esc(e.target)}` : ''}</div>
          ${e.verification ? `<div class="detail">${esc(e.verification)}</div>` : ''}
        </div>
        <span class="trust-label trust-${trustClass(e.trust_label)}">${esc(e.trust_label)}</span>
      </div>
    `).join('');
  }
}

async function loadPermissions(snapshot) {
  const result = snapshot?.permissions;
  const container = document.getElementById('permission-list');
  if (!result || !Array.isArray(result.permissions)) {
    renderUnavailable(container, 'Permission state is unavailable until the local runtime reports it.');
  } else if (result.permissions.length === 0) {
    renderEmpty(container, 'No permission records.');
  } else {
    container.innerHTML = result.permissions.map(p => `
      <div class="permission-item">
        <div>
          <div class="name">${esc(p.capability)}</div>
          <div class="description">${esc(p.description)}</div>
        </div>
        <span class="permission-state ${p.state === 'GRANTED' ? 'permission-granted' : 'permission-not-granted'}">
          ${esc(p.state)}${p.required ? ' · required' : ''}
        </span>
      </div>
    `).join('');
  }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

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

function countIfKnown(value) {
  return Array.isArray(value) ? value.length : null;
}

function renderUnavailable(container, message) {
  container.innerHTML = `<div class="empty-state unavailable">${esc(message)}</div>`;
}

function renderEmpty(container, message) {
  container.innerHTML = `<div class="empty-state">${esc(message)}</div>`;
}

function metric(value, suffix = '') {
  return value === null || value === undefined ? 'Unavailable' : `${esc(value)}${suffix}`;
}

function renderEconomics(economics) {
  if (!economics) {
    return '<div class="task-card"><h3>Economics</h3><div class="detail unavailable">Runtime economics unavailable.</div></div>';
  }
  return `
    <div class="task-card">
      <h3>Economics</h3>
      <div class="task-meta">
        <span>Verified units: ${metric(economics.verified_work_units)}</span>
        <span>Model cost: ${metric(economics.model_cost_micro_usd, ' micro-USD')}</span>
        <span>Human intervention: ${metric(economics.human_intervention_minutes, ' min')}</span>
        <span>Minutes released: ${metric(economics.released_human_minutes, ' min')}</span>
      </div>
    </div>
  `;
}

function trustClass(label) {
  return String(label || 'unknown').toLowerCase().split(/[^a-z]+/)[0] || 'unknown';
}

function updateBadge(id, count) {
  const el = document.getElementById(id);
  if (!el) return;
  if (typeof count === 'number' && count > 0) {
    el.textContent = count;
    el.classList.remove('hidden');
  } else {
    el.classList.add('hidden');
  }
}

function updateConnection(connection, execution) {
  const el = document.getElementById('connection-status');
  if (!el) return;
  const labels = {
    connected: '● Runtime connected',
    disconnected: '● Runtime disconnected',
    unknown: '● Runtime unavailable',
  };
  const label = labels[connection] || labels.unknown;
  el.textContent = execution === 'unavailable' && connection === 'connected'
    ? `${label} · execution unavailable`
    : execution === 'stopped'
      ? `${label} · stopped`
      : label;
  el.style.color = connection === 'connected' ? 'var(--success)' : 'var(--warning)';
}

// Auto-refresh every 5 seconds.
setInterval(refresh, 5000);

// Initial load.
document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('.nav-btn').forEach(button => {
    button.addEventListener('click', () => showView(button.dataset.view));
  });
  document.getElementById('kill-switch')?.addEventListener('click', emergencyStop);
  refresh();
});
