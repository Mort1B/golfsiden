import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { fantasyApi, fantasyKeys } from '../../api/fantasy'
import { ApiHttpError } from '../../api/http'
import { loadPrivateResult } from '../../api/privateResults'

export function fantasyRoundQuery(client: QueryClient, tournament: string, round: string, session: AuthSession | null) {
  const user = session?.user_id ?? '', queryKey = fantasyKeys.query(user, tournament, 'round', round)
  return { queryKey, enabled: !!session, gcTime: 0,
    retry: (count: number, error: unknown) => count < 2 && error instanceof ApiHttpError && error.status === 409 && error.code === 'fantasy_conflict',
    retryDelay: 250,
    queryFn: ({ signal }: { signal: AbortSignal }) => loadPrivateResult(client, { userId: user, tournamentId: tournament }, queryKey, signal, async () => {
      const current = () => {
        const cached = client.getQueryData<AuthSession | null>(authKeys.session)
        return !!session && cached?.user_id === session.user_id && cached.csrf_token === session.csrf_token
      }
      if (!current()) throw new Error('Sesjonen er endret.')
      const view = await fantasyApi.round(tournament, round, user, signal)
      signal.throwIfAborted()
      if (!current()) throw new Error('Sesjonen er endret.')
      return view
    }),
  }
}
