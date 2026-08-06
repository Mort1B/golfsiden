import { decodeRoundLeaderboard, decodeTournamentLeaderboard } from './leaderboards'
import type { LeaderboardMetric, Player, Round, Team, Tournament, TournamentPlayer } from './types'

const apiUrl = import.meta.env.VITE_API_URL ?? ''

interface ApiErrorBody {
  error?: { message?: string }
}

async function get<T>(path: string): Promise<T> {
  const response = await fetch(`${apiUrl}${path}`)
  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as ApiErrorBody
    throw new Error(body.error?.message ?? `Forespørselen feilet (${response.status})`)
  }
  return response.json() as Promise<T>
}

async function getDecoded<T>(path: string, decode: (value: unknown) => T): Promise<T> {
  const response = await fetch(`${apiUrl}${path}`)
  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as ApiErrorBody
    throw new Error(body.error?.message ?? `Forespørselen feilet (${response.status})`)
  }
  return decode(await response.json() as unknown)
}

export const api = {
  tournaments: () => get<Tournament[]>('/api/tournaments'),
  tournament: (id: string) => get<Tournament>(`/api/tournaments/${id}`),
  tournamentPlayers: (id: string) => get<TournamentPlayer[]>(`/api/tournaments/${id}/players`),
  rounds: (id: string) => get<Round[]>(`/api/tournaments/${id}/rounds`),
  round: (id: string) => get<Round>(`/api/rounds/${id}`),
  teams: (id: string) => get<Team[]>(`/api/rounds/${id}/teams`),
  players: () => get<Player[]>('/api/players'),
  roundLeaderboard: (id: string, metric: LeaderboardMetric) =>
    getDecoded(`/api/rounds/${id}/leaderboards/${metric}`, (value) =>
      decodeRoundLeaderboard(value, id, metric)),
  tournamentLeaderboard: (id: string, metric: LeaderboardMetric) =>
    getDecoded(`/api/tournaments/${id}/leaderboards/${metric}`, (value) =>
      decodeTournamentLeaderboard(value, id, metric)),
  liveUrl: `${apiUrl}/api/live`,
}
