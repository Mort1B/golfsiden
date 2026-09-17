import { loadPrivateResult } from '../../api/privateResults'
import type { QueryClient, QueryKey } from '@tanstack/react-query'
import { validateTournamentLeaderboardRounds } from '../../api/leaderboards'
import type { Round, TournamentLeaderboard } from '../../api/types'

interface TournamentLeaderboardLoad {
  queryClient: QueryClient
  roundsQueryKey: QueryKey
  signal?: AbortSignal
  loadRounds: () => Promise<Round[]>
  loadLeaderboard: () => Promise<TournamentLeaderboard>
}

export async function loadTournamentLeaderboardAfterRounds({
  queryClient,
  roundsQueryKey,
  loadRounds,
  signal,
  loadLeaderboard,
}: TournamentLeaderboardLoad): Promise<TournamentLeaderboard> {
  signal?.throwIfAborted()
  const userId = roundsQueryKey[1], tournamentId = roundsQueryKey[3]
  if (typeof userId !== 'string' || typeof tournamentId !== 'string') throw new Error('Ugyldig resultatmål.')
  const rounds = await queryClient.fetchQuery({
    queryKey: roundsQueryKey,
    queryFn: context => loadPrivateResult(queryClient, { userId, tournamentId }, roundsQueryKey, context.signal, loadRounds),
    staleTime: 0,
  })
  signal?.throwIfAborted()
  const leaderboard = await loadLeaderboard()
  signal?.throwIfAborted()
  return validateTournamentLeaderboardRounds(leaderboard, rounds)
}
