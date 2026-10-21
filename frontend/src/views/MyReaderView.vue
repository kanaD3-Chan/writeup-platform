<script setup lang="ts">
import { t, translateError } from '@/i18n'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { downloadMyWriteup, getMyWriteupForGame, type WriteupRecord } from '@/api/writeup'
import PdfPreview from '@/components/PdfPreview.vue'
import DocxPreview from '@/components/DocxPreview.vue'
import UiIcon from '@/components/UiIcon.vue'

const route = useRoute()
const gameId = computed(() => Number(route.params.gameId))
const writeup = ref<WriteupRecord | null>(null)
const loading = ref(true)
const error = ref('')
const downloadError = ref('')
const downloading = ref(false)
const previewVersion = ref(0)
let loadVersion = 0

async function load() {
  const version = ++loadVersion
  loading.value = true
  error.value = ''
  downloadError.value = ''
  writeup.value = null
  try {
    const result = await getMyWriteupForGame(gameId.value)
    if (version === loadVersion) writeup.value = result
  } catch (e: any) {
    if (version === loadVersion) error.value = translateError(e.response?.data?.error, 'reader.loadInfoFailed')
  } finally {
    if (version === loadVersion) loading.value = false
  }
}

async function download() {
  downloading.value = true
  downloadError.value = ''
  try {
    await downloadMyWriteup(gameId.value)
  } catch {
    downloadError.value = t('common.downloadFailed')
  } finally {
    downloading.value = false
  }
}

watch(gameId, load, { immediate: true })
onBeforeUnmount(() => { loadVersion++ })
</script>

<template>
  <div class="my-reader">
    <header class="reader-header">
      <RouterLink to="/my" class="btn btn-ghost btn-sm back-link"><UiIcon name="arrowLeft" :size="16" />{{ t('reader.backMine') }}</RouterLink>
      <div class="reader-title">
        <h1 :title="writeup?.game_name">{{ writeup?.game_name || t('reader.preview') }}</h1>
        <span>{{ t('common.mine') }}</span>
      </div>
      <button v-if="writeup" class="btn btn-sm" :disabled="downloading" @click="download"><UiIcon name="download" :size="16" />{{ t('common.download') }}</button>
    </header>
    <p v-if="downloadError" class="download-error" role="alert">{{ downloadError }}</p>
    <div v-if="loading" class="reader-state" role="status"><div class="spinner"></div>{{ t('reader.loadingInfo') }}</div>
    <div v-else-if="error" class="reader-state" role="alert"><p>{{ error }}</p><button class="btn btn-sm" @click="load">{{ t('common.retry') }}</button></div>
    <div v-else-if="!writeup" class="reader-state">{{ t('reader.noSubmission') }}</div>
    <PdfPreview v-else-if="writeup.file_type.toLowerCase() === 'pdf'" :key="`${gameId}-${writeup.id}-${previewVersion}`" source="mine" :game-id="gameId" :writeup-id="writeup.id" :file-name="writeup.file_name" :owner="writeup.team_name || writeup.user_account" @retry="previewVersion++" />
    <DocxPreview v-else-if="writeup.file_type.toLowerCase() === 'docx'" :key="`${gameId}-${writeup.id}-${previewVersion}`" source="mine" :game-id="gameId" :writeup-id="writeup.id" :file-name="writeup.file_name" :owner="writeup.team_name || writeup.user_account" @retry="previewVersion++" />
    <div v-else class="reader-state"><UiIcon name="file" :size="40" /><p>{{ writeup.file_name }}</p><p>{{ t('reader.unsupportedDownload', { type: writeup.file_type.toUpperCase() }) }}</p></div>
  </div>
</template>

<style scoped>
.my-reader { height: calc(100dvh - var(--nav-height)); min-height: 400px; display: flex; flex-direction: column; background: var(--bg-sunken); }
.reader-header { display: flex; align-items: center; gap: 16px; padding: 4px 16px; border-bottom: 1px solid var(--border); background: var(--surface); }
.back-link { color: var(--text-secondary); }
.reader-title { flex: 1; min-width: 0; }
.reader-title h1 { font-size: 16px; line-height: 20px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.reader-title > span { display: block; font-size: 12px; line-height: 16px; color: var(--text-secondary); }
.reader-state { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; padding: 24px; font-size: 14px; color: var(--text-secondary); text-align: center; overflow-wrap: anywhere; }
.download-error { padding: 8px 16px; color: var(--danger); font-size: 12px; }
@media (max-width: 580px) {
  .reader-header { gap: 8px; padding: 4px 8px; }
  .reader-title h1 { font-size: 14px; }
}
</style>
