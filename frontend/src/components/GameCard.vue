<script setup lang="ts">
import { t, dateLocale } from '@/i18n'
import type { GameItem } from '@/api/game'
import { useAuthStore } from '@/stores/auth'
import { useRouter } from 'vue-router'
import UiIcon from './UiIcon.vue'

const props = defineProps<{ game: GameItem }>()
const auth = useAuthStore()
const router = useRouter()

function formatDate(s: string | null): string {
  if (!s) return '—'
  return new Date(s).toLocaleDateString(dateLocale, { year: 'numeric', month: 'short', day: 'numeric' })
}

function handleSubmit() {
  if (!auth.isLoggedIn) {
    router.push({ name: 'login', query: { redirect: `/submit/${props.game.id}` } })
  } else {
    router.push({ name: 'submit', params: { gameId: props.game.id } })
  }
}

function handleAdmin() {
  router.push({ name: 'admin', params: { gameId: props.game.id } })
}
</script>

<template>
  <article class="game-card panel">
    <div class="game-content panel-body">
      <div class="card-header">
        <UiIcon :name="game.host_type === 0 ? 'training' : 'flag'" :size="24" />
        <div class="card-tags">
          <span v-if="game.pinned" class="tag tag-primary"><UiIcon name="pin" :size="12" />{{ t('game.pinned') }}</span>
          <span class="tag" :class="game.can_submit ? 'tag-success' : 'tag-warning'">{{ game.can_submit ? t('game.open') : t('game.archived') }}</span>
        </div>
      </div>
      <h2 class="game-name">{{ game.name }}</h2>
      <p class="game-brief">{{ game.brief || t('game.noBrief') }}</p>
      <div class="game-meta">
        <UiIcon name="clock" :size="15" />
        <span v-if="game.start_at || game.end_at">{{ formatDate(game.start_at) }} — {{ formatDate(game.end_at) }}</span>
        <span v-else>{{ t('game.alwaysOpen') }}</span>
      </div>
    </div>
    <div class="card-actions">
      <span class="card-type">{{ game.host_type === 0 ? t('common.training') : t('common.competition') }}</span>
      <button v-if="game.can_submit" class="btn btn-primary btn-sm" @click="handleSubmit"><UiIcon name="upload" :size="16" />{{ t('game.submit') }}</button>
      <button v-if="auth.isLoggedIn && auth.isGameAdmin(game.admins ?? [])" class="btn btn-sm" @click="handleAdmin"><UiIcon name="settings" :size="16" />{{ t('common.manage') }}</button>
    </div>
  </article>
</template>

<style scoped>
.game-card { display: flex; flex-direction: column; min-width: 0; transition: border-color var(--duration-base); }
.game-card:hover { border-color: var(--border-strong); }
.game-content { display: flex; flex-direction: column; flex: 1; gap: 12px; }
.card-header { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 8px; color: var(--text-secondary); }
.card-tags { display: flex; align-items: center; flex-wrap: wrap; justify-content: flex-end; gap: 6px; }
.game-name { font-size: 18px; line-height: 1.5; overflow-wrap: anywhere; }
.game-brief { color: var(--text-secondary); font-size: 13px; line-height: 1.7; flex: 1; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; min-height: 44px; }
.game-meta { display: flex; align-items: center; gap: 7px; font-size: 11px; color: var(--text-tertiary); margin-top: 12px; }
.card-actions { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; padding: 14px 20px; border-top: 1px solid var(--border); }
.card-type { margin-right: auto; font-size: 11px; color: var(--text-tertiary); }
</style>
