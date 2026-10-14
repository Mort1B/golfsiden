import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { trackFantasyRefresh } from './fantasyRefreshProof'

const root = ['fantasy'], key = [...root, 'round']
let client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
const stops: (() => void)[] = []
afterEach(() => { for (const stop of stops.splice(0)) stop(); client.clear(); client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } }) })
function observe(load = async () => 'fresh') {
  const observer = new QueryObserver(client, { queryKey: key, queryFn: load })
  const stop = observer.subscribe(() => {})
  stops.push(stop)
  return { stop, ready: () => client.fetchQuery({ queryKey: key, queryFn: load }) }
}
async function refreshed() {
  const observed = observe(); await observed.ready()
  const proof = trackFantasyRefresh(client, root)
  try { await client.invalidateQueries({ queryKey: root }, { throwOnError: true }) } finally { proof.stop() }
  expect(proof.reusable(key)).toBe(true)
  return { proof, observed }
}
it('proves a new network fetch, not a manual cache insertion', async () => {
  const observed = observe(); await observed.ready()
  const proof = trackFantasyRefresh(client, root)
  client.setQueryData(key, 'manual')
  expect(proof.reusable(key)).toBe(false)
  await client.invalidateQueries({ queryKey: root }, { throwOnError: true })
  proof.stop()
  expect(proof.reusable(key)).toBe(true)
})
it('does not reuse an initial request started before reconciliation', async () => {
  let finish: (value: string) => void = () => {}
  const pending = new Promise<string>(resolve => { finish = resolve })
  observe(() => pending)
  const proof = trackFantasyRefresh(client, root)
  const refresh = client.invalidateQueries({ queryKey: root })
  finish('old request'); await refresh; proof.stop()
  expect(client.getQueryData(key)).toBe('old request')
  expect(proof.reusable(key)).toBe(false)
})
it.each(['invalidate', 'manual', 'erase', 'inactive', 'replace'] as const)('rejects %s after proof', async change => {
  const { proof, observed } = await refreshed()
  if (change === 'invalidate') await client.invalidateQueries({ queryKey: root, refetchType: 'none' })
  if (change === 'manual') client.setQueryData(key, 'manual')
  if (change === 'erase') client.getQueryCache().find({ queryKey: key })?.setState({ data: undefined })
  if (change === 'inactive') observed.stop()
  if (change === 'replace') { observed.stop(); client.removeQueries({ queryKey: root }); await observe().ready() }
  expect(proof.reusable(key)).toBe(false)
})
it('rejects a newly mounted target absent from the initial active inventory', async () => {
  const proof = trackFantasyRefresh(client, root)
  await observe().ready(); proof.stop()
  expect(proof.reusable(key)).toBe(false)
})
it('rejects canceled/reverted reads and failed refreshes', async () => {
  const load = vi.fn(async () => 'initial')
  const observed = observe(load); await observed.ready()
  load.mockImplementation(() => new Promise(() => {}))
  const proof = trackFantasyRefresh(client, root)
  const refresh = client.invalidateQueries({ queryKey: root })
  await client.cancelQueries({ queryKey: root }); await refresh
  expect(proof.reusable(key)).toBe(false)
  load.mockRejectedValue(new Error('failed'))
  await expect(client.invalidateQueries({ queryKey: root }, { throwOnError: true })).rejects.toThrow('failed')
  proof.stop(); expect(proof.reusable(key)).toBe(false)
})
it('requires the newer overlapping refresh and ignores late canceled data', async () => {
  let finishA: (value: string) => void = () => {}, finishB: (value: string) => void = () => {}
  const load = vi.fn(async () => 'initial')
  const observed = observe(load); await observed.ready()
  load.mockImplementationOnce(() => new Promise(resolve => { finishA = resolve }))
    .mockImplementationOnce(() => new Promise(resolve => { finishB = resolve }))
  const proof = trackFantasyRefresh(client, root)
  const first = client.invalidateQueries({ queryKey: root })
  const second = client.invalidateQueries({ queryKey: root })
  finishA('obsolete'); await Promise.resolve()
  expect(proof.reusable(key)).toBe(false)
  finishB('latest'); await Promise.all([first, second]); proof.stop()
  expect(client.getQueryData(key)).toBe('latest')
  expect(proof.reusable(key)).toBe(true)
})
