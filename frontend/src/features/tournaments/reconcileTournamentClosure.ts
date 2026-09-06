import type { QueryClient } from '@tanstack/react-query'

export async function reconcileTournamentClosure(client: QueryClient, userId: string, tournamentId: string): Promise<void> {
  const filters = { predicate: (query: { queryKey: readonly unknown[] }) => {
    const key = query.queryKey
    if (key[0] !== 'private-workspace' || key[1] !== userId) return false
    return (key[2] === 'tournaments' && [tournamentId, 'mine', 'list'].includes(String(key[3])))
      || (key[2] === 'invitations' && key[3] === tournamentId)
      || (key[2] === 'leaderboards' && key[3] === 'tournament' && key[4] === tournamentId)
  } }
  // Supersede pre-mutation reads; a newer SSE refetch is not a failed refresh.
  // Never insert data from a mutation response into a potentially signed-out cache.
  await client.invalidateQueries(filters)
  if (client.getQueryCache().findAll(filters).some((query) => query.isActive()
    && (query.state.status === 'error' || query.state.fetchStatus === 'paused'
      || (query.state.isInvalidated && query.state.fetchStatus === 'idle')))) {
    throw new Error('Turneringsdata kunne ikke oppdateres.')
  }
}
