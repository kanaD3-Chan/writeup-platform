import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { getPinnedGame } from '@/api/game'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/',          name: 'home',   component: () => import('@/views/HomeView.vue') },
    { path: '/login',     name: 'login',  component: () => import('@/views/LoginView.vue') },
    { path: '/submit/:gameId', name: 'submit', component: () => import('@/views/SubmitView.vue'), meta: { requiresAuth: true } },
    { path: '/my',        name: 'my',     component: () => import('@/views/MyView.vue'),    meta: { requiresAuth: true } },
    { path: '/my/:gameId/read', name: 'my-reader', component: () => import('@/views/MyReaderView.vue'), meta: { requiresAuth: true } },
    { path: '/admin/:gameId', name: 'admin', component: () => import('@/views/AdminView.vue'), meta: { requiresAuth: true } },
    { path: '/admin/:gameId/read', name: 'admin-reader', component: () => import('@/views/AdminReaderView.vue'), meta: { requiresAuth: true } },
  ],
})

// Only the very first navigation is redirected to the pinned game, so visitors
// can still reach the game list in the same session.
let firstNavigation = true

router.beforeEach(async (to) => {
  const isFirstNavigation = firstNavigation
  firstNavigation = false

  const auth = useAuthStore()
  if (to.meta.requiresAuth && !auth.isLoggedIn) {
    return { name: 'login', query: { redirect: to.fullPath } }
  }

  if (!isFirstNavigation || to.name !== 'home' || to.query.type) return

  try {
    const { game } = await getPinnedGame()
    if (!game) return
    // Admins of the pinned game manage it; everyone else submits to it.
    const name = auth.isLoggedIn && auth.isGameAdmin(game.admins ?? []) ? 'admin' : 'submit'
    return { name, params: { gameId: game.id } }
  } catch {
    // Pinning is a shortcut; fall back to the game list if it cannot be loaded.
    return
  }
})

export default router
