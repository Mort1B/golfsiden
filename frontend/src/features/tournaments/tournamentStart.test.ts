import { describe, expect, it } from 'vitest'
import { ApiHttpError } from '../../api/http'
import type { Round, Tournament, TournamentPlayerRoster } from '../../api/types'
import { tournamentStartFailure, tournamentStartReadiness } from './tournamentStart'

const tournament: Tournament = {
  id: '00000000-0000-0000-0000-000000000001',
  name: 'En turnering med et svært langt navn som fortsatt skal vises uten å endre reglene',
  description: '', start_date: '2026-09-01', end_date: '2026-09-04',
  number_of_rounds: 2, counted_rounds: 1, mandatory_round_id: null, status: 'draft', scoring_mode: 'combined',
  created_at: '2026-08-16T12:00:00Z', updated_at: '2026-08-16T12:00:00Z',
}

function round(number: number, status: Round['status'] = 'draft'): Round {
  return {
    id: `00000000-0000-0000-0000-00000000000${number}`,
    tournament_id: tournament.id, round_number: number, name: `Runde ${number}`,
    round_date: '2026-09-01', course_id: null, course_name: '', tee_id: null, tee_name: '',
    number_of_holes: 18, status, handicap_enabled: true, handicap_allowance_percent: 100,
    scoring_format: 'individual_stroke_play', created_at: tournament.created_at, updated_at: tournament.updated_at,
  }
}

const roster: TournamentPlayerRoster = {
  handicap_correction: { state: 'editable' },
  players: [{
    tournament_id: tournament.id, player_id: '00000000-0000-0000-0000-000000000010',
    display_name: 'En deltaker med et svært langt navn som må kunne brytes over flere linjer',
    player_active: true, tournament_handicap: 14.4, seed: null, status: 'active',
    created_at: tournament.created_at, updated_at: tournament.updated_at,
  }],
}

function readiness(overrides: Partial<Parameters<typeof tournamentStartReadiness>[0]> = {}) {
  return tournamentStartReadiness({
    tournament, rounds: [round(1), round(2)], roundsPending: false, roundsError: null,
    roster, rosterPending: false, rosterError: null, ...overrides,
  })
}

describe('tournament start presentation state', () => {
  it('requires a complete uniquely numbered draft plan and an active entrant', () => {
    expect(readiness()).toMatchObject({
      roundPlan: 'ready', draftRounds: 'ready', activeEntrant: 'ready', canStart: true,
    })
    expect(readiness({ rounds: [round(1), round(1)] })).toMatchObject({ roundPlan: 'missing', canStart: false })
    expect(readiness({ rounds: [round(1), round(2, 'open')] })).toMatchObject({ draftRounds: 'missing', canStart: false })
    const withdrawnRoster: TournamentPlayerRoster = {
      ...roster,
      players: roster.players.map((player) => ({ ...player, status: 'withdrawn' })),
    }
    expect(readiness({ roster: withdrawnRoster }))
      .toMatchObject({ activeEntrant: 'missing', canStart: false })
    const deactivatedRoster: TournamentPlayerRoster = {
      ...roster,
      players: roster.players.map((player) => ({ ...player, player_active: false })),
    }
    expect(readiness({ roster: deactivatedRoster }))
      .toMatchObject({ activeEntrant: 'missing', canStart: false })
  })

  it('fails closed while reads load or fail and after the tournament leaves draft', () => {
    expect(readiness({ rounds: undefined, roundsPending: true })).toMatchObject({ roundPlan: 'pending', canStart: false })
    expect(readiness({ roster: undefined, rosterError: new Error('En lang nettverksfeil'), rosterPending: false }))
      .toMatchObject({ activeEntrant: 'error', canStart: false })
    expect(readiness({ tournament: { ...tournament, status: 'active' } }).canStart).toBe(false)
  })

  it('maps lifecycle conflicts to precise authoritative refresh guidance', () => {
    expect(tournamentStartFailure(new ApiHttpError(409, 'tournament_start_stale', 'stale')))
      .toMatchObject({ refresh: 'tournament', message: expect.stringContaining('endret et annet sted') })
    expect(tournamentStartFailure(new ApiHttpError(409, 'tournament_start_not_ready', 'not ready')))
      .toMatchObject({ refresh: 'all', message: expect.stringContaining('runder og deltakere') })
    expect(tournamentStartFailure(new ApiHttpError(409, 'tournament_start_invalid_state', 'invalid')))
      .toMatchObject({ refresh: 'tournament', message: expect.stringContaining('gjeldende status') })
    expect(tournamentStartFailure(new Error('En lang og konkret feil fra nettverket')))
      .toEqual({ refresh: 'none', message: 'En lang og konkret feil fra nettverket' })
  })
})
