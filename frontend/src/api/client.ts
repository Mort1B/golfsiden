import { decodeRoundLeaderboard, decodeTournamentLeaderboard } from './leaderboards'
import { decodeAuthSession } from './auth'
import { ApiHttpError, jsonRequest, liveUrl, requestDecoded, requestUnchecked } from './http'
import { decodeArray } from './decoder'
import { decodeRound, decodeTournament, tournamentApi } from './tournaments'
import {
  decodeCompletionValidation,
  decodeSavedScore,
  decodeScorecard,
  decodeScoreAccess,
  ownerTypeForFormat,
  type ScoreOwner,
} from './scorecards'
import type { LeaderboardMetric, Player, ScoringFormat, Team } from './types'
import { roundLifecycleApi } from './roundLifecycle'

async function get<T>(path: string): Promise<T> {
  return requestUnchecked<T>(path)
}

export const api = {
  login: (username: string, password: string) => requestDecoded('/api/auth/login', decodeAuthSession,
    jsonRequest('POST', { username, password })),
  session: async () => {
    try {
      return await requestDecoded('/api/auth/session', decodeAuthSession)
    } catch (error) {
      if (error instanceof ApiHttpError && error.status === 401) return null
      throw error
    }
  },
  logout: (csrfToken: string) => requestUnchecked<undefined>('/api/auth/logout', {
    method: 'POST',
    headers: { 'x-csrf-token': csrfToken },
  }),
  tournaments: () => requestDecoded('/api/tournaments', (value) =>
    decodeArray(value, 'tournaments', decodeTournament, 'turneringsdata')),
  myTournaments: tournamentApi.mine,
  tournament: tournamentApi.detail,
  tournamentPlayers: tournamentApi.players,
  correctTournamentHandicap: tournamentApi.correctHandicap,
  rounds: tournamentApi.rounds,
  round: (id: string) => requestDecoded(`/api/rounds/${id}`, (value) => decodeRound(value)),
  pairingValidation: roundLifecycleApi.validation,
  openRound: roundLifecycleApi.open,
  teams: (id: string) => get<Team[]>(`/api/rounds/${id}/teams`),
  players: () => get<Player[]>('/api/players'),
  roundLeaderboard: (id: string, metric: LeaderboardMetric) =>
    requestDecoded(`/api/rounds/${id}/leaderboards/${metric}`, (value) =>
      decodeRoundLeaderboard(value, id, metric)),
  tournamentLeaderboard: (id: string, metric: LeaderboardMetric) =>
    requestDecoded(`/api/tournaments/${id}/leaderboards/${metric}`, (value) =>
      decodeTournamentLeaderboard(value, id, metric)),
  completionValidation: (roundId: string, format: ScoringFormat) =>
    requestDecoded(`/api/rounds/${roundId}/completion-validation`, (value) =>
      decodeCompletionValidation(value, roundId, ownerTypeForFormat(format))),
  scoreAccess: (roundId: string) => requestDecoded(`/api/rounds/${roundId}/score-access`,
    (value) => decodeScoreAccess(value, roundId)),
  scorecard: (roundId: string, owner: ScoreOwner) =>
    requestDecoded(`/api/rounds/${roundId}/scorecards/${owner.type}/${owner.id}`, (value) =>
      decodeScorecard(value, roundId, owner)),
  saveScore: (roundId: string, holeId: string, owner: ScoreOwner, grossStrokes: number, csrfToken: string) =>
    requestDecoded(`/api/rounds/${roundId}/scores`, (value) =>
      decodeSavedScore(value, roundId, holeId, owner, grossStrokes), jsonRequest('PUT', {
        hole_id: holeId,
        owner,
        gross_strokes: grossStrokes,
      }, csrfToken)),
  confirmScorecard: (roundId: string, owner: ScoreOwner, csrfToken: string) =>
    requestDecoded(`/api/rounds/${roundId}/scorecards/${owner.type}/${owner.id}/confirm`, (value) =>
      decodeScorecard(value, roundId, owner), jsonRequest('POST', {}, csrfToken)),
  liveUrl,
}
