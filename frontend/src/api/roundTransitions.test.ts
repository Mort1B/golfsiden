import { afterEach, describe, expect, it, vi } from 'vitest'
import { decodeRoundTransition, roundLifecycleApi } from './roundLifecycle'
import { round, tournament } from '../features/tournaments/lifecycle/__tests__/fixtures'

afterEach(() => vi.unstubAllGlobals())

describe('round transition API', () => {
  it.each(['open', 'complete', 'lock'] as const)('sends CSRF for %s and decodes the resulting state', async (action) => {
    const status = action === 'complete' ? 'completed' : action === 'lock' ? 'locked' : 'open'
    const result = { ...round, status }
    const body = action === 'open' ? { round: result, handicap_snapshots: [], team_handicap_snapshots: [] } : result
    const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status: 200 }))
    vi.stubGlobal('fetch', fetch)
    expect(await roundLifecycleApi.transition(round.id, tournament.id, action, 'csrf-test')).toEqual(result)
    expect(fetch).toHaveBeenCalledWith(`/api/rounds/${round.id}/${action}`, expect.objectContaining({
      method: 'POST', body: '{}', headers: expect.objectContaining({ 'x-csrf-token': 'csrf-test' }),
    }))
  })
  it.each(['complete', 'lock'] as const)('rejects malformed and cross-target %s responses', (action) => {
    const status = action === 'complete' ? 'completed' : 'locked'
    expect(() => decodeRoundTransition({}, round.id, tournament.id, action)).toThrow()
    for (const patch of [{ id: tournament.id }, { tournament_id: round.id }, { status: 'open' }]) {
      expect(() => decodeRoundTransition({ ...round, status, ...patch }, round.id, tournament.id, action)).toThrow()
    }
  })
  it('rejects an opened round from another tournament', () => {
    expect(() => decodeRoundTransition({
      round: { ...round, status: 'open', tournament_id: round.id }, handicap_snapshots: [], team_handicap_snapshots: [],
    }, round.id, tournament.id, 'open')).toThrow()
  })
})
