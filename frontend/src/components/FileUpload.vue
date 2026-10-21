<script setup lang="ts">
import { t } from '@/i18n'
import { computed, ref, watch } from 'vue'
import UiIcon from './UiIcon.vue'
import { DEFAULT_FORMATS, type SubmissionFormat } from '@/api/game'

const props = withDefaults(defineProps<{ formats?: SubmissionFormat[] }>(), {
  formats: () => DEFAULT_FORMATS,
})

const dragging = ref(false)
const selectedFile = defineModel<File | null>({ default: null })
const fileInput = ref<HTMLInputElement | null>(null)
const error = ref('')

watch(selectedFile, (file) => {
  if (!file && fileInput.value) fileInput.value.value = ''
})

const MAX_SIZE = 50 * 1024 * 1024

const accept = computed(() => props.formats.map(format => `.${format}`).join(','))
const formatLabel = computed(() => props.formats.map(format => format.toUpperCase()).join(' / '))

function extensionOf(file: File): string {
  return (file.name.split('.').pop() ?? '').toLowerCase()
}

function validate(file: File): string {
  if (!props.formats.includes(extensionOf(file) as SubmissionFormat)) {
    return t('upload.unsupported', { formats: formatLabel.value })
  }
  if (file.size > MAX_SIZE) return t('upload.tooLarge')
  return ''
}

// A game's accepted formats can change while this component stays mounted.
watch(() => props.formats, (formats) => {
  if (selectedFile.value && !formats.includes(extensionOf(selectedFile.value) as SubmissionFormat)) {
    selectedFile.value = null
    error.value = ''
  }
})

function pick(file: File) {
  const msg = validate(file)
  if (msg) { error.value = msg; return }
  error.value = ''
  selectedFile.value = file
}

function onDrop(e: DragEvent) {
  dragging.value = false
  const file = e.dataTransfer?.files[0]
  if (file) pick(file)
}

function onInput(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) pick(file)
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1024 / 1024).toFixed(1) + ' MB'
}

function clear() {
  selectedFile.value = null
  error.value = ''
}
</script>

<template>
  <div class="file-upload">
    <label
      class="drop-zone"
      :class="{ dragging, 'has-file': selectedFile }"
      @dragover.prevent="dragging = true"
      @dragleave="dragging = false"
      @drop.prevent="onDrop"
    >
      <input
        ref="fileInput"
        type="file"
        :accept="accept"
        class="hidden-input" :aria-label="t('upload.choose')"
        @change="onInput"
      />

      <template v-if="!selectedFile">
        <div class="upload-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="17 8 12 3 7 8"/>
            <line x1="12" y1="3" x2="12" y2="15"/>
          </svg>
        </div>
        <p class="upload-hint">{{ t('upload.drop') }} <span class="link">{{ t('upload.browse') }}</span></p>
        <p class="upload-types">{{ t('upload.formats', { formats: formatLabel }) }}</p>
      </template>

      <template v-else>
        <div class="file-info">
          <div class="file-icon"><UiIcon name="file" :size="28" /></div>
          <div class="file-details">
            <span class="file-name">{{ selectedFile.name }}</span>
            <span class="file-size">{{ formatSize(selectedFile.size) }}</span>
          </div>
          <button class="clear-btn btn btn-ghost btn-sm btn-square" type="button" @click.prevent="clear" :title="t('upload.remove')" :aria-label="t('upload.removeLabel')">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </template>
    </label>

    <p v-if="error" class="status-message error" role="alert">{{ error }}</p>
  </div>
</template>

<style scoped>
.file-upload { width: 100%; }
.drop-zone { position: relative; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; min-height: 200px; padding: 32px 20px; border: 1px dashed var(--border-strong); border-radius: var(--radius-md); background: var(--overlay); cursor: pointer; text-align: center; transition: border-color var(--duration-fast), background var(--duration-fast); }
.drop-zone:hover, .drop-zone.dragging { border-color: var(--accent); background: var(--accent-muted); }
.drop-zone:focus-within { outline: 2px solid var(--border-focus); outline-offset: 3px; }
.drop-zone.has-file { border-style: solid; min-height: 100px; padding: 20px; text-align: left; }
.hidden-input { position: absolute; width: 1px; height: 1px; opacity: 0; overflow: hidden; }
.upload-icon { color: var(--text-secondary); }
.upload-hint { font-size: 13px; color: var(--text-secondary); }
.link { color: var(--accent); }
.upload-types { font-size: 11px; color: var(--text-tertiary); }
.file-info { display: flex; align-items: center; gap: 12px; width: 100%; }
.file-icon { display: flex; color: var(--text-secondary); }
.file-details { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
.file-name { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.file-size { font-size: 11px; color: var(--text-tertiary); }
.clear-btn { color: var(--text-secondary); }
.clear-btn:hover { color: var(--danger); }
.status-message { margin-top: 12px; }
</style>
