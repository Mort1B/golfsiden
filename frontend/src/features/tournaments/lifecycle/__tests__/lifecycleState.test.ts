import { describe, expect, it } from 'vitest'
import { QueryClient, QueryObserver } from '@tanstack/react-query'
import { leaderboardKeys } from '../../../../api/leaderboards'
import { scoringKeys } from '../../../../api/scorecards'
import { privateWorkspaceKeys, clearPrivateWorkspace } from '../../../../api/privateWorkspace'
import { tournamentKeys } from '../../../../api/tournaments'
import { pairingKeys } from '../../../../api/pairings'
import { roundLifecycleKeys } from '../../../../api/roundLifecycle'
import { isLifecycleTarget, lifecycleReady, reconcileLifecycleQueries, roundAction, roundManagementUrl } from '../lifecycleState'
import { completion, opening, round, session, tournament } from './fixtures'

describe('round lifecycle state', () => {
  it('offers only the legal next transition', () => {
    expect(['draft', 'open', 'completed', 'locked'].map((status) => roundAction(status as typeof round.status)))
      .toEqual(['open', 'complete', 'lock', null])
  })
  it('requires exact and authoritative readiness', () => {
    expect(lifecycleReady(round, opening)).toBe(true)
    expect(lifecycleReady(round, { ...opening, round_id: tournament.id })).toBe(false)
    const openRound = { ...round, status: 'open' as const }
    expect(lifecycleReady(openRound, undefined, completion())).toBe(true)
    expect(lifecycleReady(openRound, undefined, completion('completed'))).toBe(false)
    expect(lifecycleReady(openRound, undefined, { ...completion(), visibility: { mode: 'front_nine' }, ready_to_complete: null })).toBe(false)
    expect(lifecycleReady({ ...round, status: 'locked' }, undefined, completion('locked'))).toBe(false)
  })
  it('preserves selected round when linking to setup', () => {
    const url = new URL(roundManagementUrl(tournament.id, round.id, 'courses'), 'http://localhost')
    expect(url.searchParams.get('round')).toBe(round.id)
    expect(url.hash).toBe('#courses')
  })
  it('invalidates exact lifecycle consumers without touching another user or tournament', async () => {
    const client = new QueryClient()
    const user = session.user_id
    const owner = { type: 'player' as const, id: session.player_id ?? '' }
    const affected = [
      tournamentKeys.round(user, round.id), tournamentKeys.rounds(user, tournament.id),
      tournamentKeys.players(user, tournament.id), tournamentKeys.detail(user, tournament.id),
      tournamentKeys.mine(user), tournamentKeys.list(user), pairingKeys.detail(user, round.id),
      roundLifecycleKeys.validation(user, round.id), privateWorkspaceKeys.completion(user, round.id),
      privateWorkspaceKeys.scoreAccess(user, round.id), scoringKeys.read(user, round.id, owner),
      scoringKeys.scoring(user, round.id, owner), leaderboardKeys.round(user, round.id, 'gross'),
      leaderboardKeys.round(user, round.id, 'net'), leaderboardKeys.tournament(user, tournament.id, 'gross'),
      leaderboardKeys.tournament(user, tournament.id, 'net'),
    ]
    const untouched = [tournamentKeys.round('other-user', round.id), tournamentKeys.detail(user, 'other-trip'),
      leaderboardKeys.round(user, 'other-round', 'gross'), ['auth', 'session'],
      ['private-workspace', user, 'course-provider', tournament.id]]
    for (const key of [...affected, ...untouched]) client.setQueryData(key, true)
    const invalidate = () => client.invalidateQueries({ predicate: (query) => isLifecycleTarget(query.queryKey, user, tournament.id, round.id) })
    await invalidate()
    for (const key of affected) expect(client.getQueryState(key)?.isInvalidated).toBe(true)
    for (const key of untouched) expect(client.getQueryState(key)?.isInvalidated).toBe(false)
    clearPrivateWorkspace(client)
    await invalidate()
    expect(client.getQueryCache().findAll({ queryKey: privateWorkspaceKeys.root })).toHaveLength(0)
    client.clear()
  })
  it('accepts a newer SSE refetch replacing reconciliation without reporting a false failure', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const queryKey = tournamentKeys.round(session.user_id, round.id)
    client.setQueryData(queryKey, round)
    let release: ((value: typeof round) => void) | undefined
    let calls = 0
    const observer = new QueryObserver(client, { queryKey, staleTime: Infinity, queryFn: () => {
      calls++
      if (calls === 1) return new Promise<typeof round>((resolve) => { release = resolve })
      return Promise.resolve({ ...round, status: 'open' as const })
    } })
    const unsubscribe = observer.subscribe(() => undefined)
    const reconcile = reconcileLifecycleQueries(client, session.user_id, tournament.id, round.id)
    expect(calls).toBe(1)
    await client.invalidateQueries({ queryKey })
    await expect(reconcile).resolves.toBeUndefined()
    expect(client.getQueryData(queryKey)).toMatchObject({ status: 'open' })
    release?.(round)
    unsubscribe()
    client.clear()
  })
  it('supersedes a pending pre-commit snapshot with a new post-outcome read', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const queryKey = tournamentKeys.round(session.user_id, round.id)
    client.setQueryData(queryKey, round)
    let release: ((value: typeof round) => void) | undefined
    let calls = 0
    const observer = new QueryObserver(client, { queryKey, staleTime: Infinity, queryFn: () => {
      calls++
      if (calls === 1) return new Promise<typeof round>((resolve) => { release = resolve })
      return Promise.resolve({ ...round, status: 'open' as const })
    } })
    const unsubscribe = observer.subscribe(() => undefined)
    const preCommitRead = client.refetchQueries({ queryKey })
    expect(calls).toBe(1)
    await reconcileLifecycleQueries(client, session.user_id, tournament.id, round.id)
    expect(calls).toBe(2)
    expect(client.getQueryData(queryKey)).toMatchObject({ status: 'open' })
    release?.(round)
    await preCommitRead
    expect(client.getQueryData(queryKey)).toMatchObject({ status: 'open' })
    unsubscribe()
    client.clear()
  })
})
