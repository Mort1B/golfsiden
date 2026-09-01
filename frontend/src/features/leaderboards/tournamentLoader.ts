import type { QueryClient, QueryKey } from '@tanstack/react-query'
import { validateTournamentLeaderboardRounds } from '../../api/leaderboards'
import type { Round, TournamentLeaderboard } from '../../api/types'

interface TournamentLeaderboardLoad {
  queryClient: QueryClient
  roundsQueryKey: QueryKey
  loadRounds: () => Promise<Round[]>
  loadLeaderboard: () => Promise<TournamentLeaderboard>
}

export async function loadTournamentLeaderboardAfterRounds({
  queryClient,
  roundsQueryKey,
  loadRounds,
  loadLeaderboard,
}: TournamentLeaderboardLoad): Promise<TournamentLeaderboard> {
  const rounds = await queryClient.fetchQuery({
    queryKey: roundsQueryKey,
    queryFn: loadRounds,
    staleTime: 0,
  })
  const leaderboard = await loadLeaderboard()
  return validateTournamentLeaderboardRounds(leaderboard, rounds)
}
