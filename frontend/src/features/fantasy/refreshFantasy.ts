import type { QueryClient } from '@tanstack/react-query'
import type { AuthSession } from '../../api/auth'
import { fantasyApi, fantasyKeys, type OwnerKind } from '../../api/fantasy'
import { loadPrivateResult } from '../../api/privateResults'

// Only identifiers survive query unmounts; no projection or query closure is retained.
export type FantasyReadTarget =
  | { kind: 'game'; round?: string }
  | { kind: 'round'; round: string }
  | { kind: 'source'; round: string; ownerKind: OwnerKind; owner: string }

export async function refreshFantasyTarget(
  client: QueryClient, tournament: string, session: AuthSession,
  target: FantasyReadTarget, current: () => boolean,
): Promise<void> {
  const read = <T,>(parts: string[], load: (signal: AbortSignal) => Promise<T>): Promise<T> => {
    const key = fantasyKeys.query(session.user_id, tournament, ...parts)
    return client.fetchQuery({ queryKey: key, staleTime: 0, gcTime: 0, retry: false,
      queryFn: ({ signal }) => loadPrivateResult(client, { userId: session.user_id, tournamentId: tournament }, key, signal, async () => {
        if (!current()) throw new Error('Sesjonen er endret.')
        const data = await load(signal)
        signal.throwIfAborted()
        if (!current()) throw new Error('Sesjonen er endret.')
        return data
      }),
    })
  }
  if (target.kind === 'source') {
    await read(['source', target.round, target.ownerKind, target.owner], signal => fantasyApi.source(tournament, target.round, target.ownerKind, target.owner, signal))
  } else if (target.kind === 'round') {
    await read(['round', target.round], signal => fantasyApi.round(tournament, target.round, session.user_id, signal))
  } else {
    const game = await read(['game'], signal => fantasyApi.game(tournament, signal))
    // Enabling the game mounts these reads after configuration succeeds. They
    // must reconcile even if React has not mounted their observers yet.
    if (!game?.enabled || !current()) return
    await read(['results'], signal => fantasyApi.results(tournament, signal))
    if (target.round && current()) {
      const round = target.round
      await read(['round', round], signal => fantasyApi.round(tournament, round, session.user_id, signal))
      if (current()) await read(['round-results', round], signal => fantasyApi.roundResults(tournament, round, signal))
    }
  }
}
