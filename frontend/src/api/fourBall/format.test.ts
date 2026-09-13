import { expect, it } from 'vitest'
import { decodeRoundLeaderboard } from '../leaderboards'
import { defaultHandicapAllowanceForFormat, inputOwnerTypeForScoringFormat, isScoringFormat, ownerTypeForScoringFormat } from '../scoringFormats'
import { fourBallIds as ids } from './fixtures'
it('exposes four-ball as a team competition with player inputs and an 85 percent default', () => {
  expect(isScoringFormat('four_ball_stroke_play')).toBe(true)
  expect(ownerTypeForScoringFormat('four_ball_stroke_play')).toBe('team')
  expect(inputOwnerTypeForScoringFormat('four_ball_stroke_play')).toBe('player')
  expect(defaultHandicapAllowanceForFormat('four_ball_stroke_play')).toBe(85)
  expect(defaultHandicapAllowanceForFormat('two_player_foursomes')).toBe(50)
  expect(isScoringFormat('stableford')).toBe(false)
  expect(isScoringFormat('match_play')).toBe(false)
})
it('accepts absent team handicap only for four-ball and rejects a fabricated scalar handicap', () => {
  const data = { round_id: ids.round, tournament_id: ids.actor, status: 'open', scoring_format: 'four_ball_stroke_play',
    metric: 'gross', number_of_holes: 18, visible_hole_count: 18, visibility: { mode: 'full' },
    entries: [{ position: null, tied: false, owner: { type: 'team', id: ids.team }, owner_name: 'Partners',
      members: [{ player_id: ids.first, display_name: 'First', display_order: 1 }, { player_id: ids.second, display_name: 'Second', display_order: 2 }],
      holes_scored: 0, number_of_holes: 18, complete: false, confirmed: false, playing_handicap: null,
      gross_total: 0, net_total: 0, par_played: 0, score_to_par: 0 }] }
  const decode = (value: unknown) => decodeRoundLeaderboard(value, ids.round, ids.actor, 'gross')
  expect(decode(data).entries[0]?.playing_handicap).toBeNull()
  expect(() => decode({ ...data, scoring_format: 'team_scramble' })).toThrow('playing_handicap')
  expect(() => decode({ ...data, entries: data.entries.map(entry => ({ ...entry, playing_handicap: 0 })) })).toThrow('playing_handicap')
})
