import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { ApiHttpError } from './http'
import { denyPrivateResults, loadPrivateResult } from './privateResults'
import { tournamentKeys } from './tournaments'
import { leaderboardKeys } from './leaderboards'
import { scoringKeys } from './scorecards'
import { matchKeys } from './matchPlay'
const scope = { userId: 'user', tournamentId: 'trip', roundId: 'round' }
const denied = new ApiHttpError(403, 'forbidden', 'Access denied')
const source = tournamentKeys.rounds('user', 'trip')
function setup() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  client.setQueryData(source, [{ id: 'round', tournament_id: 'trip' }])
  return client
}
it('erases read projections across formats and metrics, preserves other accounts/trips and scoring state', () => {
  const client = setup(), owner = { type: 'player' as const, id: 'player' }
  const erased = [leaderboardKeys.round('user','round','gross'), leaderboardKeys.round('user','round','net'), leaderboardKeys.tournament('user','trip','gross'), scoringKeys.read('user','round',owner), matchKeys.table('user','trip'), matchKeys.list('user','round'), matchKeys.read('user','round','match')]
  client.setQueryData(tournamentKeys.rounds('user', 'other'), [{ id: 'other-round', tournament_id: 'other' }])
  const preserved = [leaderboardKeys.round('user','other-round','gross'), scoringKeys.read('user','other-round',owner), leaderboardKeys.round('other','round','gross'), leaderboardKeys.tournament('user','other','gross'), scoringKeys.scoring('user','round',owner), matchKeys.scoring('user','round','match')]
  for (const key of [...erased, ...preserved]) client.setQueryData(key, { private: 'retained' })
  denyPrivateResults(client, scope, source, denied)
  for (const key of erased) expect(client.getQueryData(key)).toBeUndefined()
  for (const key of preserved) expect(client.getQueryData(key)).toEqual({ private: 'retained' })
})
it('cancels a delayed sibling success without reverting erased data on observer unmount', async () => {
  const client = setup(), key = leaderboardKeys.round('user','round','net')
  client.setQueryData(key, 'old private result')
  let release: (value: string) => void = () => undefined
  const pending = new Promise<string>(resolve => { release = resolve })
  const observer = new QueryObserver(client, { queryKey: key, queryFn: context => loadPrivateResult(client, scope, key, context.signal, () => pending) })
  const unsubscribe = observer.subscribe(() => undefined)
  denyPrivateResults(client, scope, source, denied)
  unsubscribe(); release('late private result'); await pending; await Promise.resolve()
  expect(client.getQueryData(key)).toBeUndefined()
  await expect(client.fetchQuery({ queryKey: key, queryFn: async () => { throw new ApiHttpError(500, 'unavailable', 'Temporary failure') } })).rejects.toThrow()
  expect(client.getQueryData(key)).toBeUndefined()
  expect(await client.fetchQuery({ queryKey: key, queryFn: async () => 'fresh permitted result' })).toBe('fresh permitted result')
})
it('ignores late denials from cancelled reads after a fresh success', async () => {
  const client = setup(), key = leaderboardKeys.round('user','round','net')
  let reject: (value: unknown) => void = () => undefined
  const pending = new Promise<string>((_resolve, fail) => { reject = fail })
  const request = client.fetchQuery({ queryKey: key, queryFn: context => loadPrivateResult(client, scope, key, context.signal, () => pending) }).catch(() => undefined)
  await client.cancelQueries({ queryKey: key }); client.setQueryData(key, 'fresh permitted')
  reject(denied); await request; await Promise.resolve()
  expect(client.getQueryData(key)).toBe('fresh permitted')
})
it('clears cards and pending reads even after rounds metadata disappeared', async () => {
  const client = setup(), owner = { type: 'player' as const, id: 'player' }
  const boardKey = leaderboardKeys.round('user','round','net'), cardKey = scoringKeys.read('user','round',owner)
  client.setQueryData(boardKey, { tournament_id: 'trip', round_id: 'round' })
  client.setQueryData(cardKey, { round_id: 'round', private: 'old card' })
  client.removeQueries({ queryKey: source })
  denyPrivateResults(client, { userId: 'user', tournamentId: 'trip' }, source, denied)
  expect(client.getQueryData(cardKey)).toBeUndefined()
  let release: (value: string) => void = () => undefined
  const late = new Promise<string>(resolve => { release = resolve })
  const pending = client.fetchQuery({ queryKey: cardKey, queryFn: () => late }).catch(() => undefined)
  denyPrivateResults(client, { userId: 'user', tournamentId: 'trip' }, source, denied)
  release('late card'); await pending
  expect(client.getQueryData(cardKey)).toBeUndefined()
})
it('source observer unmount cannot revert to its pre-denial private snapshot', async () => {
  const client = setup()
  const observer = new QueryObserver(client, { queryKey: source,
    queryFn: context => loadPrivateResult(client, scope, source, context.signal, async () => { throw denied }) })
  let unsubscribe: () => void = () => undefined
  unsubscribe = observer.subscribe(result => { if (result.error) unsubscribe() })
  await observer.refetch(); await Promise.resolve()
  expect(client.getQueryData(source)).toBeUndefined()
  expect(client.getQueryState(source)?.error).toBe(denied)
  unsubscribe()
})
