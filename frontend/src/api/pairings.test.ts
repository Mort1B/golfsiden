import { afterEach, describe, expect, it, vi } from 'vitest'
import { decodeRoundPairings, pairingApi, pairingKeys } from './pairings'

const roundId = '00000000-0000-0000-0000-000000000001'
const tournamentId = '00000000-0000-0000-0000-000000000002'
const playerId = '00000000-0000-0000-0000-000000000003'
const groupId = '00000000-0000-0000-0000-000000000004'

const response = {
  round_id: roundId, tournament_id: tournamentId, status: 'draft', scoring_format: 'team_scramble',
  updated_at: '2026-08-24T10:00:00Z',
  active_entrants: [{ player_id: playerId, display_name: 'Et svært langt spillernavn', status: 'active', player_active: true }],
  inactive_entrants: [],
  teams: [{ id: groupId, name: 'Lag 1', starting_hole: null, tee_time: null,
    created_at: '2026-08-24T09:00:00Z', updated_at: '2026-08-24T09:00:00Z',
    members: [{ player_id: playerId, display_name: 'Et svært langt spillernavn', display_order: 0 }] }],
  flights: [], legacy_individual_groups: [],
}

afterEach(() => vi.unstubAllGlobals())

describe('pairing API', () => {
  it('decodes the aggregate and rejects invalid finite states and schedules', () => {
    expect(decodeRoundPairings(response).teams[0]?.members[0]?.player_id).toBe(playerId)
    expect(() => decodeRoundPairings({ ...response, status: 'opening' })).toThrow('status')
    const team = response.teams[0]
    if (!team) throw new Error('Expected team fixture')
    expect(() => decodeRoundPairings({ ...response, teams: [{ ...team, tee_time: '8:30' }] })).toThrow('tee_time')
    expect(decodeRoundPairings({ ...response, scoring_format: 'two_player_foursomes' }).scoring_format)
      .toBe('two_player_foursomes')
    expect(() => decodeRoundPairings({ ...response, scoring_format: 'greensomes' })).toThrow('scoring_format')
  })

  it('roots its key by user and sends the complete replacement with CSRF', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      void input
      void init
      return new Response(JSON.stringify(response), { status: 200 })
    })
    vi.stubGlobal('fetch', fetchMock)
    const replacement = {
      expected_round_updated_at: response.updated_at,
      teams: [{ id: groupId, name: 'Lag 1', members: [{ player_id: playerId }], schedule_flight_id: null }],
      flights: [], legacy_conversions: [],
    }

    await pairingApi.replace(roundId, replacement, 'csrf-value')

    expect(pairingKeys.detail('user-one', roundId).slice(0, 2)).toEqual(['private-workspace', 'user-one'])
    expect(fetchMock.mock.calls[0]?.[0]).toBe(`/api/rounds/${roundId}/pairings`)
    expect(fetchMock.mock.calls[0]?.[1]).toMatchObject({
      method: 'PUT', headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf-value' },
      body: JSON.stringify(replacement),
    })
  })

  it('rejects a response for a different round identity', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({
      ...response, round_id: '00000000-0000-0000-0000-000000000099',
    }), { status: 200 })))
    await expect(pairingApi.get(roundId)).rejects.toThrow('identity')
  })
})
