import { describe, expect, it } from 'vitest'
import { decodeRoundLeaderboard, decodeTournamentLeaderboard, leaderboardKeys } from './leaderboards'

const seedTournamentId = '00000000-0000-0000-0000-000000002001'
const seedRoundId = '00000000-0000-0000-0000-000000004001'

describe('leaderboard decoders', () => {
  it('isolates leaderboard caches by session user', () => {
    expect(leaderboardKeys.round('user-one', seedRoundId, 'net')).toEqual([
      'private-workspace', 'user-one', 'leaderboards', 'round', seedRoundId, 'net',
    ])
    expect(leaderboardKeys.tournament('user-two', seedTournamentId, 'gross')).toEqual([
      'private-workspace', 'user-two', 'leaderboards', 'tournament', seedTournamentId, 'gross',
    ])
  })

  it('accepts the canonical zero-version UUIDs used by development seed data', () => {
    const round = decodeRoundLeaderboard({
      round_id: seedRoundId,
      tournament_id: seedTournamentId,
      status: 'draft',
      scoring_format: 'team_scramble',
      metric: 'net',
      number_of_holes: 18,
      entries: [],
    }, seedRoundId, 'net')

    const tournament = decodeTournamentLeaderboard({
      tournament_id: seedTournamentId,
      metric: 'gross',
      current_round_id: null,
      included_round_ids: [],
      entries: [],
    }, seedTournamentId, 'gross')

    expect(round.round_id).toBe(seedRoundId)
    expect(tournament.tournament_id).toBe(seedTournamentId)
  })

  it('rejects a response for a different leaderboard identity', () => {
    expect(() => decodeRoundLeaderboard({
      round_id: '6b68e090-0ed8-4f4e-9a9d-f4689836b201',
      tournament_id: seedTournamentId,
      status: 'open',
      scoring_format: 'individual_stroke_play',
      metric: 'gross',
      number_of_holes: 18,
      entries: [],
    }, seedRoundId, 'gross')).toThrow('leaderboard.identity')
  })

  it('accepts foursomes and rejects unknown scoring formats', () => {
    const response = {
      round_id: seedRoundId,
      tournament_id: seedTournamentId,
      status: 'open',
      scoring_format: 'two_player_foursomes',
      metric: 'gross',
      number_of_holes: 18,
      entries: [],
    }
    expect(decodeRoundLeaderboard(response, seedRoundId, 'gross').scoring_format)
      .toBe('two_player_foursomes')
    expect(() => decodeRoundLeaderboard({ ...response, scoring_format: 'greensomes' }, seedRoundId, 'gross'))
      .toThrow('scoring_format')
  })
})
