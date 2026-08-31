import { describe, expect, it } from 'vitest'
import type { TournamentLeaderboardEntry } from '../../api/types'
import { bestRoundsProgressLabel, scoreToParLabel, scoringFormatLabel } from './format'

const entry: TournamentLeaderboardEntry = {
  position: 1,
  tied: false,
  player_id: '00000000-0000-0000-0000-000000001001',
  display_name: 'Spiller En',
  status: 'active',
  completed_rounds: 2,
  counted_contributions: 2,
  eligible: false,
  gross_total: 145,
  net_total: 141,
  par_total: 144,
  score_to_par: -3,
  contributions: [],
  current_team: null,
}

describe('leaderboard format labels', () => {
  it('labels every current scoring format', () => {
    expect(scoringFormatLabel('individual_stroke_play')).toBe('Individuelt slagspill')
    expect(scoringFormatLabel('team_scramble')).toBe('Lag-scramble')
    expect(scoringFormatLabel('two_player_foursomes')).toBe('Foursomes (to spillere)')
  })

  it('formats selected score-to-par and best-N qualification progress', () => {
    expect(scoreToParLabel(-3)).toBe('-3')
    expect(bestRoundsProgressLabel(entry, 3))
      .toBe('Beste 3 · 2 av 3 tellende · 2 fullførte runder · Ikke kvalifisert ennå')
    expect(bestRoundsProgressLabel({
      ...entry,
      completed_rounds: 4,
      counted_contributions: 3,
      eligible: true,
    }, 3)).toBe('Beste 3 · 3 av 3 tellende · 4 fullførte runder · Kvalifisert')
    expect(bestRoundsProgressLabel({
      ...entry,
      completed_rounds: 0,
      counted_contributions: 0,
    }, 3)).toContain('Ingen fullførte runder')
  })
})
