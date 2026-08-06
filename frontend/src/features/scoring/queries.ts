import type { QueryClient } from '@tanstack/react-query'
import { leaderboardKeys } from '../../api/leaderboards'
import { scoringKeys, type ScoreOwner } from '../../api/scorecards'

export async function invalidateScoreDependents(
  queryClient: QueryClient,
  roundId: string,
  tournamentId: string,
): Promise<void> {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: scoringKeys.completion(roundId), exact: true }),
    queryClient.invalidateQueries({ queryKey: leaderboardKeys.round(roundId, 'gross'), exact: true }),
    queryClient.invalidateQueries({ queryKey: leaderboardKeys.round(roundId, 'net'), exact: true }),
    queryClient.invalidateQueries({ queryKey: leaderboardKeys.tournament(tournamentId, 'gross'), exact: true }),
    queryClient.invalidateQueries({ queryKey: leaderboardKeys.tournament(tournamentId, 'net'), exact: true }),
  ])
}

export async function invalidateScorecard(
  queryClient: QueryClient,
  roundId: string,
  owner: ScoreOwner,
): Promise<void> {
  await queryClient.invalidateQueries({
    queryKey: scoringKeys.scorecard(roundId, owner),
    exact: true,
    refetchType: 'none',
  })
}
