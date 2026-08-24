import { describe, expect, it } from 'vitest'
import type { RoundPairings } from '../../../api/pairings'
import {
  assignEntrant,
  draftFromPairings,
  moveMember,
  pairingsFingerprint,
  scheduleFlightOptions,
  selectScheduleFlight,
} from './draft'
import { legacyConversionReplacement, replacementFromDraft } from './serialization'
import { validateDraft } from './validation'

const ids = {
  round: '00000000-0000-0000-0000-000000000001', tournament: '00000000-0000-0000-0000-000000000002',
  one: '00000000-0000-0000-0000-000000000003', two: '00000000-0000-0000-0000-000000000004',
  team: '00000000-0000-0000-0000-000000000005', flight: '00000000-0000-0000-0000-000000000006',
  otherFlight: '00000000-0000-0000-0000-000000000007',
  inactive: '00000000-0000-0000-0000-000000000008',
}

function group(id: string, members: string[], scheduled = false) {
  return { id, name: 'Gruppe', starting_hole: scheduled ? 4 : null, tee_time: scheduled ? '08:30:00' : null,
    created_at: '2026-08-24T10:00:00Z', updated_at: '2026-08-24T10:00:00Z',
    members: members.map((player_id, display_order) => ({ player_id, display_name: player_id, display_order })) }
}

function aggregate(format: RoundPairings['scoring_format'] = 'team_scramble'): RoundPairings {
  return {
    round_id: ids.round, tournament_id: ids.tournament, status: 'draft', scoring_format: format,
    updated_at: '2026-08-24T10:00:00Z',
    active_entrants: [ids.one, ids.two].map((player_id) => ({ player_id, display_name: player_id, status: 'active', player_active: true })),
    inactive_entrants: [], teams: [group(ids.team, [ids.one, ids.two], true)],
    flights: [group(ids.flight, [ids.one, ids.two])], legacy_individual_groups: [],
  }
}

describe('pairing draft', () => {
  it('moves and orders members independently in teams and flights', () => {
    let draft = draftFromPairings({ ...aggregate(), teams: [], flights: [{ ...group(ids.flight, [ids.one, ids.two]), name: 'Flight 1' }] })
    draft = assignEntrant(draft, 'team', ids.one, null)
    expect(draft.flights[0]?.memberIds).toEqual([ids.one, ids.two])
    draft = moveMember(draft, 'flight', ids.flight, ids.two, -1)
    expect(draft.flights[0]?.memberIds).toEqual([ids.two, ids.one])
  })

  it('requires an explicit exact-member schedule transfer and preserves ordered identities', () => {
    const pairings = aggregate()
    let draft = draftFromPairings(pairings)
    expect(scheduleFlightOptions(draft.teams[0]!, draft.flights).map((flight) => flight.id)).toEqual([ids.flight])
    expect(validateDraft(draft, pairings).blocking[0]).toContain('eksplisitt')
    draft = selectScheduleFlight(draft, ids.team, ids.flight)
    expect(validateDraft(draft, pairings).blocking).toEqual([])
    const replacement = replacementFromDraft(draft, pairings.scoring_format)
    expect(replacement.teams[0]).toMatchObject({
      id: ids.team, members: [{ player_id: ids.one }, { player_id: ids.two }], schedule_flight_id: ids.flight,
    })
    expect(replacement.flights[0]).toMatchObject({ starting_hole: 4, tee_time: '08:30:00' })
  })

  it('blocks saving if an explicitly transferred schedule is later changed', () => {
    const pairings = aggregate()
    const selected = selectScheduleFlight(draftFromPairings(pairings), ids.team, ids.flight)
    const changed = { ...selected, flights: selected.flights.map((flight) => ({ ...flight, teeTime: '09:00', teeTimeEdited: true })) }

    expect(validateDraft(changed, pairings).blocking).toEqual([
      'Velg eksplisitt hvilken flight som overtar starttiden fra Gruppe.',
    ])
  })

  it('preserves exact server time precision until the field is edited', () => {
    const preciseFlight = { ...group(ids.flight, [ids.one]), tee_time: '08:30:15.123' }
    const pairings = { ...aggregate('individual_stroke_play'), teams: [], flights: [preciseFlight] }
    const draft = draftFromPairings(pairings)
    expect(replacementFromDraft(draft, pairings.scoring_format).flights[0]?.tee_time).toBe('08:30:15.123')

    const edited = { ...draft, flights: draft.flights.map((flight) => ({
      ...flight, teeTime: '09:45', teeTimeEdited: true,
    })) }
    expect(replacementFromDraft(edited, pairings.scoring_format).flights[0]?.tee_time).toBe('09:45:00')
  })

  it('copies an exact fractional team schedule into an explicitly selected flight', () => {
    const pairings = {
      ...aggregate(),
      teams: [{ ...group(ids.team, [ids.one, ids.two], true), tee_time: '08:30:15.123' }],
    }
    const draft = selectScheduleFlight(draftFromPairings(pairings), ids.team, ids.flight)

    expect(validateDraft(draft, pairings).blocking).toEqual([])
    expect(replacementFromDraft(draft, pairings.scoring_format).flights[0]?.tee_time).toBe('08:30:15.123')
  })

  it('restores complete prior schedules when a transfer is cleared or moved', () => {
    const originalTarget = {
      ...group(ids.otherFlight, []), name: 'Flight 2', starting_hole: 10, tee_time: '12:45:30.123',
    }
    const pairings = { ...aggregate(), flights: [...aggregate().flights, originalTarget] }
    const initial = draftFromPairings(pairings)

    const selectedA = selectScheduleFlight(initial, ids.team, ids.flight)
    const clearedA = selectScheduleFlight(selectedA, ids.team, null)
    const clearedRequest = replacementFromDraft(clearedA, pairings.scoring_format)
    expect(clearedRequest.flights[0]).toMatchObject({ starting_hole: null, tee_time: null })

    let reassigned = assignEntrant(selectedA, 'flight', ids.one, ids.otherFlight)
    reassigned = assignEntrant(reassigned, 'flight', ids.two, ids.otherFlight)
    const selectedB = selectScheduleFlight(reassigned, ids.team, ids.otherFlight)
    const movedRequest = replacementFromDraft(selectedB, pairings.scoring_format)
    expect(movedRequest.flights[0]).toMatchObject({ starting_hole: null, tee_time: null })
    expect(movedRequest.flights[1]).toMatchObject({ starting_hole: 4, tee_time: '08:30:00' })

    const clearedB = selectScheduleFlight(selectedB, ids.team, null)
    expect(replacementFromDraft(clearedB, pairings.scoring_format).flights[1]).toMatchObject({
      starting_hole: 10, tee_time: '12:45:30.123',
    })
  })

  it('detects same-timestamp authoritative roster changes', () => {
    const original = aggregate()
    const changed = { ...original, active_entrants: original.active_entrants.slice(0, 1) }

    expect(changed.updated_at).toBe(original.updated_at)
    expect(pairingsFingerprint(changed)).not.toBe(pairingsFingerprint(original))
    expect(draftFromPairings(original).sourceFingerprint).toBe(pairingsFingerprint(original))
  })

  it('blocks stored inactive members until the remove-only cleanup removes them', () => {
    const inactiveEntrant = { player_id: ids.inactive, display_name: 'Tidligere spiller', status: 'withdrawn' as const, player_active: true }
    const pairings = {
      ...aggregate('individual_stroke_play'),
      teams: [],
      inactive_entrants: [inactiveEntrant],
      flights: [{ ...group(ids.flight, [ids.one, ids.inactive]), name: 'Flight 1' }],
    }
    const draft = draftFromPairings(pairings)
    expect(validateDraft(draft, pairings).blocking[0]).toContain('ikke lenger er aktiv')

    const cleaned = assignEntrant(draft, 'flight', ids.inactive, null)
    expect(validateDraft(cleaned, pairings).blocking).toEqual([])
    expect(cleaned.flights[0]?.memberIds).toEqual([ids.one])
  })

  it('copies legacy individual groups exactly into generated flights for the first save', () => {
    const preservedId = '00000000-0000-0000-0000-000000000007'
    const legacy = { ...aggregate('individual_stroke_play'), teams: [], flights: [{ ...group(preservedId, []), name: 'Eksisterende' }], legacy_individual_groups: [group(ids.team, [ids.two, ids.one], true)] }
    const request = legacyConversionReplacement(legacy, () => ids.flight)
    expect(request.teams).toEqual([])
    expect(request.flights[0]).toMatchObject({ id: preservedId, name: 'Eksisterende' })
    expect(request.flights[1]).toMatchObject({ id: ids.flight, starting_hole: 4, tee_time: '08:30:00', members: [{ player_id: ids.two }, { player_id: ids.one }] })
    expect(request.legacy_conversions).toEqual([{ team_id: ids.team, flight_id: ids.flight }])
  })

  it('allows incomplete rosters while reporting readiness work separately from blockers', () => {
    const pairings = aggregate()
    const draft = { ...draftFromPairings(pairings), teams: [], flights: [] }
    const validation = validateDraft(draft, pairings)
    expect(validation.blocking).toEqual([])
    expect(validation.unresolved).toEqual([
      '2 aktive spillere mangler flight.', '2 aktive spillere mangler lag.',
    ])
  })

  it('treats foursomes as an exact two-player team format', () => {
    const pairings = aggregate('two_player_foursomes')
    const draft = draftFromPairings(pairings)
    expect(replacementFromDraft(draft, pairings.scoring_format).teams).toHaveLength(1)
    expect(validateDraft(draft, pairings).unresolved).toEqual([])
    const firstTeam = draft.teams[0]
    if (!firstTeam) throw new Error('expected team fixture')
    const incomplete = { ...draft, teams: [{ ...firstTeam, memberIds: [ids.one] }] }
    expect(validateDraft(incomplete, pairings).unresolved)
      .toContain('1 foursomes-lag har ikke nøyaktig to spillere.')
  })
})
