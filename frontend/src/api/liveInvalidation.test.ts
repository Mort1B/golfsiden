import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { describe, expect, it, vi } from 'vitest'
import { leaderboardKeys } from './leaderboards'
import { handleTournamentLiveSignal } from './liveInvalidation'
import { privateWorkspaceKeys } from './privateWorkspace'
import { scoringKeys } from './scorecards'
import { authKeys } from './auth'
import { session } from '../features/tournaments/lifecycle/__tests__/fixtures'

const owner = { type: 'player' as const, id: 'player-one' }

it('deduplicates return checks and revalidates identity before refetching private data', async () => {
  const client = new QueryClient()
  let finish: (value: typeof session | null) => void = () => undefined
  const sessionRead = vi.fn(() => new Promise<typeof session | null>(resolve => { finish = resolve }))
  const auth = new QueryObserver(client, { queryKey: authKeys.session, queryFn: sessionRead, initialData: session, staleTime: Infinity })
  const privateRead = vi.fn(async () => 'fresh')
  const card = new QueryObserver(client, { queryKey: scoringKeys.scoring(session.user_id, 'round', owner),
    queryFn: privateRead, initialData: 'cached', staleTime: Infinity })
  const stopAuth = auth.subscribe(() => undefined); const stopCard = card.subscribe(() => undefined)
  try {
    const first = handleTournamentLiveSignal(client, session.user_id, 'resume')
    expect(handleTournamentLiveSignal(client, session.user_id, 'resume')).toBe(first)
    expect(privateRead).not.toHaveBeenCalled()
    await vi.waitFor(() => expect(sessionRead).toHaveBeenCalledTimes(1))
    finish(session); await first
    expect(sessionRead).toHaveBeenCalledOnce()
    expect(privateRead).toHaveBeenCalledOnce()
    const next = handleTournamentLiveSignal(client, session.user_id, 'resume')
    await vi.waitFor(() => expect(sessionRead).toHaveBeenCalledTimes(2))
    finish(null); await next
    expect(privateRead).toHaveBeenCalledOnce()
  } finally { stopAuth(); stopCard(); client.clear() }
})

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

it('refreshes score-dependent views without refetching unchanged setup on a score event', async () => {
  const client = new QueryClient()
  const changed = [
    leaderboardKeys.round('user', 'round', 'gross'),
    leaderboardKeys.round('user', 'round', 'net'),
    leaderboardKeys.tournament('user', 'tour', 'gross'),
    leaderboardKeys.tournament('user', 'tour', 'net'),
    privateWorkspaceKeys.completion('user', 'round'),
    scoringKeys.read('user', 'round', owner),
    scoringKeys.scoring('user', 'round', owner),
  ]
  const unchanged = [
    ['private-workspace', 'user', 'tournaments', 'list'],
    ['private-workspace', 'user', 'tournaments', 'tour', 'rounds'],
    ['private-workspace', 'user', 'tournaments', 'tour', 'players'],
    ['private-workspace', 'user', 'rounds', 'round', 'teams'],
    ['private-workspace', 'user', 'rounds', 'round', 'detail'],
    ['private-workspace', 'user', 'rounds', 'round', 'pairing-validation'],
    privateWorkspaceKeys.scoreAccess('user', 'round'),
    privateWorkspaceKeys.invitations('user', 'tour'),
    ['auth', 'session'],
    scoringKeys.read('another-user', 'round', owner),
  ]
  const calls = [...changed, ...unchanged].map(queryKey => {
    const queryFn = vi.fn(async () => ({ refreshed: true }))
    const observer = new QueryObserver(client, { queryKey, queryFn, initialData: { refreshed: false }, staleTime: Infinity })
    return { queryFn, unsubscribe: observer.subscribe(() => undefined) }
  })
  try {
    await handleTournamentLiveSignal(client, 'user', 'score')
    expect(calls.slice(0, changed.length).every(item => item.queryFn.mock.calls.length === 1)).toBe(true)
    expect(calls.slice(changed.length).every(item => item.queryFn.mock.calls.length === 0)).toBe(true)
    await handleTournamentLiveSignal(client, 'user', 'round')
    // Lifecycle updates still revalidate setup as well as all affected scores.
    expect(calls.slice(changed.length, -2).every(item => item.queryFn.mock.calls.length === 1)).toBe(true)
    expect(calls.slice(-2).every(item => item.queryFn.mock.calls.length === 0)).toBe(true)
  } finally { calls.forEach(item => item.unsubscribe()) }
})
