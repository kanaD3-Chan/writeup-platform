<script setup lang="ts">
import { t, dateLocale, translateError } from '@/i18n'
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { getGame, DEFAULT_FORMATS, type GameItem } from '@/api/game'
import { submitWriteup, getMyWriteupForGame, type WriteupRecord } from '@/api/writeup'
import FileUpload from '@/components/FileUpload.vue'
import UiIcon from '@/components/UiIcon.vue'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const gameId = Number(route.params.gameId)

const game = ref<GameItem | null>(null)
const existing = ref<WriteupRecord | null>(null)
const selectedFile = ref<File | null>(null)
const loading = ref(false)
const submitting = ref(false)
const error = ref('')
const success = ref(false)
const lockReason = ref('')  // non-empty = locked

// A pinned game is the landing page, so there is no list to go back to.
const showBack = computed(() => (game.value ? !game.value.pinned : !!error.value))
const formats = computed(() => game.value?.allowed_formats ?? DEFAULT_FORMATS)
const formatLabel = computed(() => formats.value.map(format => format.toUpperCase()).join(' / '))

function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1024 / 1024).toFixed(1) + ' MB'
}

function formatDate(s: string): string {
  return new Date(s).toLocaleString(dateLocale)
}

function computeLockReason(g: GameItem): string {
  const isAdmin = auth.isGameAdmin(g.admins ?? [])
  if (isAdmin) return ''
  if (g.host_type === 1) {
    const now = Date.now()
    if (g.start_at && new Date(g.start_at).getTime() > now) return 'not_started'
    if (g.archive_at && new Date(g.archive_at).getTime() <= now) return 'ended'
  }
  return ''
}

function lockMessage(reason: string): string {
  if (reason === 'not_started') return t('submit.notStarted')
  if (reason === 'ended') return t('submit.ended')
  return ''
}

async function load() {
  loading.value = true
  try {
    const [gamesRes, existingRes] = await Promise.all([
      getGame(gameId),
      getMyWriteupForGame(gameId),
    ])
    game.value = gamesRes
    existing.value = existingRes
    if (game.value) lockReason.value = computeLockReason(game.value)
  } catch {
    error.value = t('common.loadFailed')
  } finally {
    loading.value = false
  }
}

async function handleSubmit() {
  if (!selectedFile.value) return
  submitting.value = true
  error.value = ''
  success.value = false
  try {
    existing.value = await submitWriteup(gameId, selectedFile.value)
    success.value = true
    selectedFile.value = null
  } catch (e: any) {
    error.value = translateError(e.response?.data?.error, 'submit.failed')
  } finally {
    submitting.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="submit-view page-shell">
    <div class="submit-container">
      <button v-if="showBack" class="back-btn" @click="router.push('/')"><UiIcon name="arrowLeft" :size="16" />{{ t('common.back') }}</button>
      <div v-if="loading" class="state-msg" role="status"><div class="spinner"></div>{{ t('common.loading') }}</div>
      <template v-else-if="game">
        <div class="page-heading">
          <div>
            <h1 class="heading-title"><UiIcon name="upload" :size="26" />{{ game.name }}</h1>
            <p class="heading-sub">{{ t('submit.description') }}</p>
          </div>
          <span class="tag" :class="game.host_type === 0 ? 'tag-success' : 'tag-primary'">{{ game.host_type === 0 ? t('common.training') : t('common.competition') }}</span>
        </div>
        <section class="panel">
          <div class="panel-body submission-body">
            <div class="section-heading"><UiIcon name="file" /><h2>{{ t('submit.report') }}</h2><span class="file-formats">{{ t('submit.formats', { formats: formatLabel }) }}</span></div>
            <div v-if="existing" class="existing-card">
              <div class="existing-header"><span class="tag tag-success"><UiIcon name="check" :size="14" />{{ t('submit.submitted') }}</span><span class="existing-date">{{ formatDate(existing.updated_at) }}</span></div>
              <div class="existing-info"><UiIcon name="file" :size="18" /><span class="existing-name">{{ existing.file_name }}</span></div>
              <p class="existing-hint">{{ formatSize(existing.file_size) }} · {{ existing.file_type.toUpperCase() }} · {{ t('submit.overwrite') }}</p>
            </div>
            <div v-if="lockReason" class="status-message warning">{{ lockMessage(lockReason) }}</div>
            <template v-else>
              <FileUpload v-model="selectedFile" :formats="formats" />
              <div v-if="success" class="status-message success" role="status"><UiIcon name="check" :size="18" />{{ t('submit.success') }}</div>
              <div v-if="error" class="status-message error" role="alert">{{ error }}</div>
              <div class="submit-footer"><span>{{ t('submit.confirmFile') }}</span><button class="btn btn-primary" :disabled="!selectedFile || submitting" @click="handleSubmit"><span v-if="submitting" class="btn-spinner"></span><UiIcon v-else name="upload" :size="18" />{{ submitting ? t('submit.submitting') : (existing ? t('submit.update') : t('game.submit')) }}</button></div>
            </template>
          </div>
        </section>
      </template>
      <div v-else class="state-msg error" role="alert">{{ error || t('game.notFound') }}</div>
    </div>
  </div>
</template>

<style scoped>
.submit-container { max-width: 800px; margin: 0 auto; }
.submission-body { display: flex; flex-direction: column; gap: 20px; }
.section-heading { display: flex; align-items: center; gap: 8px; padding-bottom: 16px; border-bottom: 1px solid var(--border); }
.section-heading h2 { font-size: 14px; }
.file-formats { margin-left: auto; color: var(--text-tertiary); font-size: 11px; }
.existing-card { padding: 16px; border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--overlay); }
.existing-header { display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }
.existing-date, .existing-hint { color: var(--text-tertiary); font-size: 11px; }
.existing-info { display: flex; align-items: center; gap: 8px; }
.existing-name { min-width: 0; overflow-wrap: anywhere; font-size: 13px; }
.existing-hint { margin-top: 6px; }
.submit-footer { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding-top: 4px; }
.submit-footer > span { font-size: 11px; color: var(--text-tertiary); }
@media (max-width: 580px) {
  .section-heading { flex-wrap: wrap; }
  .file-formats { width: 100%; margin: 0; }
  .submit-footer { flex-direction: column; align-items: stretch; }
}
</style>
