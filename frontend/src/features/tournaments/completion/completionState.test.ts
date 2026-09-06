import { describe, expect, it } from 'vitest'
import { completionReadiness } from './completionState'
import { round, tournament } from '../lifecycle/__tests__/fixtures'

describe('tournament completion readiness', () => {
  it('requires all configured rounds, not just counted rounds', () => {
    const trip = { ...tournament, number_of_rounds: 2, counted_rounds: 1 }
    expect(completionReadiness(trip, [{ ...round, status: 'locked' }]).ready).toBe(false)
    expect(completionReadiness(trip, [{ ...round, status: 'locked' }, { ...round, id: 'second', round_number: 2, status: 'locked' }]).ready).toBe(true)
  })
  it.each(['draft', 'open', 'completed'] as const)('lists %s rounds as unresolved', (status) => {
    const state = completionReadiness(tournament, [{ ...round, status }])
    expect(state.ready).toBe(false)
    expect(state.unlocked).toHaveLength(1)
  })
  it('rejects empty, duplicate, non-contiguous and foreign plans', () => {
    expect(completionReadiness(tournament, undefined).valid).toBe(false)
    expect(completionReadiness(tournament, []).valid).toBe(false)
    expect(completionReadiness(tournament, [{ ...round, round_number: 2 }]).valid).toBe(false)
    expect(completionReadiness(tournament, [{ ...round, tournament_id: 'other' }]).valid).toBe(false)
    expect(completionReadiness({ ...tournament, number_of_rounds: 2 }, [round, round]).valid).toBe(false)
  })
  it.each(['draft', 'completed', 'archived'] as const)('cannot complete a %s tournament', (status) => {
    expect(completionReadiness({ ...tournament, status }, [{ ...round, status: 'locked' }]).ready).toBe(false)
  })
})
