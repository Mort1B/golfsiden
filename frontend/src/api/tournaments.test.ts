import { describe, expect, it } from 'vitest'
import { decodeMyTournaments, tournamentKeys, withCreatedTournament } from './tournaments'
import type { Tournament } from './types'

const tournament: Tournament = {
  id: '00000000-0000-0000-0000-000000000001',
  name: 'Tur',
  description: '',
  start_date: '2026-09-01',
  end_date: '2026-09-02',
  number_of_rounds: 2,
  status: 'draft',
  scoring_mode: 'combined',
  created_at: '2026-08-16T12:00:00Z',
  updated_at: '2026-08-16T12:00:00Z',
}

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

  it('uses stable hierarchical cache keys', () => {
    expect(tournamentKeys.mineRoot).toEqual(['tournaments', 'mine'])
    expect(tournamentKeys.mine('user-one')).toEqual(['tournaments', 'mine', 'user-one'])
    expect(tournamentKeys.rounds(tournament.id)).toEqual(['tournaments', tournament.id, 'rounds'])
  })

  it('adds a created tournament without retaining a duplicate', () => {
    const created = { ...tournament, name: 'Ny tur' }
    expect(withCreatedTournament([tournament], created)).toEqual([created])
  })
})
