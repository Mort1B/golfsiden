import { expect, it } from 'vitest'
import { QueryClient } from '@tanstack/react-query'
import { handleTournamentLiveSignal } from '../liveInvalidation'
import { matchKeys } from '../matchPlay'
import { matchIds } from './fixtures'
it('match signals invalidate dependent results while visibility errors remove read projections only', async () => {
  const client = new QueryClient(), user = matchIds.user
  const keys = [matchKeys.read(user, matchIds.round, matchIds.match), matchKeys.list(user, matchIds.round), matchKeys.table(user, matchIds.tournament), matchKeys.completion(user, matchIds.round)]
  const scoring = matchKeys.scoring(user, matchIds.round, matchIds.match), other = matchKeys.read(matchIds.second, matchIds.round, matchIds.match)
  for (const key of [...keys, scoring, other]) client.setQueryData(key, { secret: 'value' })
  await handleTournamentLiveSignal(client, user, 'match')
  for (const key of [...keys, scoring]) expect(client.getQueryState(key)?.isInvalidated).toBe(true)
  expect(client.getQueryState(other)?.isInvalidated).toBe(false)
  await handleTournamentLiveSignal(client, user, 'error')
  for (const key of keys) expect(client.getQueryData(key)).toBeUndefined()
  expect(client.getQueryData(scoring)).toEqual({ secret: 'value' })
  expect(client.getQueryData(other)).toEqual({ secret: 'value' })
  client.clear()
})
