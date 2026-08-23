import type { QueryClient } from '@tanstack/react-query'
import { authKeys } from './auth'

export function isLiveInvalidationTarget(queryKey: readonly unknown[]): boolean {
  if (queryKey[0] === authKeys.session[0]) return false
  return queryKey[2] !== 'course-catalog' && queryKey[2] !== 'course-provider'
}

export function invalidateLiveQueries(queryClient: QueryClient): Promise<void> {
  return queryClient.invalidateQueries({
    predicate: (query) => isLiveInvalidationTarget(query.queryKey),
  })
}
