import type { QueryClient } from '@tanstack/react-query'
import { authKeys } from './auth'

export function invalidateLiveQueries(queryClient: QueryClient): Promise<void> {
  return queryClient.invalidateQueries({
    predicate: (query) => query.queryKey[0] !== authKeys.session[0],
  })
}
