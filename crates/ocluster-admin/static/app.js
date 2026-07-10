/**
 * Ollama Cluster admin panel SPA.
 */

const state = {
  cluster: null,
  nodes: [],
  models: [],
  events: [],
  requests: [],
  pollMs: 2000,
  pollTimer: null,
  lastPollAt: null,
  pollError: null,
  route: parseRoute(),
};

const contentEl = document.getElementById('content');
const pageTitleEl = document.getElementById('page-title');
const heartbeatEl = document.getElementById('heartbeat');
const heartbeatLabelEl = document.getElementById('heartbeat-label');
const pollIntervalEl = document.getElementById('poll-interval');
const refreshBtn = document.getElementById('refresh-btn');
const modalBackdrop = document.getElementById('modal-backdrop');
const modalEl = document.getElementById('modal');
const toastEl = document.getElementById('toast');

/**
 * Parse the current hash route.
 */
function parseRoute() {
  const hash = window.location.hash.slice(1) || '/';
  const parts = hash.split('/').filter(Boolean);

  if (parts.length === 0) {
    return { name: 'dashboard' };
  }
  if (parts[0] === 'nodes' && parts.length === 1) {
    return { name: 'nodes' };
  }
  if (parts[0] === 'nodes' && parts.length === 2) {
    return { name: 'node-detail', nodeName: decodeURIComponent(parts[1]) };
  }
  if (parts[0] === 'models' && parts.length === 1) {
    return { name: 'models' };
  }
  if (parts[0] === 'models' && parts.length === 2) {
    return { name: 'model-detail', modelName: decodeURIComponent(parts[1]) };
  }
  if (parts[0] === 'events') {
    return { name: 'events' };
  }
  return { name: 'dashboard' };
}

/**
 * Navigate to a hash route.
 */
function navigate(path) {
  window.location.hash = path;
}

/**
 * Show a transient toast message.
 */
function showToast(message, { isError = false } = {}) {
  toastEl.textContent = message;
  toastEl.classList.toggle('error', isError);
  toastEl.classList.remove('hidden');
  setTimeout(() => toastEl.classList.add('hidden'), 3500);
}

/**
 * Open a modal dialog.
 */
function openModal({ title, bodyHtml, onSubmit }) {
  modalEl.innerHTML = `
    <h3>${title}</h3>
    ${bodyHtml}
    <div class="modal-actions">
      <button type="button" class="btn btn-secondary" id="modal-cancel">Cancel</button>
      <button type="button" class="btn btn-primary" id="modal-submit">Save</button>
    </div>
  `;
  modalBackdrop.classList.remove('hidden');
  document.getElementById('modal-cancel').onclick = closeModal;
  document.getElementById('modal-submit').onclick = async () => {
    try {
      await onSubmit();
      closeModal();
    } catch (err) {
      showToast(err.message, { isError: true });
    }
  };
}

/**
 * Close the modal dialog.
 */
function closeModal() {
  modalBackdrop.classList.add('hidden');
  modalEl.innerHTML = '';
}

/**
 * Update heartbeat indicator UI.
 */
function updateHeartbeat() {
  heartbeatEl.classList.remove('ok', 'error', 'idle');
  if (state.pollError) {
    heartbeatEl.classList.add('error');
    heartbeatLabelEl.textContent = `Disconnected — ${state.pollError}`;
    return;
  }
  if (!state.lastPollAt) {
    heartbeatEl.classList.add('idle');
    heartbeatLabelEl.textContent = 'Connecting…';
    return;
  }
  heartbeatEl.classList.add('ok');
  const ago = Math.round((Date.now() - state.lastPollAt) / 1000);
  heartbeatLabelEl.textContent = `Live — polled ${ago}s ago`;
}

/**
 * Fetch dashboard data from the management API.
 */
async function refreshData() {
  try {
    const [cluster, nodes, models, events, requests] = await Promise.all([
      api.clusterStatus(),
      api.listNodes(),
      api.listModels(),
      api.listEvents(),
      api.listRequests(),
    ]);
    state.cluster = cluster;
    state.nodes = nodes;
    state.models = models;
    state.events = events;
    state.requests = requests;
    state.lastPollAt = Date.now();
    state.pollError = null;
  } catch (err) {
    state.pollError = err.message;
  }
  updateHeartbeat();
  render();
}

/**
 * Start or restart the polling loop.
 */
function startPolling() {
  if (state.pollTimer) {
    clearInterval(state.pollTimer);
  }
  refreshData();
  state.pollTimer = setInterval(refreshData, state.pollMs);
}

/**
 * Format runtime state for display.
 */
function formatState(value) {
  if (!value) return 'unknown';
  if (typeof value === 'object') {
    return Object.keys(value)[0] || 'unknown';
  }
  return String(value);
}

/**
 * CSS class for node runtime state.
 */
function runtimeClass(runtimeState) {
  const key = formatState(runtimeState).toLowerCase();
  return key;
}

/**
 * Render node positions on the cluster visualisation ring.
 */
function renderClusterViz() {
  const nodes = state.nodes;
  const count = nodes.length || 1;
  const chips = nodes.map((node, index) => {
    const angle = (index / count) * 2 * Math.PI - Math.PI / 2;
    const radius = 42;
    const x = 50 + radius * Math.cos(angle);
    const y = 50 + radius * Math.sin(angle);
    const rt = runtimeClass(node.runtime_state);
    return `
      <div class="node-orbit" style="left:${x}%; top:${y}%">
        <div class="node-chip ${rt}" data-node="${escapeHtml(node.name)}">
          <span class="name">${escapeHtml(node.name)}</span>
          <span class="state">${escapeHtml(formatState(node.runtime_state))}</span>
        </div>
      </div>
    `;
  }).join('');

  const ringClass = state.pollError ? '' : 'pulse-ring';
  const coreState = state.cluster?.state || 'unknown';

  return `
    <div class="cluster-viz">
      <div class="cluster-viz-ring ${ringClass}"></div>
      <div class="cluster-core">${escapeHtml(coreState)}<br>controller</div>
      ${chips || '<p class="empty" style="position:absolute;bottom:0;width:100%">No nodes registered</p>'}
    </div>
  `;
}

/**
 * Escape HTML entities in user-provided strings.
 */
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

/**
 * Render the dashboard page.
 */
function renderDashboard() {
  const c = state.cluster;
  pageTitleEl.textContent = 'Dashboard';

  contentEl.innerHTML = `
    <div class="stats-grid">
      <div class="stat-card"><div class="label">Nodes</div><div class="value">${c?.nodes_total ?? '—'}</div></div>
      <div class="stat-card"><div class="label">Ready</div><div class="value ok">${c?.nodes_ready ?? '—'}</div></div>
      <div class="stat-card"><div class="label">Unavailable</div><div class="value bad">${c?.nodes_unavailable ?? '—'}</div></div>
      <div class="stat-card"><div class="label">Models</div><div class="value">${c?.models_total ?? '—'}</div></div>
      <div class="stat-card"><div class="label">Active requests</div><div class="value">${c?.active_requests ?? '—'}</div></div>
      <div class="stat-card"><div class="label">Uptime</div><div class="value">${c ? formatUptime(c.uptime_seconds) : '—'}</div></div>
    </div>

    <div class="layout-split">
      <div class="panel">
        <div class="panel-header"><h2>Cluster heartbeat</h2></div>
        <div class="panel-body">${renderClusterViz()}</div>
      </div>
      <div class="panel">
        <div class="panel-header"><h2>Recent events</h2></div>
        <div class="panel-body">${renderEventsList(state.events.slice(0, 8))}</div>
      </div>
    </div>
  `;

  contentEl.querySelectorAll('.node-chip').forEach((chip) => {
    chip.addEventListener('click', () => {
      navigate(`/nodes/${encodeURIComponent(chip.dataset.node)}`);
    });
  });
}

/**
 * Format uptime seconds as human-readable string.
 */
function formatUptime(seconds) {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

/**
 * Render events list HTML.
 */
function renderEventsList(events) {
  if (!events.length) {
    return '<p class="empty">No events yet</p>';
  }
  return `<ul class="event-list">${events.map((ev) => `
    <li>
      <span class="time">${escapeHtml(ev.created_at)}</span>
      <strong>${escapeHtml(ev.event_type)}</strong>
      ${ev.target ? ` — ${escapeHtml(ev.target)}` : ''}
      <div>${escapeHtml(ev.message)}</div>
    </li>
  `).join('')}</ul>`;
}

/**
 * Render the nodes list page with CRUD controls.
 */
function renderNodesPage() {
  pageTitleEl.textContent = 'Nodes';

  const rows = state.nodes.map((node) => {
    const rt = formatState(node.runtime_state);
    const admin = formatState(node.admin_state);
    return `
      <tr class="clickable" data-node="${escapeHtml(node.name)}">
        <td><strong>${escapeHtml(node.name)}</strong></td>
        <td>${escapeHtml(node.url)}</td>
        <td><span class="badge ${rt}">${escapeHtml(rt)}</span></td>
        <td><span class="badge ${admin}">${escapeHtml(admin)}</span></td>
        <td>${node.active_requests}</td>
        <td>${node.model_count}</td>
        <td class="actions" onclick="event.stopPropagation()">
          <button class="btn btn-sm btn-secondary" data-action="edit" data-node="${escapeHtml(node.name)}">Edit</button>
          <button class="btn btn-sm btn-danger" data-action="remove" data-node="${escapeHtml(node.name)}">Remove</button>
        </td>
      </tr>
    `;
  }).join('');

  contentEl.innerHTML = `
    <div class="panel">
      <div class="panel-header">
        <h2>Registered nodes</h2>
        <button class="btn btn-primary" id="add-node-btn" type="button">Add node</button>
      </div>
      <div class="panel-body">
        ${state.nodes.length ? `
          <table>
            <thead>
              <tr>
                <th>Name</th><th>URL</th><th>Runtime</th><th>Admin</th><th>Requests</th><th>Models</th><th></th>
              </tr>
            </thead>
            <tbody>${rows}</tbody>
          </table>
        ` : '<p class="empty">No nodes registered. Add one to get started.</p>'}
      </div>
    </div>
  `;

  document.getElementById('add-node-btn')?.addEventListener('click', showAddNodeModal);
  contentEl.querySelectorAll('tr.clickable').forEach((row) => {
    row.addEventListener('click', () => {
      navigate(`/nodes/${encodeURIComponent(row.dataset.node)}`);
    });
  });
  contentEl.querySelectorAll('[data-action="edit"]').forEach((btn) => {
    btn.addEventListener('click', () => showEditNodeModal(btn.dataset.node));
  });
  contentEl.querySelectorAll('[data-action="remove"]').forEach((btn) => {
    btn.addEventListener('click', () => confirmRemoveNode(btn.dataset.node));
  });
}

/**
 * Show modal to add a new node.
 */
function showAddNodeModal() {
  openModal({
    title: 'Add node',
    bodyHtml: `
      <div class="form-group"><label>Name</label><input id="node-name" placeholder="gpu-01"></div>
      <div class="form-group"><label>URL</label><input id="node-url" placeholder="http://127.0.0.1:11434"></div>
      <div class="form-group"><label>Model mode</label>
        <select id="node-mode">
          <option value="discover">discover</option>
          <option value="allow">allow</option>
          <option value="static">static</option>
        </select>
      </div>
      <div class="form-group"><label>Max concurrent</label><input id="node-max" type="number" value="8"></div>
    `,
    onSubmit: async () => {
      const name = document.getElementById('node-name').value.trim();
      const url = document.getElementById('node-url').value.trim();
      const model_mode = document.getElementById('node-mode').value;
      const max = parseInt(document.getElementById('node-max').value, 10);
      if (!name || !url) throw new Error('Name and URL are required');
      const resp = await api.addNode({ name, url, model_mode, max_concurrent: max });
      showToast(resp.message);
      await refreshData();
    },
  });
}

/**
 * Show modal to edit an existing node.
 */
async function showEditNodeModal(name) {
  const node = state.nodes.find((n) => n.name === name) || await api.getNode(name);
  openModal({
    title: `Edit ${name}`,
    bodyHtml: `
      <div class="form-group"><label>URL</label><input id="edit-url" value="${escapeHtml(node.url)}"></div>
      <div class="form-group"><label>Max concurrent</label><input id="edit-max" type="number" value="${node.max_concurrent || 8}"></div>
    `,
    onSubmit: async () => {
      const url = document.getElementById('edit-url').value.trim();
      const max_concurrent = parseInt(document.getElementById('edit-max').value, 10);
      const resp = await api.updateNode(name, { url, max_concurrent });
      showToast(resp.message);
      await refreshData();
    },
  });
}

/**
 * Confirm and remove a node.
 */
async function confirmRemoveNode(name) {
  if (!window.confirm(`Remove node "${name}"?`)) return;
  try {
    const resp = await api.removeNode(name);
    showToast(resp.message);
    await refreshData();
  } catch (err) {
    showToast(err.message, { isError: true });
  }
}

/**
 * Render node detail page with actions and models.
 */
async function renderNodeDetailPage(nodeName) {
  pageTitleEl.textContent = `Node: ${nodeName}`;

  let node;
  try {
    node = await api.getNode(nodeName);
  } catch (err) {
    contentEl.innerHTML = `<p class="empty">Node not found: ${escapeHtml(err.message)}</p>`;
    return;
  }

  const rt = formatState(node.runtime_state);
  const admin = formatState(node.admin_state);
  const models = (node.models || []).map((m) => `<li>${escapeHtml(m)}</li>`).join('');

  contentEl.innerHTML = `
    <p style="margin-bottom:1rem"><a href="#/nodes" style="color:var(--accent)">← Back to nodes</a></p>
    <div class="detail-grid">
      <div class="detail-item"><label>URL</label><span>${escapeHtml(node.url)}</span></div>
      <div class="detail-item"><label>Runtime</label><span class="badge ${rt}">${escapeHtml(rt)}</span></div>
      <div class="detail-item"><label>Admin</label><span class="badge ${admin}">${escapeHtml(admin)}</span></div>
      <div class="detail-item"><label>Ollama version</label><span>${escapeHtml(node.ollama_version || '—')}</span></div>
      <div class="detail-item"><label>Active requests</label><span>${node.active_requests}</span></div>
      <div class="detail-item"><label>Loaded models</label><span>${node.loaded_models}</span></div>
      <div class="detail-item"><label>Last contact</label><span>${escapeHtml(node.last_contact || '—')}</span></div>
      <div class="detail-item"><label>Max concurrent</label><span>${node.max_concurrent}</span></div>
    </div>

    <div class="panel">
      <div class="panel-header"><h2>Actions</h2></div>
      <div class="panel-body actions">
        <button class="btn btn-primary" data-action="enable">Enable</button>
        <button class="btn btn-secondary" data-action="disable">Disable</button>
        <button class="btn btn-secondary" data-action="drain">Drain</button>
        <button class="btn btn-secondary" data-action="probe">Probe</button>
        <button class="btn btn-secondary" data-action="sync">Sync models</button>
        <button class="btn btn-secondary" data-action="edit">Edit</button>
      </div>
    </div>

    <div class="panel">
      <div class="panel-header"><h2>Discovered models</h2></div>
      <div class="panel-body">
        ${models ? `<ul>${models}</ul>` : '<p class="empty">No models discovered on this node</p>'}
      </div>
    </div>
  `;

  contentEl.querySelectorAll('[data-action]').forEach((btn) => {
    btn.addEventListener('click', () => handleNodeAction(nodeName, btn.dataset.action));
  });
}

/**
 * Execute a node management action.
 */
async function handleNodeAction(nodeName, action) {
  try {
    let resp;
    if (action === 'enable') resp = await api.enableNode(nodeName);
    else if (action === 'disable') resp = await api.disableNode(nodeName);
    else if (action === 'drain') resp = await api.drainNode(nodeName);
    else if (action === 'probe') resp = await api.probeNode(nodeName);
    else if (action === 'sync') resp = await api.syncModels();
    else if (action === 'edit') {
      showEditNodeModal(nodeName);
      return;
    }
    showToast(resp?.message || 'Action completed');
    await refreshData();
    if (state.route.name === 'node-detail') {
      await renderNodeDetailPage(nodeName);
    }
  } catch (err) {
    showToast(err.message, { isError: true });
  }
}

/**
 * Render global models page.
 */
function renderModelsPage() {
  pageTitleEl.textContent = 'Models';

  const rows = state.models.map((model) => `
    <tr class="clickable" data-model="${escapeHtml(model.name)}">
      <td><strong>${escapeHtml(model.name)}</strong></td>
      <td>${model.node_count}</td>
      <td>${model.ready_nodes}</td>
      <td>${model.loaded_instances}</td>
      <td>${model.active_requests}</td>
    </tr>
  `).join('');

  contentEl.innerHTML = `
    <div class="panel">
      <div class="panel-header">
        <h2>Cluster models</h2>
        <button class="btn btn-primary" id="sync-models-btn" type="button">Sync all models</button>
      </div>
      <div class="panel-body">
        ${state.models.length ? `
          <table>
            <thead>
              <tr><th>Model</th><th>Nodes</th><th>Ready</th><th>Loaded</th><th>Requests</th></tr>
            </thead>
            <tbody>${rows}</tbody>
          </table>
        ` : '<p class="empty">No models discovered yet. Add nodes and sync.</p>'}
      </div>
    </div>
  `;

  document.getElementById('sync-models-btn')?.addEventListener('click', async () => {
    try {
      const resp = await api.syncModels();
      showToast(resp.message);
      await refreshData();
    } catch (err) {
      showToast(err.message, { isError: true });
    }
  });

  contentEl.querySelectorAll('tr.clickable').forEach((row) => {
    row.addEventListener('click', () => {
      navigate(`/models/${encodeURIComponent(row.dataset.model)}`);
    });
  });
}

/**
 * Render model detail page with per-node breakdown.
 */
async function renderModelDetailPage(modelName) {
  pageTitleEl.textContent = `Model: ${modelName}`;

  let model;
  let explain;
  try {
    [model, explain] = await Promise.all([
      api.getModel(modelName),
      api.explainModel(modelName).catch(() => null),
    ]);
  } catch (err) {
    contentEl.innerHTML = `<p class="empty">Model not found: ${escapeHtml(err.message)}</p>`;
    return;
  }

  const rows = (model.nodes || []).map((n) => `
    <tr>
      <td><a href="#/nodes/${encodeURIComponent(n.node)}" style="color:var(--accent)">${escapeHtml(n.node)}</a></td>
      <td>${n.available ? 'yes' : 'no'}</td>
      <td>${n.loaded ? 'yes' : 'no'}</td>
      <td>${escapeHtml(n.digest || '—')}</td>
      <td>${n.size ?? '—'}</td>
    </tr>
  `).join('');

  const eligible = explain?.eligible?.join(', ') || '—';

  contentEl.innerHTML = `
    <p style="margin-bottom:1rem"><a href="#/models" style="color:var(--accent)">← Back to models</a></p>
    <div class="panel">
      <div class="panel-header">
        <h2>${escapeHtml(modelName)}</h2>
        <button class="btn btn-primary" id="sync-models-btn" type="button">Sync models</button>
      </div>
      <div class="panel-body">
        <p style="margin-bottom:1rem;color:var(--text-muted)">Eligible nodes for routing: ${escapeHtml(eligible)}</p>
        <table>
          <thead><tr><th>Node</th><th>Available</th><th>Loaded</th><th>Digest</th><th>Size</th></tr></thead>
          <tbody>${rows || '<tr><td colspan="5">No nodes</td></tr>'}</tbody>
        </table>
      </div>
    </div>
  `;

  document.getElementById('sync-models-btn')?.addEventListener('click', async () => {
    try {
      const resp = await api.syncModels();
      showToast(resp.message);
      await renderModelDetailPage(modelName);
    } catch (err) {
      showToast(err.message, { isError: true });
    }
  });
}

/**
 * Render events page.
 */
function renderEventsPage() {
  pageTitleEl.textContent = 'Events';
  contentEl.innerHTML = `
    <div class="panel">
      <div class="panel-header"><h2>Cluster events</h2></div>
      <div class="panel-body">${renderEventsList(state.events)}</div>
    </div>
  `;
}

/**
 * Route to the correct page renderer.
 */
async function render() {
  updateNavActive();
  const route = state.route;

  if (route.name === 'dashboard') {
    renderDashboard();
    return;
  }
  if (route.name === 'nodes') {
    renderNodesPage();
    return;
  }
  if (route.name === 'node-detail') {
    await renderNodeDetailPage(route.nodeName);
    return;
  }
  if (route.name === 'models') {
    renderModelsPage();
    return;
  }
  if (route.name === 'model-detail') {
    await renderModelDetailPage(route.modelName);
    return;
  }
  if (route.name === 'events') {
    renderEventsPage();
  }
}

/**
 * Highlight active nav link.
 */
function updateNavActive() {
  document.querySelectorAll('.nav-link').forEach((link) => {
    const route = link.dataset.route;
    const active =
      (route === 'dashboard' && state.route.name === 'dashboard') ||
      (route === 'nodes' && state.route.name.startsWith('node')) ||
      (route === 'models' && state.route.name.startsWith('model')) ||
      (route === 'events' && state.route.name === 'events');
    link.classList.toggle('active', active);
  });
}

/**
 * Handle hash route changes.
 */
function onRouteChange() {
  state.route = parseRoute();
  render();
}

pollIntervalEl.addEventListener('change', () => {
  state.pollMs = parseInt(pollIntervalEl.value, 10);
  localStorage.setItem('ocluster-poll-ms', String(state.pollMs));
  startPolling();
});

refreshBtn.addEventListener('click', refreshData);

window.addEventListener('hashchange', onRouteChange);

modalBackdrop.addEventListener('click', (ev) => {
  if (ev.target === modalBackdrop) closeModal();
});

const savedPoll = localStorage.getItem('ocluster-poll-ms');
if (savedPoll) {
  state.pollMs = parseInt(savedPoll, 10);
  pollIntervalEl.value = savedPoll;
}

startPolling();
setInterval(updateHeartbeat, 500);
