import type { QueryClient } from '@tanstack/react-query'
import { privateWorkspaceKeys } from './privateWorkspace'

export function isLiveInvalidationTarget(queryKey: readonly unknown[], userId: string): boolean {
  if (queryKey[0] !== privateWorkspaceKeys.root[0] || queryKey[1] !== userId) return false
  return queryKey[2] !== 'course-catalog' && queryKey[2] !== 'course-provider'
}

export function invalidateLiveQueries(queryClient: QueryClient, userId: string): Promise<void> {
  return queryClient.invalidateQueries({
    predicate: (query) => isLiveInvalidationTarget(query.queryKey, userId),
  })
}
