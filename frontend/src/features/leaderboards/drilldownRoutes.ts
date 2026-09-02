import type { LeaderboardMetric, LeaderboardOwner } from '../../api/types'
import type { ScoreView } from '../scoring/selection'

export function parseDrilldownMetric(value: string | null): LeaderboardMetric {
  return value === 'net' ? 'net' : 'gross'
}

export function parseOwnerType(value: string | undefined): LeaderboardOwner['type'] | null {
  return value === 'player' || value === 'team' ? value : null
}

export function playerHistoryUrl(tournamentId: string, playerId: string, metric: LeaderboardMetric): string {
  return `/tournaments/${tournamentId}/results/players/${playerId}?metric=${metric}`
}

export function scorecardUrl(
  tournamentId: string,
  roundId: string,
  owner: LeaderboardOwner,
  metric: LeaderboardMetric,
  view: ScoreView = 'summary',
  hole?: number,
): string {
  const search = scorecardSearch(metric, view, hole)
  return `/tournaments/${tournamentId}/rounds/${roundId}/scorecards/${owner.type}/${owner.id}?${search}`
}

export function scorecardSearch(metric: LeaderboardMetric, view: ScoreView, hole?: number): URLSearchParams {
  const search = new URLSearchParams({ metric, view })
  if (view === 'hole' && hole !== undefined) search.set('hole', String(hole))
  return search
}
