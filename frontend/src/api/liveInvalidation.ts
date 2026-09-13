import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from './auth'
import { privateWorkspaceKeys } from './privateWorkspace'
import type { TournamentLiveSignal } from './tournamentLive'

const resumes = new WeakMap<QueryClient, Map<string, Promise<void>>>()

function refreshOnReturn(client: QueryClient, userId: string): Promise<void> {
  let pending = resumes.get(client)
  if (!pending) { pending = new Map(); resumes.set(client, pending) }
  const existing = pending.get(userId)
  if (existing) return existing
  const refresh = client.invalidateQueries({ queryKey: authKeys.session, exact: true }).then(async () => {
    // A return may discover expiry or another signed-in account. Never revive its predecessor.
    if (client.getQueryState(authKeys.session)?.status !== 'success'
      || client.getQueryData<AuthSession | null>(authKeys.session)?.user_id !== userId) return
    await invalidateLiveQueries(client, userId)
  }).finally(() => pending.delete(userId))
  pending.set(userId, refresh)
  return refresh
}

export function isLiveInvalidationTarget(queryKey: readonly unknown[], userId: string): boolean {
  if (queryKey[0] !== privateWorkspaceKeys.root[0] || queryKey[1] !== userId) return false
  return queryKey[2] !== 'course-catalog' && queryKey[2] !== 'course-provider'
}

export function invalidateLiveQueries(queryClient: QueryClient, userId: string): Promise<void> {
  return queryClient.invalidateQueries({
    predicate: (query) => isLiveInvalidationTarget(query.queryKey, userId),
  })
}

function isScoreInvalidationTarget(queryKey: readonly unknown[], userId: string): boolean {
  if (!isLiveInvalidationTarget(queryKey, userId)) return false
  if (queryKey[2] === 'leaderboards') return true
  return queryKey[2] === 'rounds'
    && (queryKey[4] === 'completion-validation' || queryKey[4] === 'scorecards')
}

export function isVisibilityProjectionTarget(queryKey: readonly unknown[], userId: string): boolean {
  if (queryKey[0] !== privateWorkspaceKeys.root[0] || queryKey[1] !== userId) return false
  if (queryKey[2] === 'leaderboards') return true
  if (queryKey[2] !== 'rounds') return false
  return queryKey[4] === 'completion-validation'
    || (queryKey[4] === 'scorecards' && queryKey[5] === 'read')
}

function failClosedVisibilityProjectionQueries(queryClient: QueryClient, userId: string): void {
  const projections = queryClient.getQueryCache().findAll({
    predicate: (query) => isVisibilityProjectionTarget(query.queryKey, userId),
  })
  for (const query of projections) {
    void query.cancel({ silent: true })
    query.setState({
      data: undefined,
      dataUpdatedAt: 0,
      error: null,
      errorUpdatedAt: 0,
      status: 'pending',
      fetchStatus: 'idle',
      fetchFailureCount: 0,
      fetchFailureReason: null,
      isInvalidated: false,
    })
  }
}

export function handleTournamentLiveSignal(
  queryClient: QueryClient,
  userId: string,
  signal: TournamentLiveSignal,
): Promise<void> {
  if (signal === 'resume') return refreshOnReturn(queryClient, userId)
  // Score saves/confirmations do not change tournament setup or score access.
  // Structural events and reconnection still reconcile the full private workspace.
  if (signal === 'score') {
    return queryClient.invalidateQueries({
      predicate: (query) => isScoreInvalidationTarget(query.queryKey, userId),
    })
  }
  if (signal === 'error') {
    failClosedVisibilityProjectionQueries(queryClient, userId)
    return Promise.resolve()
  }
  if (signal === 'open' || signal === 'visibility') {
    failClosedVisibilityProjectionQueries(queryClient, userId)
  }
  return invalidateLiveQueries(queryClient, userId)
}
