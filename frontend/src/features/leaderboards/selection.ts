import type { LeaderboardMetric, Round } from '../../api/types'

export type LeaderboardScope = 'round' | 'tournament'

export function parseScope(value: string | null): LeaderboardScope {
  return value === 'tournament' ? 'tournament' : 'round'
}

export function parseMetric(value: string | null): LeaderboardMetric {
  return value === 'gross' ? 'gross' : 'net'
}

export function preferredRound(rounds: Round[]): Round | undefined {
  const descending = (left: Round, right: Round) => right.round_number - left.round_number
  const open = rounds.filter((round) => round.status === 'open').sort(descending)[0]
  if (open) return open

  const finished = rounds
    .filter((round) => round.status === 'completed' || round.status === 'locked')
    .sort(descending)[0]
  if (finished) return finished

  return rounds
    .filter((round) => round.status === 'draft')
    .sort((left, right) => left.round_number - right.round_number)[0]
}

export function leaderboardSearch(
  tournamentId: string,
  scope: LeaderboardScope,
  roundId: string | undefined,
  metric: LeaderboardMetric,
): URLSearchParams {
  const params = new URLSearchParams()
  params.set('tournament', tournamentId)
  params.set('scope', scope)
  if (roundId) params.set('round', roundId)
  params.set('metric', metric)
  return params
}
