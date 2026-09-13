import { expect, it } from 'vitest'
import { finalMatchExplanation } from './finalRoundExplanation'
import { round } from '../tournaments/lifecycle/__tests__/fixtures'
it('explains only the actual final match for final-score tie breaking', () => {
  const matches = [{ ...round, scoring_format: 'singles_match_play' as const, round_number: 2 }]
  expect(finalMatchExplanation(matches, 2, 'final_round_score')).toContain('beholder delt plass')
  expect(finalMatchExplanation(matches, 3, 'final_round_score')).toBeNull()
  expect(finalMatchExplanation(matches, 2, 'shared_positions')).toBeNull()
})
