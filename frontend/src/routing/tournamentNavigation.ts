import { createContext, useContext, useEffect } from 'react'
import type { LeaderboardMetric, Round } from '../api/types'
import { leaderboardSearch, type LeaderboardScope } from '../features/leaderboards/selection'

export interface TournamentNavigationSelection {
  tournamentId: string
  roundId?: string | null
  scope?: LeaderboardScope
  metric?: LeaderboardMetric
}

export const TournamentNavigationContext = createContext<{
  selection: TournamentNavigationSelection | null
  pending: boolean
  publish: (selection: TournamentNavigationSelection | null, validRoundIds?: readonly string[]) => void
} | null>(null)

// Route owners publish IDs only after their typed authority queries succeed.
// Standalone route renders may omit the shell: there is no navigation to update.
export function usePublishTournamentNavigation(selection: TournamentNavigationSelection | null, ready: boolean, rounds?: readonly Round[]) {
  const publish = useContext(TournamentNavigationContext)?.publish
  const tournamentId = selection?.tournamentId, roundId = selection?.roundId
  const scope = selection?.scope, metric = selection?.metric
  useEffect(() => {
    if (ready) publish?.(tournamentId ? { tournamentId, roundId, scope, metric } : null,
      rounds?.filter(round => round.tournament_id === tournamentId).map(round => round.id))
  }, [publish, ready, tournamentId, roundId, scope, metric, rounds])
}

export function tournamentNavigationLinks(selection: TournamentNavigationSelection | null) {
  if (!selection) return { score: '/score', results: '/leaderboard' }
  const search = new URLSearchParams({ tournament: selection.tournamentId })
  if (selection.roundId) search.set('round', selection.roundId)
  search.set('resume', '1')
  return {
    score: `/score?${search}`,
    results: `/leaderboard?${leaderboardSearch(selection.tournamentId, selection.scope ?? 'round', selection.roundId ?? undefined, selection.metric ?? 'net')}`,
  }
}

export function isScoreResumeSearch(search: string): boolean {
  const params = new URLSearchParams(search)
  return search === '' || params.get('resume') === '1'
    && [...params.keys()].every(key => ['tournament', 'round', 'resume'].includes(key))
}
