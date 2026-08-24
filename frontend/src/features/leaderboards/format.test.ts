import { describe, expect, it } from 'vitest'
import { scoringFormatLabel } from './format'

describe('leaderboard format labels', () => {
  it('labels every current scoring format', () => {
    expect(scoringFormatLabel('individual_stroke_play')).toBe('Individuelt slagspill')
    expect(scoringFormatLabel('team_scramble')).toBe('Lag-scramble')
    expect(scoringFormatLabel('two_player_foursomes')).toBe('Foursomes (to spillere)')
  })
})
