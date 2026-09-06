import { describe, expect, it } from 'vitest'
import type { RoundPairings } from '../../api/pairings'
import type { RoundCompletionValidation } from '../../api/scorecards'
import { flightProgress } from './flightProgress'

import { group, pairings, progress } from './flightProgressFixtures'

describe('flight progress mapping', () => {
  it('counts preserved individual owners without active-roster or schedule inference', () => {
    const result = flightProgress(pairings(), progress())
    expect(result[0]).toMatchObject({ scored: 20, required: 36, completed: 1, confirmed: 1 })
    expect(result[1]).toMatchObject({ scored: 4, required: 36, completed: 0 })
  })
  it.each(['team_scramble', 'two_player_foursomes'] as const)('counts shared %s cards once, including multiple teams per flight', (scoring_format) => {
    const roster = { ...pairings(), scoring_format, flights: [group('f', ['a', 'b', 'c', 'd'])], teams: [group('t1', ['a', 'b']), group('t2', ['c', 'd'])] }
    const cards = { ...progress(), owners: progress().owners.slice(0, 2).map((owner, i) => ({ ...owner, owner: { type: 'team' as const, id: `t${i + 1}` } })) }
    expect(flightProgress(roster, cards)[0]).toMatchObject({ scored: 20, required: 36, completed: 1, confirmed: 1 })
  })
  it.each(['open', 'completed', 'locked'] as const)('never infers hidden completeness from %s status or nine visible holes', (status) => {
    const cards: RoundCompletionValidation = { ...progress(), status, visibility: { mode: 'front_nine' }, ready_to_complete: null, ready_to_lock: null,
      owners: progress().owners.map((owner) => ({ ...owner, holes_scored: 9, required_holes: 9, complete: null, confirmed: null })) }
    expect(flightProgress({ ...pairings(), status }, cards)[0]).toMatchObject({ scored: 18, required: 18, completed: null, confirmed: null })
  })
  it('rejects historical owners with no stored flights instead of inferring from legacy groups', () => {
    expect(() => flightProgress({ ...pairings(), flights: [], legacy_individual_groups: [group('old', ['a', 'b', 'c', 'd'])] }, progress())).toThrow(/Eldre runder/)
  })
  it('rejects missing or split teams', () => {
    const cards: RoundCompletionValidation = { ...progress(), owners: [{ ...progress().owners[0], owner: { type: 'team', id: 'team' }, owner_name: 'Team', holes_scored: 0, required_holes: 18, complete: false, confirmed: false }] }
    const roster: RoundPairings = { ...pairings(), scoring_format: 'team_scramble' }
    expect(() => flightProgress(roster, cards)).toThrow(/lagoppsett/)
    expect(() => flightProgress({ ...roster, teams: [group('team', ['a', 'c'])] }, cards)).toThrow(/flighttilknytning/)
  })
  it('rejects mismatched round, lifecycle, draft and owner format', () => {
    expect(() => flightProgress(pairings(), { ...progress(), round_id: 'other' })).toThrow()
    expect(() => flightProgress(pairings(), { ...progress(), status: 'locked' })).toThrow()
    expect(() => flightProgress({ ...pairings(), status: 'draft' }, { ...progress(), status: 'draft' })).toThrow()
    expect(() => flightProgress({ ...pairings(), scoring_format: 'team_scramble' }, progress())).toThrow(/spilleformen/)
  })
  it('keeps empty collections explicit', () => {
    expect(flightProgress({ ...pairings(), flights: [] }, { ...progress(), owners: [] })).toEqual([])
  })
})
