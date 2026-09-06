import type { QueryClient } from '@tanstack/react-query'
import type { Round, Tournament } from '../../../api/types'
import { ApiHttpError } from '../../../api/http'

export function completionReadiness(tournament: Tournament, rounds: Round[] | undefined) {
  const items = rounds ?? []
  const valid = items.length === tournament.number_of_rounds
    && new Set(items.map((round) => round.id)).size === items.length
    && new Set(items.map((round) => round.round_number)).size === items.length
    && items.every((round) => round.tournament_id === tournament.id
      && round.round_number >= 1 && round.round_number <= tournament.number_of_rounds)
  const unlocked = items.filter((round) => round.tournament_id === tournament.id && round.status !== 'locked')
    .sort((a, b) => a.round_number - b.round_number)
  return { valid, unlocked, ready: valid && unlocked.length === 0 && tournament.status === 'active' }
}

export function completionFailure(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if (error.status === 401) return 'Økten er utløpt. Logg inn på nytt.'
    if (error.status === 403) return 'Du har ikke lenger administratortilgang til turneringen.'
    if (error.status === 404) return 'Turneringen finnes ikke lenger.'
    if (error.code === 'tournament_completion_not_ready') return 'Alle planlagte runder må være låst. Kontrollen oppdateres.'
    if (error.status === 409) return 'Turneringen er endret. Se oppdatert status og kontroller på nytt.'
  }
  return 'Svaret kunne ikke bekreftes. Kontroller oppdatert turneringsstatus før du prøver igjen.'
}

export async function reconcileCompletion(client: QueryClient, userId: string, tournamentId: string): Promise<void> {
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
