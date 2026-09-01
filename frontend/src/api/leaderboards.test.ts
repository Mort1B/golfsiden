import { describe, expect, it } from 'vitest'
import { decodeRoundLeaderboard, decodeTournamentLeaderboard, leaderboardKeys, validateTournamentLeaderboardRounds } from './leaderboards'
import type { Round } from './types'

const seedTournamentId = '00000000-0000-0000-0000-000000002001'
const seedRoundId = '00000000-0000-0000-0000-000000004001'
const fullVisibility = { mode: 'full', observed_at: '2026-09-01T10:00:00Z', hidden_until: null }

function roundFixture(id: string, roundNumber: number, status: Round['status']): Round {
  return {
    id,
    tournament_id: seedTournamentId,
    round_number: roundNumber,
    name: `Runde ${roundNumber}`,
    round_date: '2026-09-01',
    course_id: null,
    course_name: 'Testbane',
    tee_id: null,
    tee_name: 'Gul',
    number_of_holes: 18,
    status,
    handicap_enabled: true,
    handicap_allowance_percent: 100,
    scoring_format: 'individual_stroke_play',
    created_at: '2026-09-01T10:00:00Z',
    updated_at: '2026-09-01T10:00:00Z',
  }
}

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
      visible_hole_count: 18,
      visibility: fullVisibility,
      entries: [],
    }, seedRoundId, seedTournamentId, 'net')

    const tournament = decodeTournamentLeaderboard({
      tournament_id: seedTournamentId,
      metric: 'gross',
      required_counted_rounds: 3,
      mandatory_round_id: null,
      current_round_id: null,
      included_round_ids: [],
      visibility: fullVisibility,
      entries: [],
    }, seedTournamentId, 'gross')

    expect(round.round_id).toBe(seedRoundId)
    expect(tournament.tournament_id).toBe(seedTournamentId)
    expect(validateTournamentLeaderboardRounds(tournament, [])).toBe(tournament)
    expect(() => validateTournamentLeaderboardRounds({
      ...tournament,
      mandatory_round_id: seedRoundId,
    }, [])).toThrow('mandatory_round_id round identity')
  })

  it('ties included and current identities to exact loaded round lifecycle states', () => {
    const lockedRoundId = '00000000-0000-0000-0000-000000004002'
    const lowerOpenId = '00000000-0000-0000-0000-000000004003'
    const highestOpenId = '00000000-0000-0000-0000-000000004004'
    const draftRoundId = '00000000-0000-0000-0000-000000004005'
    const completed = roundFixture(seedRoundId, 1, 'completed')
    const locked = roundFixture(lockedRoundId, 2, 'locked')
    const lowerOpen = roundFixture(lowerOpenId, 3, 'open')
    const highestOpen = roundFixture(highestOpenId, 3, 'open')
    const draft = roundFixture(draftRoundId, 4, 'draft')
    const rounds = [completed, locked, lowerOpen, highestOpen, draft]
    const leaderboard = decodeTournamentLeaderboard({
      tournament_id: seedTournamentId,
      metric: 'gross',
      required_counted_rounds: 2,
      mandatory_round_id: null,
      current_round_id: highestOpenId,
      included_round_ids: [seedRoundId, lockedRoundId],
      visibility: fullVisibility,
      entries: [],
    }, seedTournamentId, 'gross')

    expect(validateTournamentLeaderboardRounds(leaderboard, rounds)).toBe(leaderboard)
    expect(() => validateTournamentLeaderboardRounds(
      { ...leaderboard, current_round_id: lowerOpenId },
      rounds,
    )).toThrow('current_round_id status')
    expect(() => validateTournamentLeaderboardRounds(
      { ...leaderboard, current_round_id: null },
      rounds,
    )).toThrow('current_round_id status')
    expect(() => validateTournamentLeaderboardRounds(
      { ...leaderboard, current_round_id: seedRoundId },
      rounds,
    )).toThrow('current_round_id status')
    expect(() => validateTournamentLeaderboardRounds(
      { ...leaderboard, current_round_id: draftRoundId },
      rounds,
    )).toThrow('current_round_id status')
    expect(() => validateTournamentLeaderboardRounds(
      { ...leaderboard, current_round_id: '00000000-0000-0000-0000-000000004099' },
      rounds,
    )).toThrow('current_round_id status')
    expect(() => validateTournamentLeaderboardRounds(
      { ...leaderboard, included_round_ids: [lowerOpenId] },
      rounds,
    )).toThrow('included_round_ids status')
    expect(() => validateTournamentLeaderboardRounds(
      { ...leaderboard, included_round_ids: [draftRoundId] },
      rounds,
    )).toThrow('included_round_ids status')
    expect(validateTournamentLeaderboardRounds(
      { ...leaderboard, current_round_id: null },
      [completed, locked, draft],
    )).toEqual({ ...leaderboard, current_round_id: null })
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
      mandatory_round_id: seedRoundId,
      current_round_id: currentRoundId,
      included_round_ids: [seedRoundId, secondRoundId, thirdRoundId],
      visibility: fullVisibility,
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
          provisional: false,
          holes_scored: 18,
          number_of_holes: 18,
          gross_total: 76,
          net_total: 70,
          par_total: 72,
          score_to_par: -2,
          counted: true,
          mandatory: true,
        }, {
          round_id: secondRoundId,
          owner: { type: 'player', id: playerId },
          owner_name: 'Spiller En',
          provisional: false,
          holes_scored: 18,
          number_of_holes: 18,
          gross_total: 75,
          net_total: 71,
          par_total: 72,
          score_to_par: -1,
          counted: true,
          mandatory: false,
        }, {
          round_id: thirdRoundId,
          owner: { type: 'player', id: playerId },
          owner_name: 'Spiller En',
          provisional: false,
          holes_scored: 18,
          number_of_holes: 18,
          gross_total: 74,
          net_total: 73,
          par_total: 72,
          score_to_par: 1,
          counted: false,
          mandatory: false,
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
      provisional: false,
      holes_scored: 18,
      number_of_holes: 18,
      gross_total: 76,
      net_total: 70,
      par_total: 72,
      score_to_par: -2,
      counted: true,
      mandatory: true,
    })
  })

  it('rejects incomplete or malformed best-N facts', () => {
    const contribution = {
      round_id: seedRoundId,
      owner: { type: 'player', id: '00000000-0000-0000-0000-000000001001' },
      owner_name: 'Spiller En',
      provisional: false,
      holes_scored: 18,
      number_of_holes: 18,
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      counted: true,
      mandatory: true,
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
      mandatory_round_id: seedRoundId,
      current_round_id: null,
      included_round_ids: [seedRoundId],
      visibility: fullVisibility,
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
      mandatory_round_id: undefined,
    }, seedTournamentId, 'gross')).toThrow('mandatory_round_id')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{
        ...leaderboardEntry,
        contributions: [{ ...contribution, mandatory: undefined }],
      }],
    }, seedTournamentId, 'gross')).toThrow('mandatory')
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
      provisional: false,
      holes_scored: 18,
      number_of_holes: 18,
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      counted: true,
      mandatory: false,
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
      mandatory_round_id: null,
      current_round_id: null,
      included_round_ids: [seedRoundId],
      visibility: fullVisibility,
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
      mandatory_round_id: seedRoundId,
      entries: [{
        ...entry,
        contributions: [{ ...contribution, mandatory: true, counted: false }],
        counted_contributions: 0,
        eligible: false,
        gross_total: 0,
        net_total: 0,
        par_total: 0,
      }],
    }, seedTournamentId, 'gross')).toThrow('counted_contributions')
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

    expect(decodeTournamentLeaderboard({
      ...base,
      required_counted_rounds: 2,
      mandatory_round_id: '00000000-0000-0000-0000-000000004002',
      entries: [{
        ...entry,
        counted_contributions: 1,
        eligible: false,
      }],
    }, seedTournamentId, 'gross').entries[0]?.eligible).toBe(false)
  })

  it('rejects incoherent included, current-round, and player-owner identities', () => {
    const playerId = '00000000-0000-0000-0000-000000001001'
    const otherPlayerId = '00000000-0000-0000-0000-000000001002'
    const currentRoundId = '00000000-0000-0000-0000-000000004002'
    const contribution = {
      round_id: seedRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller En',
      provisional: false,
      holes_scored: 18,
      number_of_holes: 18,
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      counted: true,
      mandatory: false,
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
      mandatory_round_id: null,
      current_round_id: currentRoundId,
      included_round_ids: [seedRoundId],
      visibility: fullVisibility,
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
      provisional: false,
      holes_scored: 18,
      number_of_holes: 18,
      gross_total: 72,
      net_total: 70,
      par_total: 72,
      score_to_par: 0,
      counted: true,
      mandatory: false,
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
      mandatory_round_id: null,
      current_round_id: null,
      included_round_ids: [seedRoundId],
      visibility: fullVisibility,
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

  it('keeps completed qualification separate from a selected provisional result', () => {
    const playerId = '00000000-0000-0000-0000-000000001001'
    const currentRoundId = '00000000-0000-0000-0000-000000004002'
    const completed = {
      round_id: seedRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller En',
      provisional: false,
      holes_scored: 18,
      number_of_holes: 18,
      gross_total: 73,
      net_total: 70,
      par_total: 72,
      score_to_par: 1,
      counted: true,
      mandatory: false,
    }
    const provisional = {
      round_id: currentRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller En',
      provisional: true,
      holes_scored: 9,
      number_of_holes: 18,
      gross_total: 34,
      net_total: 30,
      par_total: 36,
      score_to_par: -2,
      counted: true,
      mandatory: false,
    }
    const entry = {
      position: 1,
      tied: false,
      player_id: playerId,
      display_name: 'Spiller En',
      status: 'active',
      completed_rounds: 1,
      counted_contributions: 1,
      eligible: false,
      gross_total: 107,
      net_total: 100,
      par_total: 108,
      score_to_par: -1,
      contributions: [completed, provisional],
      current_team: null,
    }
    const base = {
      tournament_id: seedTournamentId,
      metric: 'gross',
      required_counted_rounds: 2,
      mandatory_round_id: null,
      current_round_id: currentRoundId,
      included_round_ids: [seedRoundId],
      visibility: fullVisibility,
      entries: [entry],
    }

    const decoded = decodeTournamentLeaderboard(base, seedTournamentId, 'gross')
    expect(decoded.entries[0]).toMatchObject({
      completed_rounds: 1,
      counted_contributions: 1,
      eligible: false,
      position: 1,
    })
    expect(decoded.entries[0]?.contributions[1]).toMatchObject({
      provisional: true,
      holes_scored: 9,
      counted: true,
    })

    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, completed_rounds: 2 }],
    }, seedTournamentId, 'gross')).toThrow('completed_rounds')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, position: 2 }],
    }, seedTournamentId, 'gross')).toThrow('position')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, contributions: [completed, { ...provisional, holes_scored: 0 }] }],
    }, seedTournamentId, 'gross')).toThrow('round_id')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, contributions: [completed, { ...provisional, holes_scored: 19 }] }],
    }, seedTournamentId, 'gross')).toThrow('holes_scored')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, contributions: [{ ...completed, holes_scored: 17 }, provisional] }],
    }, seedTournamentId, 'gross')).toThrow('holes_scored')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      current_round_id: null,
    }, seedTournamentId, 'gross')).toThrow('round_id')
    expect(() => decodeTournamentLeaderboard({
      ...base,
      entries: [{ ...entry, contributions: [completed, provisional, { ...provisional, round_id: '00000000-0000-0000-0000-000000004003' }] }],
    }, seedTournamentId, 'gross')).toThrow('contributions')
  })

  it('validates metric-specific displayed cutoffs and open mandatory qualification', () => {
    const playerId = '00000000-0000-0000-0000-000000001001'
    const currentRoundId = '00000000-0000-0000-0000-000000004002'
    const completed = {
      round_id: seedRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller',
      provisional: false,
      holes_scored: 18,
      number_of_holes: 18,
      gross_total: 73,
      net_total: 70,
      par_total: 72,
      score_to_par: 1,
      counted: true,
      mandatory: false,
    }
    const live = {
      round_id: currentRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller',
      provisional: true,
      holes_scored: 8,
      number_of_holes: 18,
      gross_total: 38,
      net_total: 29,
      par_total: 36,
      score_to_par: 2,
      counted: false,
      mandatory: false,
    }
    const common = {
      tournament_id: seedTournamentId,
      required_counted_rounds: 1,
      mandatory_round_id: null,
      current_round_id: currentRoundId,
      included_round_ids: [seedRoundId],
      visibility: fullVisibility,
    }
    const grossEntry = {
      position: 1,
      tied: false,
      player_id: playerId,
      display_name: 'Spiller',
      status: 'active',
      completed_rounds: 1,
      counted_contributions: 1,
      eligible: true,
      gross_total: 73,
      net_total: 70,
      par_total: 72,
      score_to_par: 1,
      contributions: [completed, live],
      current_team: null,
    }
    expect(decodeTournamentLeaderboard({ ...common, metric: 'gross', entries: [grossEntry] }, seedTournamentId, 'gross')
      .entries[0]?.gross_total).toBe(73)

    const netCompleted = { ...completed, counted: false, score_to_par: -2 }
    const netLive = { ...live, counted: true, score_to_par: -7 }
    const netEntry = {
      ...grossEntry,
      gross_total: 38,
      net_total: 29,
      par_total: 36,
      score_to_par: -7,
      contributions: [netCompleted, netLive],
    }
    expect(decodeTournamentLeaderboard({ ...common, metric: 'net', entries: [netEntry] }, seedTournamentId, 'net')
      .entries[0]?.net_total).toBe(29)

    const mandatoryLive = { ...live, counted: true, mandatory: true }
    const mandatoryEntry = {
      ...grossEntry,
      eligible: false,
      gross_total: 111,
      net_total: 99,
      par_total: 108,
      score_to_par: 3,
      contributions: [completed, mandatoryLive],
    }
    expect(decodeTournamentLeaderboard({
      ...common,
      metric: 'gross',
      required_counted_rounds: 2,
      mandatory_round_id: currentRoundId,
      entries: [mandatoryEntry],
    }, seedTournamentId, 'gross').entries[0]?.eligible).toBe(false)
  })

  it('rejects hidden final progress and a wrong deterministic cutoff against configured rounds', () => {
    const playerId = '00000000-0000-0000-0000-000000001001'
    const currentRoundId = '00000000-0000-0000-0000-000000004002'
    const live = {
      round_id: currentRoundId,
      owner: { type: 'player', id: playerId },
      owner_name: 'Spiller',
      provisional: true,
      holes_scored: 10,
      number_of_holes: 18,
      gross_total: 40,
      net_total: 38,
      par_total: 40,
      score_to_par: 0,
      counted: true,
      mandatory: false,
    }
    const decoded = decodeTournamentLeaderboard({
      tournament_id: seedTournamentId,
      metric: 'gross',
      required_counted_rounds: 1,
      mandatory_round_id: null,
      current_round_id: currentRoundId,
      included_round_ids: [],
      visibility: { mode: 'front_nine', observed_at: '2026-09-01T10:00:00Z', hidden_until: null },
      entries: [{
        position: 1,
        tied: false,
        player_id: playerId,
        display_name: 'Spiller',
        status: 'active',
        completed_rounds: 0,
        counted_contributions: 0,
        eligible: false,
        gross_total: 40,
        net_total: 38,
        par_total: 40,
        score_to_par: 0,
        contributions: [live],
        current_team: null,
      }],
    }, seedTournamentId, 'gross')
    expect(() => validateTournamentLeaderboardRounds(decoded, [
      roundFixture(seedRoundId, 1, 'completed'),
      roundFixture(currentRoundId, 2, 'open'),
    ])).toThrow('hidden final progress')
  })

  it('rejects a response for a different leaderboard identity', () => {
    expect(() => decodeRoundLeaderboard({
      round_id: '6b68e090-0ed8-4f4e-9a9d-f4689836b201',
      tournament_id: seedTournamentId,
      status: 'open',
      scoring_format: 'individual_stroke_play',
      metric: 'gross',
      number_of_holes: 18,
      visible_hole_count: 18,
      visibility: fullVisibility,
      entries: [],
    }, seedRoundId, seedTournamentId, 'gross')).toThrow('leaderboard.identity')
    expect(() => decodeRoundLeaderboard({
      round_id: seedRoundId,
      tournament_id: '00000000-0000-0000-0000-000000002099',
      status: 'open',
      scoring_format: 'individual_stroke_play',
      metric: 'gross',
      number_of_holes: 18,
      visible_hole_count: 18,
      visibility: fullVisibility,
      entries: [],
    }, seedRoundId, seedTournamentId, 'gross')).toThrow('leaderboard.identity')
  })

  it('accepts foursomes and rejects unknown scoring formats', () => {
    const response = {
      round_id: seedRoundId,
      tournament_id: seedTournamentId,
      status: 'open',
      scoring_format: 'two_player_foursomes',
      metric: 'gross',
      number_of_holes: 18,
      visible_hole_count: 18,
      visibility: fullVisibility,
      entries: [],
    }
    expect(decodeRoundLeaderboard(response, seedRoundId, seedTournamentId, 'gross').scoring_format)
      .toBe('two_player_foursomes')
    expect(() => decodeRoundLeaderboard({ ...response, scoring_format: 'greensomes' }, seedRoundId, seedTournamentId, 'gross'))
      .toThrow('scoring_format')
  })

  it('accepts only nullable hidden state in a front-nine round projection', () => {
    const entry = {
      position: 1,
      tied: false,
      owner: { type: 'player', id: '00000000-0000-0000-0000-000000001001' },
      owner_name: 'Spiller',
      members: [],
      holes_scored: 1,
      number_of_holes: 18,
      complete: null,
      confirmed: null,
      playing_handicap: 9,
      gross_total: 5,
      net_total: 4,
      par_played: 4,
      score_to_par: 1,
    }
    const response = {
      round_id: seedRoundId,
      tournament_id: seedTournamentId,
      status: 'open',
      scoring_format: 'individual_stroke_play',
      metric: 'gross',
      number_of_holes: 18,
      visible_hole_count: 9,
      visibility: { mode: 'front_nine', observed_at: '2026-09-01T10:00:00Z', hidden_until: null },
      entries: [entry],
    }
    expect(decodeRoundLeaderboard(response, seedRoundId, seedTournamentId, 'gross').visible_hole_count).toBe(9)
    expect(() => decodeRoundLeaderboard({
      ...response,
      entries: [{ ...entry, complete: false, confirmed: false }],
    }, seedRoundId, seedTournamentId, 'gross')).toThrow('complete')
    expect(() => decodeRoundLeaderboard({ ...response, visible_hole_count: 18 }, seedRoundId, seedTournamentId, 'gross'))
      .toThrow('visibility')
  })

  it('rejects duplicate or format-incoherent round result identities', () => {
    const entry = {
      position: 1,
      tied: false,
      owner: { type: 'player', id: '00000000-0000-0000-0000-000000001001' },
      owner_name: 'Spiller',
      members: [],
      holes_scored: 1,
      number_of_holes: 1,
      complete: true,
      confirmed: true,
      playing_handicap: 0,
      gross_total: 4,
      net_total: 4,
      par_played: 4,
      score_to_par: 0,
    }
    const response = {
      round_id: seedRoundId,
      tournament_id: seedTournamentId,
      status: 'open',
      scoring_format: 'individual_stroke_play',
      metric: 'gross',
      number_of_holes: 1,
      visible_hole_count: 1,
      visibility: fullVisibility,
      entries: [entry],
    }
    expect(() => decodeRoundLeaderboard({ ...response, entries: [entry, entry] }, seedRoundId, seedTournamentId, 'gross'))
      .toThrow('owner')
    expect(() => decodeRoundLeaderboard({
      ...response,
      scoring_format: 'team_scramble',
    }, seedRoundId, seedTournamentId, 'gross')).toThrow('owner.type')
  })
})
