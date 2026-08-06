import { decodeRoundLeaderboard, decodeTournamentLeaderboard } from './leaderboards'
import { jsonRequest, liveUrl, requestDecoded, requestUnchecked } from './http'
import {
  decodeCompletionValidation,
  decodeSavedScore,
  decodeScorecard,
  ownerTypeForFormat,
  type ScoreOwner,
} from './scorecards'
import type { LeaderboardMetric, Player, Round, ScoringFormat, Team, Tournament, TournamentPlayer } from './types'

async function get<T>(path: string): Promise<T> {
  return requestUnchecked<T>(path)
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
    requestDecoded(`/api/rounds/${id}/leaderboards/${metric}`, (value) =>
      decodeRoundLeaderboard(value, id, metric)),
  tournamentLeaderboard: (id: string, metric: LeaderboardMetric) =>
    requestDecoded(`/api/tournaments/${id}/leaderboards/${metric}`, (value) =>
      decodeTournamentLeaderboard(value, id, metric)),
  completionValidation: (roundId: string, format: ScoringFormat) =>
    requestDecoded(`/api/rounds/${roundId}/completion-validation`, (value) =>
      decodeCompletionValidation(value, roundId, ownerTypeForFormat(format))),
  scorecard: (roundId: string, owner: ScoreOwner) =>
    requestDecoded(`/api/rounds/${roundId}/scorecards/${owner.type}/${owner.id}`, (value) =>
      decodeScorecard(value, roundId, owner)),
  saveScore: (roundId: string, holeId: string, owner: ScoreOwner, grossStrokes: number, submittedBy: string) =>
    requestDecoded(`/api/rounds/${roundId}/scores`, (value) =>
      decodeSavedScore(value, roundId, holeId, owner, grossStrokes), jsonRequest('PUT', {
        hole_id: holeId,
        owner,
        gross_strokes: grossStrokes,
        submitted_by: submittedBy,
      })),
  confirmScorecard: (roundId: string, owner: ScoreOwner, confirmedBy: string) =>
    requestDecoded(`/api/rounds/${roundId}/scorecards/${owner.type}/${owner.id}/confirm`, (value) =>
      decodeScorecard(value, roundId, owner), jsonRequest('POST', { confirmed_by: confirmedBy })),
  liveUrl,
}
