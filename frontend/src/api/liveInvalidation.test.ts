import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { describe, expect, it, vi } from 'vitest'
import { leaderboardKeys } from './leaderboards'
import { handleTournamentLiveSignal } from './liveInvalidation'
import { privateWorkspaceKeys } from './privateWorkspace'
import { scoringKeys } from './scorecards'

const owner = { type: 'player' as const, id: 'player-one' }

function seededClient() {
  const queryClient = new QueryClient()
  const keys = {
    roundLeaderboard: leaderboardKeys.round('user-one', 'round-one', 'gross'),
    tournamentLeaderboard: leaderboardKeys.tournament('user-one', 'tour-one', 'net'),
    completion: privateWorkspaceKeys.completion('user-one', 'round-one'),
    readCard: scoringKeys.read('user-one', 'round-one', owner),
    scoringCard: scoringKeys.scoring('user-one', 'round-one', owner),
    otherUserRead: scoringKeys.read('user-two', 'round-two', owner),
  }
  for (const key of Object.values(keys)) queryClient.setQueryData(key, { cached: true })
  return { queryClient, keys }
}

describe('visibility-safe live invalidation', () => {
  for (const signal of ['visibility', 'open'] as const) {
    it(`synchronously removes protected projections on ${signal} and preserves scoring caches`, async () => {
      const { queryClient, keys } = seededClient()

      const invalidation = handleTournamentLiveSignal(queryClient, 'user-one', signal)

      expect(queryClient.getQueryData(keys.roundLeaderboard)).toBeUndefined()
      expect(queryClient.getQueryData(keys.tournamentLeaderboard)).toBeUndefined()
      expect(queryClient.getQueryData(keys.completion)).toBeUndefined()
      expect(queryClient.getQueryData(keys.readCard)).toBeUndefined()
      expect(queryClient.getQueryData(keys.scoringCard)).toEqual({ cached: true })
      expect(queryClient.getQueryData(keys.otherUserRead)).toEqual({ cached: true })
      await invalidation
    })
  }

  it('keeps protected cache data for ordinary live events while retaining invalidation behavior', async () => {
    const { queryClient, keys } = seededClient()

    await handleTournamentLiveSignal(queryClient, 'user-one', 'score')

    expect(queryClient.getQueryData(keys.roundLeaderboard)).toEqual({ cached: true })
    expect(queryClient.getQueryData(keys.completion)).toEqual({ cached: true })
    expect(queryClient.getQueryData(keys.readCard)).toEqual({ cached: true })
    expect(queryClient.getQueryData(keys.scoringCard)).toEqual({ cached: true })
    expect(queryClient.getQueryState(keys.roundLeaderboard)?.isInvalidated).toBe(true)
  })

  it('authoritatively refetches an active projection after removing its cached data', async () => {
    const queryClient = new QueryClient()
    const queryKey = leaderboardKeys.round('user-one', 'round-one', 'gross')
    const queryFn = vi.fn(async () => ({ projection: 'authoritative' }))
    const observer = new QueryObserver(queryClient, {
      queryKey,
      queryFn,
      initialData: { projection: 'previous-full' },
      staleTime: Infinity,
    })
    const unsubscribe = observer.subscribe(() => undefined)

    const refresh = handleTournamentLiveSignal(queryClient, 'user-one', 'open')
    expect(queryClient.getQueryData(queryKey)).toBeUndefined()
    await refresh

    expect(queryFn).toHaveBeenCalledOnce()
    expect(queryClient.getQueryData(queryKey)).toEqual({ projection: 'authoritative' })
    unsubscribe()
  })

  it('clears mounted full data on disconnect and repopulates only after reconnect', async () => {
    const queryClient = new QueryClient()
    const readKey = scoringKeys.read('user-one', 'round-one', owner)
    const scoringKey = scoringKeys.scoring('user-one', 'round-one', owner)
    const queryFn = vi.fn(async () => ({ visibility: { mode: 'front_nine' } }))
    const observer = new QueryObserver(queryClient, {
      queryKey: readKey,
      queryFn,
      initialData: { visibility: { mode: 'full' }, holes: [{ hole_number: 18 }] },
      staleTime: Infinity,
    })
    const unsubscribe = observer.subscribe(() => undefined)
    queryClient.setQueryData(scoringKey, { projection: 'scoring', holes: 18 })

    const disconnected = handleTournamentLiveSignal(queryClient, 'user-one', 'error')

    expect(queryClient.getQueryData(readKey)).toBeUndefined()
    expect(observer.getCurrentResult()).toMatchObject({
      data: undefined,
      status: 'pending',
      fetchStatus: 'idle',
    })
    expect(queryClient.getQueryData(scoringKey)).toEqual({ projection: 'scoring', holes: 18 })
    await disconnected
    expect(queryFn).not.toHaveBeenCalled()

    await handleTournamentLiveSignal(queryClient, 'user-one', 'open')

    expect(queryFn).toHaveBeenCalledOnce()
    expect(observer.getCurrentResult().data).toEqual({ visibility: { mode: 'front_nine' } })
    expect(queryClient.getQueryData(scoringKey)).toEqual({ projection: 'scoring', holes: 18 })
    unsubscribe()
  })
})
