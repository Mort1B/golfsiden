import { decodeObject, invalidData } from './decoder'
import { decodeTournamentLeaderboard } from './leaderboards'
import { requestDecoded } from './http'
import type { LeaderboardMetric, TournamentLeaderboard } from './types'
export interface OverallNotApplicable { type: 'not_applicable'; reason: 'match_only'; tournament_id: string; metric: LeaderboardMetric }
export type OverallResponse = TournamentLeaderboard | OverallNotApplicable
export function decodeOverall(value: unknown, tournament: string, metric: LeaderboardMetric): OverallResponse {
  const d = decodeObject(value, 'overall')
  if (d.type === 'not_applicable') {
    if (Object.keys(d).length !== 4 || d.reason !== 'match_only' || d.tournament_id !== tournament || d.metric !== metric) invalidData('resultatdata', 'overall')
    return { type: 'not_applicable', reason: 'match_only', tournament_id: tournament, metric }
  }
  return decodeTournamentLeaderboard(value, tournament, metric)
}
export const overallApi = { get: (id: string, metric: LeaderboardMetric) => requestDecoded(`/api/tournaments/${id}/leaderboards/${metric}`, v => decodeOverall(v, id, metric)) }
