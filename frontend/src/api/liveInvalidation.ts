import type { QueryClient } from '@tanstack/react-query'
import { privateWorkspaceKeys } from './privateWorkspace'
import type { TournamentLiveSignal } from './tournamentLive'

export function isLiveInvalidationTarget(queryKey: readonly unknown[], userId: string): boolean {
  if (queryKey[0] !== privateWorkspaceKeys.root[0] || queryKey[1] !== userId) return false
  return queryKey[2] !== 'course-catalog' && queryKey[2] !== 'course-provider'
}

export function invalidateLiveQueries(queryClient: QueryClient, userId: string): Promise<void> {
  return queryClient.invalidateQueries({
    predicate: (query) => isLiveInvalidationTarget(query.queryKey, userId),
  })
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
  if (signal === 'error') {
    failClosedVisibilityProjectionQueries(queryClient, userId)
    return Promise.resolve()
  }
  if (signal === 'open' || signal === 'visibility') {
    failClosedVisibilityProjectionQueries(queryClient, userId)
  }
  return invalidateLiveQueries(queryClient, userId)
}
