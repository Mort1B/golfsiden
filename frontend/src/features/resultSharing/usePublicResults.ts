import { useCallback, useEffect, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ApiHttpError } from '../../api/http'
import { resultSharingApi } from '../../api/resultSharing'
import { subscribePageReturn } from '../../api/pageReturn'
import type { LeaderboardMetric } from '../../api/types'

export function usePublicResults(grantId: string, token: string) {
  const client = useQueryClient()
  const [metric, setMetric] = useState<LeaderboardMetric>('gross')
  const [attempt, setAttempt] = useState(0)
  const [expired, setExpired] = useState(false)
  const query = useQuery({
    queryKey: ['public-results', grantId, metric, attempt],
    queryFn: ({ signal }) => resultSharingApi.read(grantId, token, metric, signal),
    enabled: !expired, retry: false, gcTime: 0, staleTime: Infinity, networkMode: 'always',
    refetchOnWindowFocus: false, refetchOnReconnect: false,
  })
  const terminal = expired || (query.error instanceof ApiHttpError && query.error.status === 404)
  const refresh = useCallback(() => {
    // Each request begins without any previous gross/net snapshot. Removal also
    // aborts the previous request, so a delayed response cannot restore it.
    client.removeQueries({ queryKey: ['public-results'] })
    setAttempt(value => value + 1)
  }, [client])
  useEffect(() => {
    if (terminal) return
    const returned = subscribePageReturn(refresh)
    window.addEventListener('offline', refresh)
    const timer = setInterval(() => { if (document.visibilityState === 'visible') refresh() }, 15_000)
    return () => { returned(); window.removeEventListener('offline', refresh); clearInterval(timer) }
  }, [refresh, terminal])
  useEffect(() => {
    const expires = query.data?.expires_at
    if (!expires) return
    let timer: ReturnType<typeof setTimeout>
    const check = () => {
      const remaining = Date.parse(expires) - Date.now()
      if (remaining <= 0) { client.removeQueries({ queryKey: ['public-results'] }); setExpired(true) }
      else timer = setTimeout(check, Math.min(remaining, 60_000))
    }
    check()
    return () => clearTimeout(timer)
  }, [client, query.data?.expires_at])
  return { query, metric, terminal, refresh, selectMetric: (next: LeaderboardMetric) => {
    if (next === metric || terminal) return
    refresh(); setMetric(next)
  } }
}
