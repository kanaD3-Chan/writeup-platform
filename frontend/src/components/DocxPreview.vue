<script setup lang="ts">
import { t } from '@/i18n'
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { getMyWriteupFile, getUserWriteupFile } from '@/api/writeup'

const props = withDefaults(defineProps<{
  gameId: number
  writeupId: number
  fileName: string
  owner: string
  source?: 'admin' | 'mine'
}>(), { source: 'admin' })
defineEmits<{ retry: [] }>()

const container = ref<HTMLDivElement>()
const pageContainer = ref<HTMLDivElement>()
const loading = ref(true)
const error = ref('')
const pageNumber = ref(1)
const pageCount = ref(0)
const zoomLevel = ref(1)
const fitMode = ref(true)
const supportsZoom = typeof CSS !== 'undefined' && CSS.supports('zoom', '1.2')

const MIN_ZOOM = 0.4
const MAX_ZOOM = 2.5
const PAGE_PADDING = 16  // must match `.docx-container` padding

const controller = new AbortController()
let pages: HTMLElement[] = []
let resizeObserver: ResizeObserver | undefined
let scrollFrame = 0

function clampZoom(value: number): number {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, value))
}

function applyFit() {
  const box = container.value
  const first = pages[0]
  if (!supportsZoom || !box || !first) return
  const baseWidth = first.getBoundingClientRect().width / zoomLevel.value
  if (!baseWidth) return
  const available = box.clientWidth - PAGE_PADDING * 2
  zoomLevel.value = clampZoom(available / baseWidth)
}

function currentPage(): number {
  const box = container.value
  if (!box || !pages.length) return 1
  const threshold = box.getBoundingClientRect().top + 40
  let current = 1
  for (let index = 0; index < pages.length; index++) {
    if (pages[index]!.getBoundingClientRect().top <= threshold) current = index + 1
    else break
  }
  return current
}

function schedulePageUpdate() {
  if (scrollFrame) return
  scrollFrame = requestAnimationFrame(() => {
    scrollFrame = 0
    pageNumber.value = currentPage()
  })
}

function scrollToPage(page: number) {
  const box = container.value
  const element = pages[page - 1]
  if (!box || !element) return
  const offset = element.getBoundingClientRect().top - box.getBoundingClientRect().top
  box.scrollTop = Math.max(0, box.scrollTop + offset)
  pageNumber.value = page
}

function goToPage(page: number) {
  if (!Number.isInteger(page) || page < 1 || page > pageCount.value) return
  scrollToPage(page)
}

function changePage(event: Event) {
  const input = event.target as HTMLInputElement
  goToPage(Number(input.value))
  input.value = String(pageNumber.value)
}

function zoom(factor: number) {
  fitMode.value = false
  const page = pageNumber.value
  zoomLevel.value = clampZoom(zoomLevel.value * factor)
  void nextTick(() => scrollToPage(page))
}

function fitWidth() {
  fitMode.value = true
  const page = pageNumber.value
  applyFit()
  void nextTick(() => scrollToPage(page))
}

onMounted(async () => {
  let phase: 'load' | 'render' = 'load'
  try {
    const data = await (props.source === 'mine'
      ? getMyWriteupFile(props.gameId, controller.signal)
      : getUserWriteupFile(props.gameId, props.writeupId, controller.signal))
    if (controller.signal.aborted) return

    phase = 'render'
    const { renderAsync } = await import('docx-preview')
    const body = pageContainer.value
    if (!body) return
    body.replaceChildren()
    await renderAsync(data, body, body, {
      className: 'docx',
      inWrapper: true,
      breakPages: true,
      ignoreWidth: !supportsZoom,
      ignoreHeight: false,
      ignoreLastRenderedPageBreak: true,
      useBase64URL: true,
      renderHeaders: true,
      renderFooters: true,
      renderFootnotes: true,
      renderEndnotes: true,
    })
    if (controller.signal.aborted) return

    pages = Array.from(body.querySelectorAll<HTMLElement>('.docx-wrapper > section'))
    pageCount.value = pages.length
    pageNumber.value = 1
    zoomLevel.value = 1
    await nextTick()
    applyFit()
    loading.value = false

    resizeObserver = new ResizeObserver(() => {
      if (fitMode.value) applyFit()
    })
    if (container.value) resizeObserver.observe(container.value)
  } catch (e: any) {
    if (controller.signal.aborted) return
    const status = e.response?.status
    error.value = status === 403 ? t('pdf.forbidden')
      : status === 401 ? t('pdf.expired')
      : status === 404 ? t('pdf.notFound')
      : phase === 'render' ? t('docx.renderFailed')
      : t('docx.loadFailed')
    loading.value = false
  }
})

onBeforeUnmount(() => {
  controller.abort()
  resizeObserver?.disconnect()
  if (scrollFrame) cancelAnimationFrame(scrollFrame)
  pageContainer.value?.replaceChildren()
})
</script>

<template>
  <div class="docx-preview">
    <div class="docx-toolbar" :aria-label="t('docx.toolbar')">
      <span class="document-name" :title="fileName">{{ fileName }}</span>
      <div class="toolbar-controls">
        <div class="toolbar-group">
          <button :title="t('common.previous')" :aria-label="t('common.previous')" :disabled="loading || !!error || pageNumber <= 1" @click="goToPage(pageNumber - 1)">‹</button>
          <input :value="pageNumber" type="number" min="1" :max="pageCount || 1" :disabled="loading || !!error" :aria-label="t('pdf.page')" @change="changePage" />
          <span>/ {{ pageCount }}</span>
          <button :title="t('common.next')" :aria-label="t('common.next')" :disabled="loading || !!error || pageNumber >= pageCount" @click="goToPage(pageNumber + 1)">›</button>
        </div>
        <div v-if="supportsZoom" class="toolbar-group">
          <button :title="t('pdf.zoomOut')" :aria-label="t('pdf.zoomOut')" :disabled="loading || !!error || zoomLevel <= MIN_ZOOM" @click="zoom(1 / 1.2)">−</button>
          <span class="zoom-value">{{ Math.round(zoomLevel * 100) }}%</span>
          <button :title="t('pdf.zoomIn')" :aria-label="t('pdf.zoomIn')" :disabled="loading || !!error || zoomLevel >= MAX_ZOOM" @click="zoom(1.2)">+</button>
          <button class="fit-button" :disabled="loading || !!error" @click="fitWidth">{{ t('pdf.fitWidth') }}</button>
        </div>
      </div>
      <span class="document-owner" :title="owner">{{ owner }}</span>
    </div>
    <div class="docx-body">
      <div ref="container" class="docx-container" :aria-busy="loading" @scroll.passive="schedulePageUpdate">
        <div ref="pageContainer" class="docx-pages" :style="{ zoom: zoomLevel.toFixed(3) }"></div>
      </div>
      <div v-if="loading || error" class="docx-state" :role="error ? 'alert' : 'status'">
        <span v-if="loading && !error" class="spinner" aria-hidden="true"></span>
        <p>{{ error || t('docx.loading') }}</p>
        <button v-if="error" class="retry-button" @click="$emit('retry')">{{ t('common.retry') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.docx-preview { display: flex; flex: 1; flex-direction: column; min-height: 0; container-type: inline-size; }
.docx-toolbar { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); align-items: center; gap: 8px 16px; padding: 8px 20px; background: var(--surface); border-bottom: 1px solid var(--border); font-size: 12px; }
.document-name, .document-owner { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
.document-name { font-weight: 500; }
.document-owner { text-align: right; color: var(--text-secondary); }
.toolbar-controls { display: flex; align-items: center; justify-content: center; gap: 24px; }
.toolbar-group { display: flex; align-items: center; gap: 6px; }
.docx-toolbar button { height: 30px; min-width: 30px; padding: 0 8px; border-radius: var(--radius-sm); color: var(--text-primary); font-size: 20px; }
.docx-toolbar button:hover:not(:disabled) { background: var(--overlay); }
.docx-toolbar button:disabled { opacity: 0.35; cursor: not-allowed; }
.docx-toolbar .fit-button { font-size: 12px; }
.docx-toolbar input { width: 52px; height: 28px; border: 1px solid transparent; border-radius: var(--radius-sm); background: var(--overlay); color: var(--text-primary); text-align: center; font-size: 12px; }
.docx-toolbar input::-webkit-inner-spin-button { appearance: none; }
.zoom-value { min-width: 40px; text-align: center; }
.docx-body { position: relative; flex: 1; min-height: 0; }
.docx-container { position: absolute; inset: 0; overflow: auto; overscroll-behavior: contain; padding: 16px; }
.docx-pages { width: max-content; }
:deep(.docx-wrapper) { background: transparent; padding: 0; }
:deep(.docx-wrapper > section.docx) { margin-bottom: 16px; box-shadow: 0 2px 10px rgba(0, 0, 0, 0.22); }
:deep(.docx-wrapper > section.docx:last-child) { margin-bottom: 0; }
.docx-state { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; padding: 24px; background: var(--bg-sunken); color: var(--text-secondary); text-align: center; font-size: 14px; }
.retry-button { padding: 8px 16px; color: var(--text-primary); background: var(--overlay); border-radius: var(--radius-sm); }
.spinner { width: 24px; height: 24px; border: 2px solid var(--border); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.7s linear infinite; }
@container (max-width: 600px) {
  .docx-toolbar { grid-template-columns: minmax(0, 1fr) auto; padding: 8px 12px; }
  .toolbar-controls { grid-column: 1 / -1; grid-row: 2; gap: 12px; }
  .document-owner { grid-column: 2; grid-row: 1; }
}
@keyframes spin { to { transform: rotate(360deg); } }
</style>
