<script setup lang="ts">
import { t, dateLocale, translateError } from '@/i18n'
import { ref, onMounted, onBeforeUnmount, computed } from 'vue'
import UiIcon from '@/components/UiIcon.vue'
import { useRoute, useRouter } from 'vue-router'
import { getGame, pinGame, unpinGame, updateGameFormats, SUBMISSION_FORMATS, DEFAULT_FORMATS, type GameItem, type SubmissionFormat } from '@/api/game'
import { getGameWriteups, downloadGameWriteups, downloadUserWriteup, deleteUserWriteup, type WriteupRecord } from '@/api/writeup'

const route = useRoute()
const router = useRouter()
const gameId = Number(route.params.gameId)

const game = ref<GameItem | null>(null)
const items = ref<WriteupRecord[]>([])
const total = ref(0)
const page = ref(1)
const loading = ref(false)
const error = ref('')
const PAGE_SIZE = 50

// A pinned game is the landing page, so there is no list to go back to.
const showBack = computed(() => (game.value ? !game.value.pinned : !!error.value))

function formatDate(s: string): string {
  return new Date(s).toLocaleString(dateLocale)
}

function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1024 / 1024).toFixed(1) + ' MB'
}

async function load() {
  loading.value = true
  error.value = ''
  try {
    const [gamesRes, writeupRes] = await Promise.all([
      getGame(gameId),
      getGameWriteups(gameId, { page: page.value, page_size: PAGE_SIZE }),
    ])
    game.value = gamesRes
    items.value = writeupRes.items
    total.value = writeupRes.total
    formats.value = gamesRes.allowed_formats ?? [...DEFAULT_FORMATS]
  } catch (e: any) {
    error.value = translateError(e.response?.data?.error, 'common.loadFailed')
  } finally {
    loading.value = false
  }
}

const deleting = ref<number | null>(null)
const pinUpdating = ref(false)

async function togglePin() {
  if (!game.value) return
  pinUpdating.value = true
  const next = !game.value.pinned
  try {
    if (next) await pinGame(gameId)
    else await unpinGame(gameId)
    game.value = { ...game.value, pinned: next }
  } catch (e: any) {
    alert(translateError(e.response?.data?.error, 'admin.pinFailed'))
  } finally {
    pinUpdating.value = false
  }
}

const formats = ref<SubmissionFormat[]>([...DEFAULT_FORMATS])
const formatSaving = ref(false)
const formatError = ref('')
const formatSaved = ref(false)
let savedTimer: number | undefined

function formatEnabled(format: SubmissionFormat): boolean {
  return formats.value.includes(format)
}

function sortFormats(list: SubmissionFormat[]): SubmissionFormat[] {
  return [...list].sort((a, b) => SUBMISSION_FORMATS.indexOf(a) - SUBMISSION_FORMATS.indexOf(b))
}

async function toggleFormat(format: SubmissionFormat) {
  if (formatSaving.value) return
  formatError.value = ''
  formatSaved.value = false
  const next = formatEnabled(format)
    ? formats.value.filter(f => f !== format)
    : sortFormats([...formats.value, format])
  if (!next.length) {
    formatError.value = t('admin.formatsAtLeastOne')
    return
  }
  const previous = formats.value
  formats.value = next
  formatSaving.value = true
  try {
    const result = await updateGameFormats(gameId, next)
    formats.value = result.allowed_formats
    if (game.value) game.value = { ...game.value, allowed_formats: result.allowed_formats }
    formatSaved.value = true
    window.clearTimeout(savedTimer)
    savedTimer = window.setTimeout(() => { formatSaved.value = false }, 2000)
  } catch (e: any) {
    formats.value = previous
    formatError.value = translateError(e.response?.data?.error, 'admin.formatsSaveFailed')
  } finally {
    formatSaving.value = false
  }
}

onBeforeUnmount(() => { window.clearTimeout(savedTimer) })

async function handleDelete(w: WriteupRecord) {
  if (!confirm(t('admin.deleteConfirm', { name: w.team_name ?? w.user_account }))) return
  deleting.value = w.id
  try {
    await deleteUserWriteup(gameId, w.id)
    items.value = items.value.filter(i => i.id !== w.id)
    total.value--
  } catch (e: any) {
    alert(translateError(e.response?.data?.error, 'admin.deleteFailed'))
  } finally {
    deleting.value = null
  }
}

onMounted(load)
</script>

<template>
  <div class="admin-view page-shell">
    <div class="admin-container">
      <button v-if="showBack" class="back-btn" @click="router.push('/')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 18 9 12 15 6"/>
        </svg>
        {{ t('common.back') }}
      </button>

      <div v-if="loading" class="state-msg">
        <div class="spinner"></div> {{ t('common.loading') }}
      </div>

      <div v-else-if="error" class="state-msg error">{{ error }}</div>

      <template v-else>
        <div class="page-heading">
          <div>
            <h1 class="heading-title"><UiIcon name="settings" :size="26" />{{ game?.name ?? t('admin.title') }}<span v-if="game?.pinned" class="tag tag-primary"><UiIcon name="pin" :size="12" />{{ t('admin.pinned') }}</span></h1>
            <p class="heading-sub">{{ t('admin.count', { count: total }) }}</p>
          </div>
          <div class="header-actions">
            <button class="btn" :disabled="pinUpdating" :aria-pressed="game?.pinned ?? false" :title="t('admin.pinHint')" @click="togglePin">
              <UiIcon name="pin" :size="16" />
              {{ game?.pinned ? t('admin.unpin') : t('admin.pin') }}
            </button>
            <button class="btn" @click="router.push({ name: 'admin-reader', params: { gameId } })">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <path d="M12 7v14M3 3h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5v16h-5a4 4 0 0 0-4 2 4 4 0 0 0-4-2H3z"/>
              </svg>
              {{ t('admin.readAll') }}
            </button>
            <button class="btn btn-primary" @click="downloadGameWriteups(gameId)">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
              {{ t('admin.downloadAll') }}
            </button>
          </div>
        </div>

        <section class="panel format-panel" :aria-label="t('admin.formatsTitle')">
          <div class="format-heading">
            <h2><UiIcon name="settings" :size="16" />{{ t('admin.formatsTitle') }}</h2>
            <span v-if="formatSaving" class="format-status" role="status">{{ t('admin.formatsSaving') }}</span>
            <span v-else-if="formatSaved" class="format-status saved" role="status">{{ t('admin.formatsSaved') }}</span>
          </div>
          <p class="format-hint">{{ t('admin.formatsHint') }}</p>
          <div class="format-options">
            <label v-for="format in SUBMISSION_FORMATS" :key="format" class="format-option" :class="{ active: formatEnabled(format), busy: formatSaving }">
              <input type="checkbox" :checked="formatEnabled(format)" :disabled="formatSaving" @change="toggleFormat(format)" />
              <span class="format-name">{{ format.toUpperCase() }}</span>
            </label>
          </div>
          <p v-if="formatError" class="format-error" role="alert">{{ formatError }}</p>
        </section>

        <div v-if="items.length === 0" class="state-msg empty">{{ t('admin.empty') }}</div>

        <div v-else class="table-wrap panel">
          <table class="table">
            <thead>
              <tr>
                <th>{{ t('common.team') }}</th>
                <th>{{ t('common.submitter') }}</th>
                <th>{{ t('common.filename') }}</th>
                <th>{{ t('common.size') }}</th>
                <th>{{ t('common.type') }}</th>
                <th>{{ t('common.submittedAt') }}</th>
                <th>{{ t('common.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="w in items" :key="w.id">
                <td class="team-name">{{ w.team_name ?? '—' }}</td>
                <td class="submitter">
                  <span class="account">{{ w.user_account }}</span>
                  <span class="nickname">{{ w.user_nickname }}</span>
                </td>
                <td class="file-name">{{ w.file_name }}</td>
                <td>{{ formatSize(w.file_size) }}</td>
                <td><span class="tag tag-primary">{{ w.file_type.toUpperCase() }}</span></td>
                <td class="date">{{ formatDate(w.updated_at) }}</td>
                <td>
                  <div class="action-btns">
                    <button class="btn btn-sm" @click="router.push({ name: 'admin-reader', params: { gameId }, query: { writeup: w.id } })">{{ t('common.preview') }}</button>
                    <button class="btn btn-sm" @click="downloadUserWriteup(gameId, w.id)">{{ t('common.download') }}</button>
                    <button class="btn btn-sm btn-danger" :disabled="deleting === w.id" @click="handleDelete(w)">{{ t('common.delete') }}</button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="total > PAGE_SIZE" class="pagination">
          <button class="btn btn-sm" :disabled="page === 1" @click="page--; load()">{{ t('common.previous') }}</button>
          <span class="page-info">{{ page }} / {{ Math.ceil(total / PAGE_SIZE) }}</span>
          <button class="btn btn-sm" :disabled="page * PAGE_SIZE >= total" @click="page++; load()">{{ t('common.next') }}</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.admin-container { width: 100%; }
.header-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.format-panel { display: flex; flex-direction: column; gap: 10px; margin-bottom: 20px; padding: 16px 20px; }
.format-heading { display: flex; align-items: center; gap: 12px; }
.format-heading h2 { display: flex; align-items: center; gap: 6px; font-size: 14px; }
.format-status { font-size: 11px; color: var(--text-tertiary); }
.format-status.saved { color: var(--accent); }
.format-hint { font-size: 12px; color: var(--text-secondary); }
.format-options { display: flex; flex-wrap: wrap; gap: 8px; }
.format-option { display: inline-flex; align-items: center; gap: 8px; padding: 8px 14px; border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--overlay); font-size: 13px; cursor: pointer; }
.format-option.active { border-color: var(--accent); background: var(--accent-muted); }
.format-option.busy { opacity: 0.6; cursor: wait; }
.format-option input { accent-color: var(--accent); cursor: inherit; }
.format-error { font-size: 12px; color: var(--danger); }
.table { min-width: 900px; }
.team-name { font-weight: 700; }
.submitter { min-width: 110px; }
.submitter .account, .submitter .nickname { display: block; }
.submitter .account { font-size: 13px; }
.submitter .nickname { font-size: 11px; color: var(--text-tertiary); margin-top: 2px; }
</style>
