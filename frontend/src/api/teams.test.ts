import { afterEach, describe, expect, it, vi } from 'vitest'
import { decodeTeams, teamApi } from './teams'

const roundId = '00000000-0000-0000-0000-000000000001'
const tournamentId = '00000000-0000-0000-0000-000000000002'
const playerId = '00000000-0000-0000-0000-000000000003'
const team = {
  id: '00000000-0000-0000-0000-000000000004',
  round_id: roundId,
  tournament_id: tournamentId,
  name: 'Lag 1',
  starting_hole: 1,
  tee_time: '08:30:00',
  members: [{ player_id: playerId, display_name: 'Spiller', display_order: 0 }],
}

afterEach(() => vi.unstubAllGlobals())

describe('team API', () => {
  it('decodes only teams belonging to the requested tournament and round', () => {
    expect(decodeTeams([team], roundId, tournamentId)).toEqual([team])
    expect(() => decodeTeams([{ ...team, round_id: '00000000-0000-0000-0000-000000000099' }], roundId, tournamentId))
      .toThrow('identity')
    expect(() => decodeTeams([{ ...team, tournament_id: '00000000-0000-0000-0000-000000000099' }], roundId, tournamentId))
      .toThrow('identity')
  })

  it('rejects duplicate team and member identities', () => {
    expect(() => decodeTeams([team, team], roundId, tournamentId)).toThrow('duplicate')
    expect(() => decodeTeams([{ ...team, members: [team.members[0], team.members[0]] }], roundId, tournamentId))
      .toThrow('duplicate')
    expect(() => decodeTeams([
      team,
      { ...team, id: '00000000-0000-0000-0000-000000000005' },
    ], roundId, tournamentId)).toThrow('assigned twice')
    expect(() => decodeTeams([{ ...team, tee_time: '8:30' }], roundId, tournamentId)).toThrow('tee_time')
  })

  it('decodes the network response before returning it', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify([
      { ...team, tournament_id: '00000000-0000-0000-0000-000000000099' },
    ]), { status: 200 })))

    await expect(teamApi.list(roundId, tournamentId)).rejects.toThrow('identity')
  })
})
