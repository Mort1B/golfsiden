import type { Query, QueryClient, QueryKey } from '@tanstack/react-query'

// Ephemeral evidence for one reconciliation, never a retained server projection.
export function trackFantasyRefresh(client: QueryClient, root: QueryKey) {
  const cache = client.getQueryCache()
  const eligible = new Set(cache.findAll({ queryKey: root, type: 'active' }))
  const started = new Set<Query>(), completed = new Map<Query, number>()
  const stop = cache.subscribe(event => {
    const query = event.query
    if (!eligible.has(query)) return
    if (event.type === 'removed') { started.delete(query); completed.delete(query); return }
    if (event.type !== 'updated') return
    completed.delete(query)
    if (event.action.type === 'fetch') started.add(query)
    else if (event.action.type === 'success') {
      if (started.has(query) && !event.action.manual) completed.set(query, query.state.dataUpdateCount)
      started.delete(query)
    } else if (event.action.type === 'error' || event.action.type === 'setState' || event.action.type === 'invalidate') started.delete(query)
  })
  return { stop, reusable: (key: QueryKey) => {
    const query = cache.find({ queryKey: key, exact: true })
    if (!query) return false
    const state = query.state
    return completed.get(query) === state.dataUpdateCount && query.isActive()
      && state.status === 'success' && state.data !== undefined && state.fetchStatus === 'idle' && !state.isInvalidated
  } }
}
