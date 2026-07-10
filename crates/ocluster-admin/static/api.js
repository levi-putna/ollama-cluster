/**
 * Management API client (proxied through the admin server).
 */
const api = {
  async get(path) {
    const resp = await fetch(path);
    if (!resp.ok) {
      const text = await resp.text();
      throw new Error(text || resp.statusText);
    }
    return resp.json();
  },

  async post(path, body = {}) {
    const resp = await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!resp.ok) {
      const text = await resp.text();
      throw new Error(text || resp.statusText);
    }
    return resp.json();
  },

  async patch(path, body = {}) {
    const resp = await fetch(path, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!resp.ok) {
      const text = await resp.text();
      throw new Error(text || resp.statusText);
    }
    return resp.json();
  },

  async del(path) {
    const resp = await fetch(path, { method: 'DELETE' });
    if (!resp.ok) {
      const text = await resp.text();
      throw new Error(text || resp.statusText);
    }
    return resp.json();
  },

  clusterStatus() {
    return this.get('/api/v1/cluster');
  },

  listNodes() {
    return this.get('/api/v1/nodes');
  },

  getNode(name) {
    return this.get(`/api/v1/nodes/${encodeURIComponent(name)}`);
  },

  addNode(body) {
    return this.post('/api/v1/nodes', body);
  },

  updateNode(name, body) {
    return this.patch(`/api/v1/nodes/${encodeURIComponent(name)}`, body);
  },

  removeNode(name) {
    return this.del(`/api/v1/nodes/${encodeURIComponent(name)}`);
  },

  enableNode(name) {
    return this.post(`/api/v1/nodes/${encodeURIComponent(name)}/enable`);
  },

  disableNode(name) {
    return this.post(`/api/v1/nodes/${encodeURIComponent(name)}/disable`);
  },

  drainNode(name) {
    return this.post(`/api/v1/nodes/${encodeURIComponent(name)}/drain`);
  },

  probeNode(name) {
    return this.post(`/api/v1/nodes/${encodeURIComponent(name)}/probe`);
  },

  listModels() {
    return this.get('/api/v1/models');
  },

  getModel(name) {
    return this.get(`/api/v1/models/${encodeURIComponent(name)}`);
  },

  syncModels() {
    return this.post('/api/v1/models/sync');
  },

  explainModel(name) {
    return this.get(`/api/v1/models/${encodeURIComponent(name)}/explain`);
  },

  listEvents() {
    return this.get('/api/v1/events');
  },

  listRequests() {
    return this.get('/api/v1/requests');
  },
};
