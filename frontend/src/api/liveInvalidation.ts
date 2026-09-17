import type { QueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from './auth'
import { privateWorkspaceKeys } from './privateWorkspace'
import type { TournamentLiveSignal } from './tournamentLive'

interface ReturnRefresh { promise: Promise<void>; queued: boolean }
const resumes = new WeakMap<QueryClient, Map<string, ReturnRefresh>>()

function refreshOnReturn(client: QueryClient, userId: string): Promise<void> {
  let pending = resumes.get(client)
  if (!pending) { pending = new Map(); resumes.set(client, pending) }
  const existing = pending.get(userId)
  if (existing) {
    existing.queued = true
    return existing.promise
  }
  const refresh: ReturnRefresh = { promise: Promise.resolve(), queued: true }
  // Defer the first pass so same-turn subscribers share one refresh. Later
  // returns queue one follow-up without cancelling reads already in progress.
  refresh.promise = Promise.resolve().then(async () => {
    try {
      while (refresh.queued) {
        refresh.queued = false
        // An old user's pending read must not restart authentication after an
        // account switch. A cached same-user auth error can still be retried.
        if (client.getQueryData<AuthSession | null>(authKeys.session)?.user_id !== userId) return
        await client.invalidateQueries({ queryKey: authKeys.session, exact: true })
        if (client.getQueryState(authKeys.session)?.status !== 'success'
          || client.getQueryData<AuthSession | null>(authKeys.session)?.user_id !== userId) return
        await invalidateLiveQueries(client, userId)
      }
    } finally {
      // Clear synchronously with drain completion, before another return can
      // queue work on a promise whose loop has already finished.
      if (pending.get(userId) === refresh) pending.delete(userId)
    }
  })
  pending.set(userId, refresh)
  return refresh.promise
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
  if (queryKey[2] === 'leaderboards' || queryKey[2] === 'tournaments' && queryKey[4] === 'match-table') return true
  return queryKey[2] === 'rounds'
    && (queryKey[4] === 'completion-validation' || queryKey[4] === 'scorecards' || queryKey[4] === 'match-play')
}

export function isVisibilityProjectionTarget(queryKey: readonly unknown[], userId: string): boolean {
  if (queryKey[0] !== privateWorkspaceKeys.root[0] || queryKey[1] !== userId) return false
  if (queryKey[2] === 'leaderboards' || queryKey[2] === 'tournaments' && queryKey[4] === 'match-table') return true
  if (queryKey[2] !== 'rounds') return false
  return queryKey[4] === 'match-play' && queryKey[5] !== 'scoring' || queryKey[4] === 'completion-validation'
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
  if (signal === 'score' || signal === 'match') {
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
