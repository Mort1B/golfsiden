import { describe, expect, it } from 'vitest'
import { decodeOpenRoundResult, decodePairingValidation } from './roundLifecycle'

const roundId = '00000000-0000-0000-0000-000000000001'
const tournamentId = '00000000-0000-0000-0000-000000000002'
const playerId = '00000000-0000-0000-0000-000000000003'
const teamId = '00000000-0000-0000-0000-000000000004'

const openedRound = {
  id: roundId,
  tournament_id: tournamentId,
  round_number: 1,
  name: 'Foursomes',
  round_date: '2026-09-01',
  course_id: null,
  course_name: '',
  tee_id: null,
  tee_name: '',
  number_of_holes: 18,
  status: 'open',
  handicap_enabled: true,
  handicap_allowance_percent: 50,
  scoring_format: 'two_player_foursomes',
  created_at: '2026-08-24T10:00:00Z',
  updated_at: '2026-08-24T10:01:00Z',
}

describe('round lifecycle decoders', () => {
  it('decodes foursomes readiness and rejects unknown issue codes', () => {
    const response = {
      round_id: roundId,
      ready: false,
      issues: [{ code: 'invalid_foursomes_team_size', message: 'requires two' }],
      missing_players: [],
      ineligible_players: [],
      team_sizes: [{ team_id: teamId, team_name: 'Lag 1', player_count: 1 }],
      missing_flight_players: [],
      ineligible_flight_players: [],
      flight_sizes: [],
      legacy_individual_groups: [],
      split_teams: [],
    }
    expect(decodePairingValidation(response, roundId).issues[0]?.code)
      .toBe('invalid_foursomes_team_size')
    expect(() => decodePairingValidation({
      ...response,
      issues: [{ code: 'invalid_greensomes_team_size', message: 'unknown' }],
    }, roundId)).toThrow('issues[0].code')
  })

  it('decodes immutable member and team snapshots and enforces identities', () => {
    const response = {
      round: openedRound,
      handicap_snapshots: [{
        round_id: roundId,
        tournament_id: tournamentId,
        player_id: playerId,
        handicap_index: 12.4,
        course_handicap: 13,
        playing_handicap: 13,
        captured_at: '2026-08-24T10:01:00Z',
      }],
      team_handicap_snapshots: [{
        round_id: roundId,
        tournament_id: tournamentId,
        team_id: teamId,
        playing_handicap: 8,
        captured_at: '2026-08-24T10:01:00Z',
      }],
    }
    expect(decodeOpenRoundResult(response, roundId).team_handicap_snapshots[0]?.playing_handicap).toBe(8)
    expect(() => decodeOpenRoundResult({
      ...response,
      team_handicap_snapshots: [{ ...response.team_handicap_snapshots[0], round_id: tournamentId }],
    }, roundId)).toThrow('opening.identity')
    expect(() => decodeOpenRoundResult({
      ...response,
      round: { ...openedRound, scoring_format: 'greensomes' },
    }, roundId)).toThrow('scoring_format')
  })
})
