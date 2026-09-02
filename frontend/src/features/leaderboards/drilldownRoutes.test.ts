import { describe, expect, it } from 'vitest'
import { parseDrilldownMetric, parseOwnerType, playerHistoryUrl, scorecardSearch, scorecardUrl } from './drilldownRoutes'

describe('leaderboard drilldown routes', () => {
  it('builds canonical metric-specific history and historical owner URLs', () => {
    expect(playerHistoryUrl('tournament', 'player', 'net'))
      .toBe('/tournaments/tournament/results/players/player?metric=net')
    expect(scorecardUrl('tournament', 'round', { type: 'team', id: 'historical-team' }, 'gross'))
      .toBe('/tournaments/tournament/rounds/round/scorecards/team/historical-team?metric=gross&view=summary')
  })

  it('canonicalizes optional state and rejects unknown owner types', () => {
    expect(parseDrilldownMetric('gross')).toBe('gross')
    expect(parseDrilldownMetric('bogey')).toBe('gross')
    expect(parseOwnerType('player')).toBe('player')
    expect(parseOwnerType('flight')).toBeNull()
    expect(scorecardSearch('net', 'summary', 8).toString()).toBe('metric=net&view=summary')
    expect(scorecardSearch('net', 'hole', 8).toString()).toBe('metric=net&view=hole&hole=8')
  })
})
