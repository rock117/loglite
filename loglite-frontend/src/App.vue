<script setup lang="ts">
import { ref } from 'vue'
import axios from 'axios'

type SearchItem = {
  id: number
  ts: string
  host: string
  source: string
  sourcetype: string | null
  severity: number | null
  message: string
  fields: unknown
}

type SearchResponse = {
  total: number
  items: SearchItem[]
}

type SearchRequest = {
  q?: string
  limit?: number
}

const q = ref('')
const limit = ref(100)
const loading = ref(false)
const error = ref<string | null>(null)
const total = ref<number>(0)
const items = ref<SearchItem[]>([])

async function doSearch() {
  loading.value = true
  error.value = null
  try {
    const payload: SearchRequest = {
      q: q.value.trim() ? q.value.trim() : undefined,
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
</script>

<template>
  <div class="page">
    <header class="header">
      <div class="title">Loglite</div>
      <div class="subtitle">Single-machine log search</div>
    </header>

    <section class="controls">
      <input v-model="q" class="input" placeholder="Search message contains..." @keydown.enter="doSearch" />
      <input v-model.number="limit" class="input limit" type="number" min="1" max="1000" />
      <button class="btn" :disabled="loading" @click="doSearch">Search</button>
    </section>

    <section v-if="error" class="error">{{ error }}</section>

    <section class="meta">
      <div>Total: {{ total }}</div>
      <div v-if="loading">Loading...</div>
    </section>

    <section class="table-wrap">
      <table class="table">
        <thead>
          <tr>
            <th>ts</th>
            <th>host</th>
            <th>source</th>
            <th>severity</th>
            <th>message</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="it in items" :key="it.id">
            <td class="mono">{{ it.ts }}</td>
            <td>{{ it.host }}</td>
            <td>{{ it.source }}</td>
            <td>{{ it.severity ?? '' }}</td>
            <td class="mono">{{ it.message }}</td>
          </tr>
          <tr v-if="!loading && items.length === 0">
            <td colspan="5" class="empty">No results</td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
.page {
  font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial;
  padding: 20px;
  color: #111827;
}
.header {
  margin-bottom: 16px;
}
.title {
  font-size: 20px;
  font-weight: 700;
}
.subtitle {
  font-size: 12px;
  color: #6b7280;
}
.controls {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 12px;
}
.input {
  height: 34px;
  padding: 0 10px;
  border: 1px solid #d1d5db;
  border-radius: 8px;
  outline: none;
}
.input:focus {
  border-color: #9ca3af;
}
.limit {
  width: 110px;
}
.btn {
  height: 34px;
  padding: 0 12px;
  border: 1px solid #111827;
  background: #111827;
  color: white;
  border-radius: 8px;
  cursor: pointer;
}
.btn:disabled {
  opacity: 0.6;
  cursor: default;
}
.error {
  color: #b91c1c;
  margin: 8px 0;
}
.meta {
  display: flex;
  justify-content: space-between;
  color: #6b7280;
  font-size: 12px;
  margin-bottom: 8px;
}
.table-wrap {
  border: 1px solid #e5e7eb;
  border-radius: 12px;
  overflow: hidden;
}
.table {
  width: 100%;
  border-collapse: collapse;
}
th, td {
  padding: 10px;
  border-bottom: 1px solid #f3f4f6;
  vertical-align: top;
}
th {
  text-align: left;
  background: #f9fafb;
  font-size: 12px;
  color: #374151;
}
.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, Liberation Mono, monospace;
  font-size: 12px;
}
.empty {
  text-align: center;
  color: #6b7280;
  padding: 18px;
}
</style>
