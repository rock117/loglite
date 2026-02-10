<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import axios from 'axios'

type App = {
  app_id: string
  name: string
  created_at: string
}

type Source = {
  id: number
  app_id: string
  kind: string
  path: string
  recursive: boolean
  encoding: string
  include_glob: string | null
  exclude_glob: string | null
  enabled: boolean
  created_at: string
}

type SearchItem = {
  id: number
  app_id: string
  ts: string
  host: string
  source: string
  sourcetype: string | null
  severity: number | null
  message: string
  fields: any
}

type SearchResponse = {
  total: number
  items: SearchItem[]
}

// State
const currentView = ref<'search' | 'sources' | 'apps'>('search')
const apps = ref<App[]>([])
const selectedAppId = ref<string>('')
const sources = ref<Source[]>([])

// Search state
const q = ref('')
const limit = ref(100)
const loading = ref(false)
const error = ref<string | null>(null)
const total = ref<number>(0)
const items = ref<SearchItem[]>([])

// Source form state
const showSourceForm = ref(false)
const sourceForm = ref({
  path: '',
  recursive: true,
  include_glob: '*.log',
  exclude_glob: '*.gz',
  enabled: true
})

// App form state
const showAppForm = ref(false)
const appName = ref('')

const selectedApp = computed(() => apps.value.find(a => a.app_id === selectedAppId.value))

// Load apps on mount
onMounted(async () => {
  await loadApps()
  // Load selected app from localStorage
  const saved = localStorage.getItem('loglite_selected_app')
  if (saved && apps.value.find(a => a.app_id === saved)) {
    selectedAppId.value = saved
  } else if (apps.value.length > 0) {
    selectedAppId.value = apps.value[0].app_id
  }
  if (selectedAppId.value) {
    await loadSources()
  }
})

async function loadApps() {
  try {
    const res = await axios.get<App[]>('/api/apps')
    apps.value = res.data
  } catch (e: any) {
    error.value = 'Failed to load apps: ' + (e?.message ?? 'unknown error')
  }
}

async function createApp() {
  if (!appName.value.trim()) return
  try {
    const res = await axios.post<App>('/api/apps', { name: appName.value.trim() })
    apps.value.push(res.data)
    selectedAppId.value = res.data.app_id
    localStorage.setItem('loglite_selected_app', res.data.app_id)
    appName.value = ''
    showAppForm.value = false
  } catch (e: any) {
    error.value = 'Failed to create app: ' + (e?.message ?? 'unknown error')
  }
}

async function selectApp(appId: string) {
  selectedAppId.value = appId
  localStorage.setItem('loglite_selected_app', appId)
  await loadSources()
  items.value = []
  total.value = 0
}

async function loadSources() {
  if (!selectedAppId.value) return
  try {
    const res = await axios.get<Source[]>(`/api/sources?app_id=${selectedAppId.value}`)
    sources.value = res.data
  } catch (e: any) {
    error.value = 'Failed to load sources: ' + (e?.message ?? 'unknown error')
  }
}

async function createSource() {
  if (!selectedAppId.value || !sourceForm.value.path.trim()) return
  try {
    await axios.post('/api/sources', {
      app_id: selectedAppId.value,
      kind: 'tail',
      ...sourceForm.value
    })
    await loadSources()
    sourceForm.value = {
      path: '',
      recursive: true,
      include_glob: '*.log',
      exclude_glob: '*.gz',
      enabled: true
    }
    showSourceForm.value = false
  } catch (e: any) {
    error.value = 'Failed to create source: ' + (e?.message ?? 'unknown error')
  }
}

async function toggleSource(source: Source) {
  try {
    await axios.put(`/api/sources/${source.id}`, { enabled: !source.enabled })
    await loadSources()
  } catch (e: any) {
    error.value = 'Failed to toggle source: ' + (e?.message ?? 'unknown error')
  }
}

async function deleteSource(id: number) {
  if (!confirm('Delete this source?')) return
  try {
    await axios.delete(`/api/sources/${id}`)
    await loadSources()
  } catch (e: any) {
    error.value = 'Failed to delete source: ' + (e?.message ?? 'unknown error')
  }
}

async function doSearch() {
  if (!selectedAppId.value) {
    error.value = 'Please select an application first'
    return
  }
  loading.value = true
  error.value = null
  try {
    const payload = {
      app_id: selectedAppId.value,
      q: q.value.trim() || undefined,
      limit: limit.value
    }
    const res = await axios.post<SearchResponse>('/api/search', payload)
    total.value = res.data.total
    items.value = res.data.items
  } catch (e: any) {
    error.value = e?.message ?? 'request failed'
  } finally {
    loading.value = false
  }
}

function formatTimestamp(ts: string): string {
  return new Date(ts).toLocaleString()
}

function getSeverityLabel(severity: number | null): string {
  if (severity === null) return ''
  const labels: Record<number, string> = {
    3: 'ERROR',
    4: 'WARN',
    6: 'INFO',
    7: 'DEBUG'
  }
  return labels[severity] || String(severity)
}

function getSeverityClass(severity: number | null): string {
  if (severity === null) return ''
  if (severity <= 3) return 'severity-error'
  if (severity === 4) return 'severity-warn'
  if (severity === 6) return 'severity-info'
  return 'severity-debug'
}
</script>

<template>
  <div class="app">
    <!-- Header -->
    <header class="header">
      <div class="header-left">
        <div class="logo">Loglite</div>
        <div class="subtitle">Multi-App Log Search</div>
      </div>
      <div class="header-right">
        <select v-model="selectedAppId" class="app-selector" @change="selectApp(selectedAppId)">
          <option value="">Select Application...</option>
          <option v-for="app in apps" :key="app.app_id" :value="app.app_id">
            {{ app.name }}
          </option>
        </select>
        <button class="btn-icon" @click="showAppForm = true" title="Create App">+</button>
      </div>
    </header>

    <!-- Navigation -->
    <nav class="nav">
      <button 
        :class="['nav-btn', { active: currentView === 'search' }]" 
        @click="currentView = 'search'"
      >
        Search
      </button>
      <button 
        :class="['nav-btn', { active: currentView === 'sources' }]" 
        @click="currentView = 'sources'"
        :disabled="!selectedAppId"
      >
        Sources
      </button>
      <button 
        :class="['nav-btn', { active: currentView === 'apps' }]" 
        @click="currentView = 'apps'"
      >
        Apps
      </button>
    </nav>

    <!-- Error Banner -->
    <div v-if="error" class="error-banner">
      {{ error }}
      <button class="error-close" @click="error = null">×</button>
    </div>

    <!-- Main Content -->
    <main class="content">
      <!-- Search View -->
      <div v-if="currentView === 'search'" class="view">
        <div v-if="!selectedAppId" class="empty-state">
          <p>Please select an application to search logs</p>
        </div>
        <div v-else>
          <div class="search-controls">
            <input 
              v-model="q" 
              class="input search-input" 
              placeholder="Search logs (e.g., ERROR, exception, user_id:123)" 
              @keydown.enter="doSearch" 
            />
            <input 
              v-model.number="limit" 
              class="input limit-input" 
              type="number" 
              min="1" 
              max="1000" 
              title="Result limit"
            />
            <button class="btn btn-primary" :disabled="loading" @click="doSearch">
              {{ loading ? 'Searching...' : 'Search' }}
            </button>
          </div>

          <div class="search-meta">
            <span>Total: {{ total }}</span>
            <span v-if="selectedApp">App: {{ selectedApp.name }}</span>
          </div>

          <div class="table-container">
            <table class="table">
              <thead>
                <tr>
                  <th style="width: 180px">Timestamp</th>
                  <th style="width: 100px">Severity</th>
                  <th style="width: 120px">Source</th>
                  <th style="width: 120px">Host</th>
                  <th>Message</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in items" :key="item.id">
                  <td class="mono">{{ formatTimestamp(item.ts) }}</td>
                  <td>
                    <span :class="['severity-badge', getSeverityClass(item.severity)]">
                      {{ getSeverityLabel(item.severity) }}
                    </span>
                  </td>
                  <td class="truncate" :title="item.source">{{ item.source }}</td>
                  <td class="truncate" :title="item.host">{{ item.host }}</td>
                  <td class="mono message-cell">{{ item.message }}</td>
                </tr>
                <tr v-if="!loading && items.length === 0">
                  <td colspan="5" class="empty-cell">No results found</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- Sources View -->
      <div v-if="currentView === 'sources'" class="view">
        <div class="view-header">
          <h2>Log Sources</h2>
          <button class="btn btn-primary" @click="showSourceForm = true">
            + Add Source
          </button>
        </div>

        <div v-if="sources.length === 0" class="empty-state">
          <p>No sources configured. Add a source to start monitoring logs.</p>
        </div>

        <div v-else class="sources-grid">
          <div v-for="source in sources" :key="source.id" class="source-card">
            <div class="source-header">
              <div class="source-status" :class="{ enabled: source.enabled }"></div>
              <div class="source-info">
                <div class="source-path">{{ source.path }}</div>
                <div class="source-meta">{{ source.kind }} • Created {{ new Date(source.created_at).toLocaleDateString() }}</div>
              </div>
            </div>
            <div class="source-details">
              <div v-if="source.recursive" class="source-tag">Recursive</div>
              <div v-if="source.include_glob" class="source-tag">Include: {{ source.include_glob }}</div>
              <div v-if="source.exclude_glob" class="source-tag">Exclude: {{ source.exclude_glob }}</div>
            </div>
            <div class="source-actions">
              <button class="btn-sm" @click="toggleSource(source)">
                {{ source.enabled ? 'Disable' : 'Enable' }}
              </button>
              <button class="btn-sm btn-danger" @click="deleteSource(source.id)">
                Delete
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Apps View -->
      <div v-if="currentView === 'apps'" class="view">
        <div class="view-header">
          <h2>Applications</h2>
          <button class="btn btn-primary" @click="showAppForm = true">
            + Create App
          </button>
        </div>

        <div class="apps-grid">
          <div 
            v-for="app in apps" 
            :key="app.app_id" 
            :class="['app-card', { selected: app.app_id === selectedAppId }]"
            @click="selectApp(app.app_id)"
          >
            <div class="app-name">{{ app.name }}</div>
            <div class="app-id">{{ app.app_id }}</div>
            <div class="app-date">Created {{ new Date(app.created_at).toLocaleDateString() }}</div>
          </div>
        </div>
      </div>
    </main>

    <!-- Create App Modal -->
    <div v-if="showAppForm" class="modal" @click.self="showAppForm = false">
      <div class="modal-content">
        <div class="modal-header">
          <h3>Create Application</h3>
          <button class="modal-close" @click="showAppForm = false">×</button>
        </div>
        <div class="modal-body">
          <label class="label">Application Name</label>
          <input 
            v-model="appName" 
            class="input" 
            placeholder="e.g., my-service" 
            @keydown.enter="createApp"
            autofocus
          />
        </div>
        <div class="modal-footer">
          <button class="btn" @click="showAppForm = false">Cancel</button>
          <button class="btn btn-primary" @click="createApp" :disabled="!appName.trim()">
            Create
          </button>
        </div>
      </div>
    </div>

    <!-- Create Source Modal -->
    <div v-if="showSourceForm" class="modal" @click.self="showSourceForm = false">
      <div class="modal-content">
        <div class="modal-header">
          <h3>Add Log Source</h3>
          <button class="modal-close" @click="showSourceForm = false">×</button>
        </div>
        <div class="modal-body">
          <label class="label">Path</label>
          <input 
            v-model="sourceForm.path" 
            class="input" 
            placeholder="/var/log/myapp or /var/log/myapp/app.log" 
          />
          
          <label class="label">Include Pattern</label>
          <input 
            v-model="sourceForm.include_glob" 
            class="input" 
            placeholder="*.log" 
          />
          
          <label class="label">Exclude Pattern</label>
          <input 
            v-model="sourceForm.exclude_glob" 
            class="input" 
            placeholder="*.gz" 
          />
          
          <label class="checkbox-label">
            <input type="checkbox" v-model="sourceForm.recursive" />
            Scan subdirectories recursively
          </label>
          
          <label class="checkbox-label">
            <input type="checkbox" v-model="sourceForm.enabled" />
            Enable immediately
          </label>
        </div>
        <div class="modal-footer">
          <button class="btn" @click="showSourceForm = false">Cancel</button>
          <button class="btn btn-primary" @click="createSource" :disabled="!sourceForm.path.trim()">
            Add Source
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
* {
  box-sizing: border-box;
}

.app {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
  min-height: 100vh;
  background: #f9fafb;
  color: #111827;
}

/* Header */
.header {
  background: white;
  border-bottom: 1px solid #e5e7eb;
  padding: 16px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-left {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

.logo {
  font-size: 24px;
  font-weight: 700;
  color: #111827;
}

.subtitle {
  font-size: 14px;
  color: #6b7280;
}

.header-right {
  display: flex;
  gap: 8px;
  align-items: center;
}

.app-selector {
  height: 38px;
  padding: 0 12px;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  background: white;
  font-size: 14px;
  min-width: 200px;
  cursor: pointer;
}

.app-selector:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.btn-icon {
  width: 38px;
  height: 38px;
  border: 1px solid #d1d5db;
  background: white;
  border-radius: 6px;
  font-size: 20px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-icon:hover {
  background: #f3f4f6;
}

/* Navigation */
.nav {
  background: white;
  border-bottom: 1px solid #e5e7eb;
  padding: 0 24px;
  display: flex;
  gap: 4px;
}

.nav-btn {
  padding: 12px 20px;
  border: none;
  background: none;
  color: #6b7280;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
}

.nav-btn:hover:not(:disabled) {
  color: #111827;
}

.nav-btn.active {
  color: #3b82f6;
  border-bottom-color: #3b82f6;
}

.nav-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Error Banner */
.error-banner {
  background: #fef2f2;
  border: 1px solid #fecaca;
  color: #991b1b;
  padding: 12px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.error-close {
  background: none;
  border: none;
  font-size: 24px;
  color: #991b1b;
  cursor: pointer;
  padding: 0;
  width: 24px;
  height: 24px;
}

/* Content */
.content {
  padding: 24px;
  max-width: 1400px;
  margin: 0 auto;
}

.view {
  background: white;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  padding: 24px;
}

.view-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.view-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: #6b7280;
}

/* Search */
.search-controls {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.search-input {
  flex: 1;
}

.limit-input {
  width: 80px;
}

.search-meta {
  display: flex;
  justify-content: space-between;
  font-size: 14px;
  color: #6b7280;
  margin-bottom: 16px;
}

/* Table */
.table-container {
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  overflow: hidden;
}

.table {
  width: 100%;
  border-collapse: collapse;
}

th {
  background: #f9fafb;
  padding: 12px;
  text-align: left;
  font-size: 12px;
  font-weight: 600;
  color: #6b7280;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

td {
  padding: 12px;
  border-top: 1px solid #f3f4f6;
  font-size: 14px;
  vertical-align: top;
}

.mono {
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace;
  font-size: 13px;
}

.message-cell {
  max-width: 600px;
  word-break: break-word;
  white-space: pre-wrap;
}

.truncate {
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-cell {
  text-align: center;
  color: #9ca3af;
  padding: 40px;
}

.severity-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
}

.severity-error {
  background: #fef2f2;
  color: #991b1b;
}

.severity-warn {
  background: #fffbeb;
  color: #92400e;
}

.severity-info {
  background: #eff6ff;
  color: #1e40af;
}

.severity-debug {
  background: #f3f4f6;
  color: #374151;
}

/* Sources */
.sources-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  gap: 16px;
}

.source-card {
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  padding: 16px;
  background: #fafafa;
}

.source-header {
  display: flex;
  gap: 12px;
  margin-bottom: 12px;
}

.source-status {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #d1d5db;
  margin-top: 4px;
  flex-shrink: 0;
}

.source-status.enabled {
  background: #10b981;
}

.source-info {
  flex: 1;
  min-width: 0;
}

.source-path {
  font-weight: 600;
  font-size: 14px;
  word-break: break-all;
  margin-bottom: 4px;
}

.source-meta {
  font-size: 12px;
  color: #6b7280;
}

.source-details {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 12px;
}

.source-tag {
  font-size: 11px;
  padding: 2px 8px;
  background: #e5e7eb;
  border-radius: 4px;
  color: #374151;
}

.source-actions {
  display: flex;
  gap: 8px;
}

/* Apps */
.apps-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 16px;
}

.app-card {
  border: 2px solid #e5e7eb;
  border-radius: 8px;
  padding: 20px;
  cursor: pointer;
  transition: all 0.2s;
}

.app-card:hover {
  border-color: #3b82f6;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.app-card.selected {
  border-color: #3b82f6;
  background: #eff6ff;
}

.app-name {
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 8px;
}

.app-id {
  font-size: 12px;
  color: #6b7280;
  font-family: monospace;
  margin-bottom: 4px;
}

.app-date {
  font-size: 12px;
  color: #9ca3af;
}

/* Buttons */
.input {
  height: 38px;
  padding: 0 12px;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  font-size: 14px;
  width: 100%;
}

.input:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.btn {
  height: 38px;
  padding: 0 16px;
  border: 1px solid #d1d5db;
  background: white;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn:hover:not(:disabled) {
  background: #f3f4f6;
}

.btn-primary {
  background: #3b82f6;
  color: white;
  border-color: #3b82f6;
}

.btn-primary:hover:not(:disabled) {
  background: #2563eb;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-sm {
  height: 32px;
  padding: 0 12px;
  border: 1px solid #d1d5db;
  background: white;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
}

.btn-sm:hover {
  background: #f3f4f6;
}

.btn-danger {
  color: #dc2626;
  border-color: #fecaca;
}

.btn-danger:hover {
  background: #fef2f2;
}

/* Modal */
.modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: white;
  border-radius: 12px;
  width: 90%;
  max-width: 500px;
  max-height: 90vh;
  overflow: auto;
}

.modal-header {
  padding: 20px 24px;
  border-bottom: 1px solid #e5e7eb;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.modal-header h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.modal-close {
  background: none;
  border: none;
  font-size: 28px;
  color: #6b7280;
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-body {
  padding: 24px;
}

.label {
  display: block;
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 6px;
  margin-top: 16px;
}

.label:first-child {
  margin-top: 0;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 16px;
  font-size: 14px;
  cursor: pointer;
}

.checkbox-label input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
}

.modal-footer {
  padding: 16px 24px;
  border-top: 1px solid #e5e7eb;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
