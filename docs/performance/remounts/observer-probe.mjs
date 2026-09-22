// Isolated installed-library model, not application code or a production repair.
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { writeFileSync } from 'node:fs'
import assert from 'node:assert/strict'
const root = fileURLToPath(new URL('../../../', import.meta.url))
const { QueryClient, QueryObserver } = createRequire(resolve(root, 'frontend/package.json'))('@tanstack/react-query')
const rows = [], realNow = Date.now
try {
  for (const ownership of ['unmount', 'disabled-observer']) {
    for (const order of ['table-first', 'list-first-fresh', 'list-first-stale']) {
      let now = 100000, starts = 0, aborts = 0, completed = 0
      Date.now = () => now
      const client = new QueryClient({ defaultOptions: { queries: { staleTime: 20000, retry: false, gcTime: Infinity } } })
      const key = ['private-workspace', 'synthetic-user', 'rounds', 'synthetic-round', 'match-play', 'read-list']
      const pending = []
      const options = enabled => ({ queryKey: key, enabled, queryFn: ({ signal }) => {
        starts++
        return new Promise((resolve, reject) => {
          const request = { aborted: false, finished: false, finish() { if (!this.aborted && !this.finished) { this.finished = true; completed++; resolve('fresh-list') } } }
          signal.addEventListener('abort', () => { request.aborted = true; aborts++; reject(signal.reason) }, { once: true })
          pending.push(request)
        })
      } })
      client.setQueryData(key, 'previous-authorized-list')
      let observer = new QueryObserver(client, options(true)), stop = observer.subscribe(() => undefined)
      assert.equal(starts, 0)
      const query = client.getQueryCache().find({ queryKey: key, exact: true })
      await query.cancel({ silent: true })
      query.setState({ data: undefined, dataUpdatedAt: 0, error: null, errorUpdatedAt: 0, status: 'pending', fetchStatus: 'idle', fetchFailureCount: 0, fetchFailureReason: null, isInvalidated: false })
      const refresh = client.invalidateQueries({ queryKey: key })
      assert.equal(starts, 1)
      if (ownership === 'unmount') stop()
      else observer.setOptions(options(false))
      const observersWhilePending = query.getObserversCount()
      if (order !== 'table-first') { pending[0].finish(); await Promise.resolve(); await Promise.resolve() }
      if (order === 'list-first-stale') now += 21000
      if (ownership === 'unmount') { observer = new QueryObserver(client, options(true)); stop = observer.subscribe(() => undefined) }
      else observer.setOptions(options(true))
      for (const request of pending) request.finish()
      await refresh
      await Promise.resolve(); await Promise.resolve()
      assert.equal(query.state.data, 'fresh-list')
      const expectedStarts = ownership === 'unmount' || order === 'list-first-stale' ? 2 : 1
      assert.equal(starts, expectedStarts)
      assert.equal(aborts, ownership === 'unmount' ? 1 : 0)
      assert.equal(completed, starts - aborts)
      rows.push({ ownership, order, simulatedElapsedMs: now - 100000, staleTimeMs: 20000, starts, aborts, completed, observersWhilePending })
      stop(); client.clear()
    }
  }
} finally { Date.now = realNow }
const coldClient = new QueryClient({ defaultOptions: { queries: { staleTime: 20000, retry: false, gcTime: Infinity } } })
let coldStarts = 0
const coldOptions = { queryKey: ['cold-list'], queryFn: async () => { coldStarts++; return 'fresh-list' } }
const cold = new QueryObserver(coldClient, { ...coldOptions, enabled: false })
const stopCold = cold.subscribe(() => undefined)
assert.equal(coldStarts, 0)
cold.setOptions({ ...coldOptions, enabled: true })
await Promise.resolve(); await Promise.resolve()
assert.equal(coldStarts, 1)
stopCold(); coldClient.clear()
const output = { description: 'Isolated QueryObserver lifecycle model using installed library; logical clock advance, no browser/network or production change', coldGate: { startsBeforeReady: 0, startsAfterReady: coldStarts }, rows }
writeFileSync(process.argv[2] ?? '/tmp/remount-observer-probe.json', JSON.stringify(output, null, 2) + '\n')
console.log(JSON.stringify(output, null, 2))
