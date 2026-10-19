import api from './index'

export interface GameItem {
  id: number
  name: string
  brief: string
  host_type: number   // 0=training, 1=competition
  start_at: string | null
  end_at: string | null
  archive_at: string | null
  can_submit: boolean
  pinned: boolean
  admins?: number[]
}

export interface GamesResponse {
  items: GameItem[]
  total: number
}

export function getGame(gameId: number): Promise<GameItem> {
  return api.get(`/games/${gameId}`).then(r => r.data)
}

export function getGames(params?: {
  page?: number
  page_size?: number
  host_type?: number
}): Promise<GamesResponse> {
  return api.get('/games', { params }).then(r => r.data)
}

export function getPinnedGame(): Promise<{ game: GameItem | null }> {
  return api.get('/games/pinned').then(r => r.data)
}

export function pinGame(gameId: number): Promise<void> {
  return api.post(`/admin/game/${gameId}/pin`).then(() => undefined)
}

export function unpinGame(gameId: number): Promise<void> {
  return api.delete(`/admin/game/${gameId}/pin`).then(() => undefined)
}
