import { decodeRoundLeaderboard, decodeTournamentLeaderboard } from './leaderboards'
import { decodeAuthSession } from './auth'
import { ApiHttpError, jsonRequest, requestDecoded, requestUnchecked, tournamentLiveUrl } from './http'
import { decodeExpectedRound, decodeTournamentList, tournamentApi } from './tournaments'
import {
  decodeCompletionValidation,
  decodeSavedScore,
  decodeReadScorecard,
  decodeScoringScorecard,
  decodeScoreAccess,
  ownerTypeForFormat,
  type ScoreOwner,
} from './scorecards'
import type { LeaderboardMetric, ScoringFormat } from './types'
import { roundLifecycleApi } from './roundLifecycle'
import { teamApi } from './teams'

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
  tournaments: () => requestDecoded('/api/tournaments', decodeTournamentList),
  myTournaments: tournamentApi.mine,
  tournament: tournamentApi.detail,
  tournamentPlayers: tournamentApi.players,
  correctTournamentHandicap: tournamentApi.correctHandicap,
  rounds: tournamentApi.rounds,
  round: (id: string) => requestDecoded(`/api/rounds/${id}`, (value) => decodeExpectedRound(value, id)),
  pairingValidation: roundLifecycleApi.validation,
  openRound: roundLifecycleApi.open,
  teams: teamApi.list,
  roundLeaderboard: (id: string, tournamentId: string, metric: LeaderboardMetric) =>
    requestDecoded(`/api/rounds/${id}/leaderboards/${metric}`, (value) =>
      decodeRoundLeaderboard(value, id, tournamentId, metric)),
  tournamentLeaderboard: (id: string, metric: LeaderboardMetric) =>
    requestDecoded(`/api/tournaments/${id}/leaderboards/${metric}`, (value) =>
      decodeTournamentLeaderboard(value, id, metric)),
  completionValidation: (roundId: string, format: ScoringFormat) =>
    requestDecoded(`/api/rounds/${roundId}/completion-validation`, (value) =>
      decodeCompletionValidation(value, roundId, ownerTypeForFormat(format))),
  scoreAccess: (roundId: string) => requestDecoded(`/api/rounds/${roundId}/score-access`,
    (value) => decodeScoreAccess(value, roundId)),
  scorecardRead: (roundId: string, owner: ScoreOwner) =>
    requestDecoded(`/api/rounds/${roundId}/scorecards/${owner.type}/${owner.id}`, (value) =>
      decodeReadScorecard(value, roundId, owner)),
  scorecardScoring: (roundId: string, owner: ScoreOwner) =>
    requestDecoded(`/api/rounds/${roundId}/scorecards/${owner.type}/${owner.id}/scoring`, (value) =>
      decodeScoringScorecard(value, roundId, owner)),
  saveScore: (roundId: string, holeId: string, owner: ScoreOwner, grossStrokes: number, csrfToken: string) =>
    requestDecoded(`/api/rounds/${roundId}/scores`, (value) =>
      decodeSavedScore(value, roundId, holeId, owner, grossStrokes), jsonRequest('PUT', {
        hole_id: holeId,
        owner,
        gross_strokes: grossStrokes,
      }, csrfToken)),
  confirmScorecard: (roundId: string, owner: ScoreOwner, csrfToken: string) =>
    requestDecoded(`/api/rounds/${roundId}/scorecards/${owner.type}/${owner.id}/confirm`, (value) =>
      decodeScoringScorecard(value, roundId, owner), jsonRequest('POST', {}, csrfToken)),
  tournamentLiveUrl,
}
