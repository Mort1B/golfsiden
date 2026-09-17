import { hashKey, type QueryClient, type QueryKey } from '@tanstack/react-query'
import { ApiHttpError } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'

const identities = new WeakMap<object, PrivateResultScope>()

export interface PrivateResultScope { userId: string; tournamentId?: string; roundId?: string }
export function isPrivateResultDenial(error: unknown): error is ApiHttpError {
  return error instanceof ApiHttpError && [401, 403, 404].includes(error.status)
}
function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}
function resultKey(key: QueryKey): boolean {
  return key[2] === 'leaderboards'
    || key[2] === 'tournaments' && ['rounds', 'match-table'].includes(String(key[4]))
    || key[2] === 'rounds' && (key[4] === 'detail'
      || key[4] === 'scorecards' && key[5] === 'read'
      || key[4] === 'match-play' && ['read', 'read-list'].includes(String(key[5])))
}
// Scope matching uses canonical keys and decoded identities, not mutable query meta.
// Other consumers (including fetchQuery) can replace meta on shared cache entries.
export function denyPrivateResults(client: QueryClient, scope: PrivateResultScope, source: QueryKey, error: ApiHttpError): void {
  const queries = client.getQueryCache().findAll({ queryKey: privateWorkspaceKeys.user(scope.userId) })
  const roundTrips = new Map<string, string>()
  for (const query of queries) {
    const known = identities.get(query), key = query.queryKey, data: unknown = query.state.data
    if (known?.tournamentId && known.roundId) roundTrips.set(known.roundId, known.tournamentId)
    if (record(data) && typeof data.tournament_id === 'string') {
      const roundId = typeof data.round_id === 'string' ? data.round_id : key[2] === 'rounds' ? key[3] : undefined
      if (typeof roundId === 'string') roundTrips.set(roundId, data.tournament_id)
    }
    if (Array.isArray(data) && key[2] === 'tournaments' && key[4] === 'rounds') {
      for (const r of data) if (record(r) && typeof r.id === 'string' && typeof key[3] === 'string') roundTrips.set(r.id, key[3])
    }
  }
  const tournamentId = scope.tournamentId ?? (scope.roundId ? roundTrips.get(scope.roundId) : undefined)
  const roundIds = new Set(scope.roundId ? [scope.roundId] : [])
  for (const [roundId, trip] of roundTrips) if (trip === tournamentId) roundIds.add(roundId)
  const all = !tournamentId && !scope.roundId
  const sourceHash = hashKey(source)
  for (const query of queries) {
    const key = query.queryKey, data: unknown = query.state.data
    const known = identities.get(query)
    const sameTournament = tournamentId !== undefined && (known?.tournamentId === tournamentId || key[2] === 'tournaments' && key[3] === tournamentId
      || key[2] === 'leaderboards' && key[3] === 'tournament' && key[4] === tournamentId
      || record(data) && data.tournament_id === tournamentId)
    const sameRound = roundIds.has(String(key[2] === 'leaderboards' ? key[4] : key[3]))
    const roundKey = key[2] === 'rounds' ? key[3] : key[2] === 'leaderboards' && key[3] === 'round' ? key[4] : undefined
    // A shared-key consumer may have created an unregistered read after its round
    // metadata was evicted. Missing scope is never evidence of retained authority.
    const unknownRound = typeof roundKey === 'string' && !roundTrips.has(roundKey)
    if (query.queryHash !== sourceHash && !(resultKey(key) && (all || sameTournament || sameRound || unknownRound))) continue
    // Cancellation must happen before erasure: an old successful read must never
    // refill the cache, nor may cancellation restore its pre-denial snapshot.
    void query.cancel({ silent: true, revert: false })
    query.setState({ data: undefined, dataUpdatedAt: 0, error, errorUpdatedAt: Date.now(),
      status: 'error', fetchStatus: 'idle',
      isInvalidated: true })
  }
}

export async function loadPrivateResult<T>(client: QueryClient, scope: PrivateResultScope, key: QueryKey, signal: AbortSignal, load: () => Promise<T>): Promise<T> {
  const query = client.getQueryCache().find({ queryKey: key, exact: true })
  if (query) identities.set(query, scope)
  try {
    signal.throwIfAborted()
    const data = await load()
    signal.throwIfAborted()
    return data
  } catch (error) {
    // Superseded reads cannot erase a newer authorized response either.
    if (!signal.aborted && isPrivateResultDenial(error)) denyPrivateResults(client, scope, key, error)
    throw error
  }
}
