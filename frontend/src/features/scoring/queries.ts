import type { QueryClient } from '@tanstack/react-query'
import { leaderboardKeys } from '../../api/leaderboards'
import { scoringKeys, type ScoreOwner } from '../../api/scorecards'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'

export async function invalidateScoreDependents(
  queryClient: QueryClient,
  userId: string,
  roundId: string,
  tournamentId: string,
): Promise<void> {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: privateWorkspaceKeys.completion(userId, roundId), exact: true }),
    queryClient.invalidateQueries({ queryKey: leaderboardKeys.round(userId, roundId, 'gross'), exact: true }),
    queryClient.invalidateQueries({ queryKey: leaderboardKeys.round(userId, roundId, 'net'), exact: true }),
    queryClient.invalidateQueries({ queryKey: leaderboardKeys.tournament(userId, tournamentId, 'gross'), exact: true }),
    queryClient.invalidateQueries({ queryKey: leaderboardKeys.tournament(userId, tournamentId, 'net'), exact: true }),
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
