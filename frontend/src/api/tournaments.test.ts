import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  decodeMyTournaments,
  decodeTournament,
  decodeTournamentHandicapCorrection,
  decodeTournamentPlayerRoster,
  tournamentApi,
  tournamentKeys,
  withCreatedTournament,
} from './tournaments'
import type { Tournament } from './types'

const tournament: Tournament = {
  id: '00000000-0000-0000-0000-000000000001',
  name: 'Tur',
  description: '',
  start_date: '2026-09-01',
  end_date: '2026-09-02',
  number_of_rounds: 2,
  counted_rounds: 1,
  status: 'draft',
  scoring_mode: 'combined',
  created_at: '2026-08-16T12:00:00Z',
  updated_at: '2026-08-16T12:00:00Z',
}

const player = {
  tournament_id: tournament.id,
  player_id: '00000000-0000-0000-0000-000000000002',
  display_name: 'Morten',
  tournament_handicap: 14.4,
  seed: null,
  status: 'active',
  created_at: '2026-08-16T12:00:00Z',
  updated_at: '2026-08-16T12:00:00Z',
}

afterEach(() => vi.unstubAllGlobals())

describe('tournament memberships', () => {
  it('decodes contextual role and linked player identity', () => {
    expect(decodeMyTournaments([{ tournament, role: 'admin', player_id: '00000000-0000-0000-0000-000000000002' }])).toEqual([{
      tournament,
      role: 'admin',
      player_id: '00000000-0000-0000-0000-000000000002',
    }])
  })

  it('rejects global or unknown roles', () => {
    expect(() => decodeMyTournaments([{ tournament, role: 'platform_admin', player_id: null }])).toThrow('role')
  })

  it('requires counted rounds within the configured tournament range', () => {
    expect(decodeTournament(tournament).counted_rounds).toBe(1)
    expect(() => decodeTournament({ ...tournament, counted_rounds: 0 })).toThrow('counted_rounds')
    expect(() => decodeTournament({ ...tournament, counted_rounds: 3 })).toThrow('counted_rounds')
    expect(() => decodeTournament({ ...tournament, counted_rounds: undefined })).toThrow('counted_rounds')
  })

  it('uses stable hierarchical cache keys', () => {
    expect(tournamentKeys.root('user-one')).toEqual(['private-workspace', 'user-one', 'tournaments'])
    expect(tournamentKeys.mine('user-one')).toEqual(['private-workspace', 'user-one', 'tournaments', 'mine'])
    expect(tournamentKeys.rounds('user-one', tournament.id)).toEqual([
      'private-workspace', 'user-one', 'tournaments', tournament.id, 'rounds',
    ])
    expect(tournamentKeys.round('user-two', 'round-one')).not.toEqual(
      tournamentKeys.round('user-one', 'round-one'),
    )
  })

  it('adds a created tournament without retaining a duplicate', () => {
    const created = { ...tournament, name: 'Ny tur' }
    expect(withCreatedTournament([tournament], created)).toEqual([created])
  })

  it('decodes editable and locked tournament handicap states', () => {
    expect(decodeTournamentPlayerRoster({ handicap_correction: { state: 'editable' }, players: [player] }))
      .toEqual({ handicap_correction: { state: 'editable' }, players: [player] })
    expect(decodeTournamentPlayerRoster({ handicap_correction: { state: 'locked', reason: 'round_opened' }, players: [] }))
      .toEqual({ handicap_correction: { state: 'locked', reason: 'round_opened' }, players: [] })
    expect(() => decodeTournamentPlayerRoster({ handicap_correction: { state: 'locked', reason: 'round_completed' }, players: [] }))
      .toThrow('handicap_correction')
  })

  it('posts an audited correction and validates its response', async () => {
    const correction = {
      player: { ...player, tournament_handicap: 13.8 },
      audit: {
        id: '00000000-0000-0000-0000-000000000003',
        tournament_id: tournament.id,
        player_id: player.player_id,
        handicap_index: 13.8,
        effective_from: '2026-08-16T13:00:00Z',
        changed_by: '00000000-0000-0000-0000-000000000004',
        reason: 'Feil ved påmelding',
        created_at: '2026-08-16T13:00:00Z',
      },
    }
    expect(decodeTournamentHandicapCorrection(correction)).toEqual(correction)
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(correction), { status: 201 }))
    vi.stubGlobal('fetch', fetchMock)

    await tournamentApi.correctHandicap(tournament.id, player.player_id, {
      handicap_index: 13.8,
      reason: 'Feil ved påmelding',
    }, 'csrf-token')

    expect(fetchMock).toHaveBeenCalledWith(
      `/api/tournaments/${tournament.id}/players/${player.player_id}/handicap-corrections`,
      {
        credentials: 'include',
        method: 'POST',
        headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf-token' },
        body: JSON.stringify({ handicap_index: 13.8, reason: 'Feil ved påmelding' }),
      },
    )
  })

  it('patches counted rounds with the optimistic tournament timestamp', async () => {
    const updated = { ...tournament, counted_rounds: 2, updated_at: '2026-08-16T13:00:00Z' }
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(updated), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(tournamentApi.updateCountedRounds(tournament.id, {
      counted_rounds: 2,
      expected_tournament_updated_at: tournament.updated_at,
    }, 'csrf-token')).resolves.toEqual(updated)

    expect(fetchMock).toHaveBeenCalledWith(`/api/tournaments/${tournament.id}/counted-rounds`, {
      credentials: 'include',
      method: 'PATCH',
      headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf-token' },
      body: JSON.stringify({
        counted_rounds: 2,
        expected_tournament_updated_at: tournament.updated_at,
      }),
    })
  })
})
