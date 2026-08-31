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
      required_counted_rounds: 3,
      current_round_id: null,
      included_round_ids: [],
      entries: [],
    }, seedTournamentId, 'gross')

    expect(round.round_id).toBe(seedRoundId)
    expect(tournament.tournament_id).toBe(seedTournamentId)
  })

  it('decodes the complete best-N tournament contribution contract', () => {
    const playerId = '00000000-0000-0000-0000-000000001001'
    const teamId = '00000000-0000-0000-0000-000000003001'
    const secondRoundId = '00000000-0000-0000-0000-000000004002'
    const thirdRoundId = '00000000-0000-0000-0000-000000004003'
    const currentRoundId = '00000000-0000-0000-0000-000000004004'
    const response = decodeTournamentLeaderboard({
      tournament_id: seedTournamentId,
      metric: 'net',
      required_counted_rounds: 2,
      current_round_id: currentRoundId,
      included_round_ids: [seedRoundId, secondRoundId, thirdRoundId],
      entries: [{
        position: 1,
        tied: false,
        player_id: playerId,
        display_name: 'Spiller En',
        status: 'active',
        completed_rounds: 3,
        counted_contributions: 2,
        eligible: true,
        gross_total: 151,
        net_total: 141,
        par_total: 144,
        score_to_par: -3,
        contributions: [{
          round_id: seedRoundId,
          owner: { type: 'team', id: teamId },
          owner_name: 'Historisk lag',
          gross_total: 76,
          net_total: 70,
          par_total: 72,
          score_to_par: -2,
          counted: true,
        }, {
          round_id: secondRoundId,
          owner: { type: 'player', id: playerId },
          owner_name: 'Spiller En',
          gross_total: 75,
          net_total: 71,
          par_total: 72,
          score_to_par: -1,
          counted: true,
        }, {
          round_id: thirdRoundId,
          owner: { type: 'player', id: playerId },
          owner_name: 'Spiller En',
          gross_total: 74,
          net_total: 73,
          par_total: 72,
          score_to_par: 1,
          counted: false,
        }],
        current_team: {
          round_id: currentRoundId,
          team_id: teamId,
          team_name: 'Nåværende lag',
        },
      }],
    }, seedTournamentId, 'net')

    expect(response.required_counted_rounds).toBe(2)
    expect(response.entries[0]).toMatchObject({
      counted_contributions: 2,
      eligible: true,
      par_total: 144,
      score_to_par: -3,
    })
    expect(response.entries[0]?.contributions[0]).toEqual({
      round_id: seedRoundId,
      owner: { type: 'team', id: teamId },
      owner_name: 'Historisk lag',
      gross_total: 76,
      net_total: 70,
      par_total: 72,
      score_to_par: -2,
      counted: true,
    })
  })

  it('rejects incomplete or malformed best-N facts', () => {
    const contribution = {
      round_id: seedRoundId,
      owner: { type: 'player', id: '00000000-0000-0000-0000-000000001001' },
      owner_name: 'Spiller En',
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      counted: true,
    }
    const leaderboardEntry = {
      position: null,
      tied: false,
      player_id: '00000000-0000-0000-0000-000000001001',
      display_name: 'Spiller En',
      status: 'active',
      completed_rounds: 1,
      counted_contributions: 1,
      eligible: true,
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      contributions: [contribution],
      current_team: null,
    }
    const base = {
      tournament_id: seedTournamentId,
      metric: 'gross',
      required_counted_rounds: 1,
      current_round_id: null,
      included_round_ids: [seedRoundId],
      entries: [leaderboardEntry],
    }

    expect(() => decodeTournamentLeaderboard(
      { ...base, required_counted_rounds: 0 },
      seedTournamentId,
      'gross',
    )).toThrow('required_counted_rounds')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...leaderboardEntry, eligible: undefined }],
    }, seedTournamentId, 'gross')).toThrow('eligible')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{
        ...leaderboardEntry,
        contributions: [{ ...contribution, owner: { type: 'group', id: seedRoundId } }],
      }],
    }, seedTournamentId, 'gross')).toThrow('owner.type')
  })

  it('rejects incoherent best-N counts, eligibility, and round attribution', () => {
    const playerId = '00000000-0000-0000-0000-000000001001'
    const contribution = {
      round_id: seedRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller En',
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      counted: true,
    }
    const entry = {
      position: 1,
      tied: false,
      player_id: playerId,
      display_name: 'Spiller En',
      status: 'active',
      completed_rounds: 1,
      counted_contributions: 1,
      eligible: true,
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      contributions: [contribution],
      current_team: null,
    }
    const base = {
      tournament_id: seedTournamentId,
      metric: 'gross',
      required_counted_rounds: 1,
      current_round_id: null,
      included_round_ids: [seedRoundId],
      entries: [entry],
    }

    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, completed_rounds: 2 }],
    }, seedTournamentId, 'gross')).toThrow('completed_rounds')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, counted_contributions: 0 }],
    }, seedTournamentId, 'gross')).toThrow('counted_contributions')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, eligible: false }],
    }, seedTournamentId, 'gross')).toThrow('eligible')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{
        ...entry,
        contributions: [contribution, { ...contribution, counted: false }],
        completed_rounds: 2,
      }],
    }, seedTournamentId, 'gross')).toThrow('round_id')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{
        ...entry,
        contributions: [{ ...contribution, round_id: '00000000-0000-0000-0000-000000004099' }],
      }],
    }, seedTournamentId, 'gross')).toThrow('round_id')
  })

  it('rejects incoherent included, current-round, and player-owner identities', () => {
    const playerId = '00000000-0000-0000-0000-000000001001'
    const otherPlayerId = '00000000-0000-0000-0000-000000001002'
    const currentRoundId = '00000000-0000-0000-0000-000000004002'
    const contribution = {
      round_id: seedRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller En',
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      counted: true,
    }
    const entry = {
      position: 1,
      tied: false,
      player_id: playerId,
      display_name: 'Spiller En',
      status: 'active',
      completed_rounds: 1,
      counted_contributions: 1,
      eligible: true,
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      contributions: [contribution],
      current_team: null,
    }
    const base = {
      tournament_id: seedTournamentId,
      metric: 'gross',
      required_counted_rounds: 1,
      current_round_id: currentRoundId,
      included_round_ids: [seedRoundId],
      entries: [entry],
    }

    expect(() => decodeTournamentLeaderboard({
      ...base,
      included_round_ids: [seedRoundId, seedRoundId],
    }, seedTournamentId, 'gross')).toThrow('included_round_ids')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      current_round_id: seedRoundId,
    }, seedTournamentId, 'gross')).toThrow('current_round_id')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{
        ...entry,
        current_team: { round_id: seedRoundId, team_id: seedRoundId, team_name: 'Lag En' },
      }],
    }, seedTournamentId, 'gross')).toThrow('current_team.round_id')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      current_round_id: null,
      entries: [{
        ...entry,
        current_team: { round_id: currentRoundId, team_id: seedRoundId, team_name: 'Lag En' },
      }],
    }, seedTournamentId, 'gross')).toThrow('current_team.round_id')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{
        ...entry,
        contributions: [{ ...contribution, owner: { type: 'player', id: otherPlayerId } }],
      }],
    }, seedTournamentId, 'gross')).toThrow('owner.id')
  })

  it('rejects incoherent contribution and selected aggregate totals', () => {
    const playerId = '00000000-0000-0000-0000-000000001001'
    const contribution = {
      round_id: seedRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller En',
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      counted: true,
    }
    const entry = {
      position: 1,
      tied: false,
      player_id: playerId,
      display_name: 'Spiller En',
      status: 'active',
      completed_rounds: 1,
      counted_contributions: 1,
      eligible: true,
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      contributions: [contribution],
      current_team: null,
    }
    const base = {
      tournament_id: seedTournamentId,
      metric: 'gross',
      required_counted_rounds: 1,
      current_round_id: null,
      included_round_ids: [seedRoundId],
      entries: [entry],
    }

    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{
        ...entry,
        contributions: [{ ...contribution, score_to_par: 1 }],
      }],
    }, seedTournamentId, 'gross')).toThrow('contributions[0].score_to_par')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, gross_total: 73 }],
    }, seedTournamentId, 'gross')).toThrow('gross_total')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, net_total: 71 }],
    }, seedTournamentId, 'gross')).toThrow('net_total')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, par_total: 73, score_to_par: -1 }],
    }, seedTournamentId, 'gross')).toThrow('par_total')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, score_to_par: 1 }],
    }, seedTournamentId, 'gross')).toThrow('score_to_par')
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
