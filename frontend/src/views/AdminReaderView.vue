<script setup lang="ts">
import { t, dateLocale, translateError } from '@/i18n'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { downloadUserWriteup, getGameWriteups, type WriteupRecord } from '@/api/writeup'
import PdfPreview from '@/components/PdfPreview.vue'
import DocxPreview from '@/components/DocxPreview.vue'
import UiIcon from '@/components/UiIcon.vue'

const route = useRoute()
const router = useRouter()
const gameId = computed(() => Number(route.params.gameId))
const items = ref<WriteupRecord[]>([])
const loading = ref(true)
const error = ref('')
const downloadError = ref('')
const downloading = ref<number[]>([])
const search = ref('')
const previewVersion = ref(0)
const selected = computed(() => items.value.find(w => w.id === Number(route.query.writeup)))
const gameName = computed(() => items.value[0]?.game_name || t('game.fallback', { id: gameId.value }))
const filteredItems = computed(() => {
  const query = search.value.trim().toLocaleLowerCase()
  return items.value.filter(w => [w.user_account, w.user_nickname, w.team_name].some(s => s?.toLocaleLowerCase().includes(query)))
})
let loadVersion = 0

async function load() {
  const version = ++loadVersion
  const id = gameId.value
  loading.value = true
  error.value = ''
  items.value = []
  search.value = ''
  try {
    // The API caps each page at 100; include every submission in the reader.
    const records: WriteupRecord[] = []
    for (let page = 1; ; page++) {
      const result = await getGameWriteups(id, { page, page_size: 100 })
      if (version !== loadVersion) return
      records.push(...result.items)
      if (records.length >= result.total || result.items.length === 0) break
    }
    items.value = records
    if (!route.query.writeup && records.length) {
      const first = records.find(w => w.file_type.toLowerCase() === 'pdf')
        ?? records.find(w => w.file_type.toLowerCase() === 'docx')
        ?? records[0]!
      await router.replace({ query: { ...route.query, writeup: first.id } })
    }
  } catch (e: any) {
    if (version === loadVersion) error.value = translateError(e.response?.data?.error, 'reader.loadListFailed')
  } finally {
    if (version === loadVersion) loading.value = false
  }
}

function selectWriteup(w: WriteupRecord) {
  router.push({ query: { ...route.query, writeup: w.id } })
}

async function download(w: WriteupRecord) {
  downloadError.value = ''
  downloading.value.push(w.id)
  try {
    await downloadUserWriteup(gameId.value, w.id)
  } catch {
    downloadError.value = t('reader.downloadFailed', { name: w.file_name })
  } finally {
    downloading.value = downloading.value.filter(id => id !== w.id)
  }
}

function formatDate(value: string) {
  return new Date(value).toLocaleString(dateLocale, { hour12: false })
}

watch(gameId, load, { immediate: true })
onBeforeUnmount(() => { loadVersion++ })
watch(() => selected.value?.id, async () => {
  await nextTick()
  document.querySelector('.writeup-card.active')?.scrollIntoView({ block: 'nearest' })
})
</script>

<template>
  <div class="reader-view">
    <header class="reader-header">
      <RouterLink class="back-link btn btn-ghost btn-sm" :to="{ name: 'admin', params: { gameId } }">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m15 18-6-6 6-6"/></svg>
        {{ t('reader.backAdmin') }}
      </RouterLink>
      <div class="reader-title">
        <h1>{{ gameName }}</h1>
        <span>{{ t('reader.summary', { count: items.length }) }}</span>
      </div>
    </header>

    <div v-if="loading" class="reader-state" role="status">{{ t('reader.loadingList') }}</div>
    <div v-else-if="error" class="reader-state" role="alert">
      <p>{{ error }}</p>
      <button class="text-button" @click="load">{{ t('common.reload') }}</button>
    </div>
    <div v-else-if="!items.length" class="reader-state">{{ t('reader.empty') }}</div>

    <div v-else class="reader-layout">
      <aside class="submission-sidebar" :aria-label="t('reader.listLabel')">
        <div class="sidebar-header">
          <h2>{{ t('reader.list') }} <span>{{ items.length }}</span></h2>
          <div class="search-field"><UiIcon name="search" :size="16" /><input v-model="search" type="search" :placeholder="t('reader.search')" :aria-label="t('reader.searchLabel')" /></div>
        </div>
        <div class="submission-list">
          <div v-for="w in filteredItems" :key="w.id" class="writeup-card" :class="{ active: selected?.id === w.id }">
            <button class="submission-select" :aria-pressed="selected?.id === w.id" @click="selectWriteup(w)">
              <span class="submitter" :title="`${w.user_nickname} (${w.user_account})`">{{ w.user_nickname || w.user_account }}</span>
              <span class="account">{{ w.user_account }}</span>
              <span class="team" :title="w.team_name || t('reader.noTeam')">{{ w.team_name || t('reader.noTeam') }}</span>
              <span class="submitted-at">{{ t('reader.submittedAt', { time: formatDate(w.updated_at) }) }}</span>
              <span class="file-type">{{ w.file_type.toUpperCase() }}</span>
            </button>
            <button class="download-icon btn btn-ghost btn-sm btn-square" :disabled="downloading.includes(w.id)" :title="t('reader.downloadFile', { name: w.file_name })" :aria-label="t('reader.downloadUser', { name: w.user_nickname || w.user_account })" @click="download(w)">
              <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M12 3v12m-5-5 5 5 5-5"/>
              </svg>
            </button>
          </div>
          <p v-if="!filteredItems.length" class="no-results">{{ t('reader.noResults') }}</p>
        </div>
        <p v-if="downloadError" class="download-error" role="alert">{{ downloadError }}</p>
      </aside>

      <section class="preview-panel" :aria-label="t('reader.preview')">
        <template v-if="selected">
          <PdfPreview v-if="selected.file_type.toLowerCase() === 'pdf'" :key="`${gameId}-${selected.id}-${previewVersion}`" :game-id="gameId" :writeup-id="selected.id" :file-name="selected.file_name" :owner="selected.team_name || selected.user_account" @retry="previewVersion++" />
          <DocxPreview v-else-if="selected.file_type.toLowerCase() === 'docx'" :key="`${gameId}-${selected.id}-${previewVersion}`" :game-id="gameId" :writeup-id="selected.id" :file-name="selected.file_name" :owner="selected.team_name || selected.user_account" @retry="previewVersion++" />
          <div v-else class="reader-state">
            <p>{{ t('reader.unsupported', { type: selected.file_type.toUpperCase() }) }}</p>
            <button class="text-button" :disabled="downloading.includes(selected.id)" @click="download(selected)">{{ t('reader.downloadRead') }}</button>
          </div>
        </template>
        <div v-else class="reader-state">{{ t('reader.deleted') }}</div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.reader-view { height: calc(100dvh - var(--nav-height)); min-height: 400px; display: flex; flex-direction: column; }
.reader-header { display: flex; align-items: center; gap: 24px; padding: 4px 16px; border-bottom: 1px solid var(--border); background: var(--surface); backdrop-filter: blur(8px); }
.back-link { display: inline-flex; align-items: center; gap: 4px; flex-shrink: 0; font-size: 14px; color: var(--text-secondary); }
.back-link:hover { color: var(--text-primary); }
.reader-title { min-width: 0; }
.reader-title h1 { font-size: 16px; line-height: 20px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.reader-title > span { display: block; font-size: 12px; line-height: 16px; color: var(--text-secondary); }
.reader-layout { flex: 1; min-height: 0; display: grid; grid-template-columns: 320px minmax(0, 1fr); }
.submission-sidebar { min-height: 0; display: flex; flex-direction: column; background: var(--surface); border-right: 1px solid var(--border); }
.sidebar-header { padding: 16px; border-bottom: 1px solid var(--border); }
.sidebar-header h2 { display: flex; align-items: center; justify-content: space-between; font-size: 14px; margin-bottom: 12px; }
.sidebar-header h2 span { color: var(--text-secondary); font-weight: 400; }
.search-field { position: relative; width: 100%; }
.search-field > svg { position: absolute; top: 11px; left: 10px; color: var(--text-tertiary); pointer-events: none; }
.sidebar-header input { width: 100%; padding: 8px 10px 8px 34px; border: 1px solid transparent; border-radius: var(--radius-sm); background: var(--overlay); color: var(--text-primary); font-size: 12px; }
.sidebar-header input:focus { outline: 2px solid var(--border-focus); outline-offset: 2px; }
.submission-list { flex: 1; min-height: 0; overflow-y: auto; padding: 10px; }
.writeup-card { display: flex; align-items: center; margin-bottom: 8px; border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--overlay); overflow: hidden; }
.writeup-card:hover { border-color: var(--border-strong); }
.writeup-card.active { border-color: var(--accent); background: var(--accent-muted); }
.submission-select { display: flex; flex: 1; min-width: 0; flex-direction: column; align-items: flex-start; gap: 3px; padding: 14px 4px 14px 14px; color: var(--text-primary); text-align: left; }
.submission-select > span { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.submitter { font-size: 14px; font-weight: 600; }
.account, .submitted-at { font-size: 11px; color: var(--text-secondary); }
.team { font-size: 13px; margin-top: 4px; }
.submission-select .file-type { margin-top: 5px; padding: 1px 6px; border-radius: 4px; color: var(--accent); background: var(--accent-muted); font-size: 10px; font-weight: 600; }
.download-icon { display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; width: 34px; height: 34px; margin: 8px; border-radius: var(--radius-sm); color: var(--text-secondary); }
.download-icon:hover { color: var(--text-primary); }
button:disabled { opacity: 0.5; cursor: wait; }
button:focus-visible, a:focus-visible { outline: 2px solid var(--border-focus); outline-offset: -2px; }
.preview-panel { min-width: 0; min-height: 0; display: flex; flex-direction: column; background: var(--bg-sunken); }
.reader-state { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 32px; text-align: center; font-size: 14px; color: var(--text-secondary); }
.text-button { padding: 8px 14px; border-radius: var(--radius-sm); color: var(--text-primary); background: var(--overlay); }
.download-error { padding: 12px 16px; font-size: 12px; color: var(--danger); border-top: 1px solid var(--border); }
.no-results { padding: 24px 8px; text-align: center; color: var(--text-secondary); font-size: 13px; }
@media (max-width: 800px) {
  .reader-layout { grid-template-columns: 240px minmax(0, 1fr); }
  .reader-header { padding: 4px 16px; gap: 16px; }
}
@media (max-width: 580px) {
  .reader-layout { grid-template-columns: minmax(0, 1fr); grid-template-rows: 220px minmax(0, 1fr); }
  .submission-sidebar { border-right: 0; border-bottom: 1px solid var(--border); }
  .sidebar-header { display: flex; align-items: center; gap: 12px; padding: 10px; }
  .sidebar-header h2 { margin: 0; gap: 8px; flex-shrink: 0; }
  .submission-list { display: flex; gap: 8px; overflow: auto; }
  .writeup-card { flex: 0 0 260px; margin-bottom: 0; }
}
</style>
