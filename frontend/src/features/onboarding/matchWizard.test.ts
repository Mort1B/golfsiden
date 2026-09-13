import { expect, it } from 'vitest'
import { addRound, createInitialDraft, toTournamentPlanRequest, updateCountedRounds, updateMandatoryRound, updateRound } from './wizardState'
import { validateRounds } from './validation'
it('reconciles match-only to explicit null and restores eligible overall N when adding stroke format', () => {
  let draft = createInitialDraft('2026-09-14')
  draft = updateMandatoryRound(draft, 'round-1')
  draft = updateRound(draft, 'round-1', { scoringFormat: 'singles_match_play' })
  expect(draft.countedRounds).toBeNull(); expect(draft.mandatoryRoundKey).toBeNull()
  expect(toTournamentPlanRequest({ ...draft, creator: { displayName: '', username: '', password: '', handicap: '0' } }).tournament.counted_rounds).toBeNull()
  draft = addRound(draft); expect(draft.countedRounds).toBeNull()
  draft = updateRound(draft, 'round-2', { scoringFormat: 'individual_stroke_play' })
  expect(draft.countedRounds).toBe(1)
  expect(updateCountedRounds(draft, 2).countedRounds).toBe(1)
  expect(updateMandatoryRound(draft, 'round-1').mandatoryRoundKey).toBeNull()
})
it('rejects N=0 and numeric N in match-only plans', () => {
  const draft = updateRound(createInitialDraft('2026-09-14'), 'round-1', { scoringFormat: 'singles_match_play' })
  expect(validateRounds(draft.rounds, draft.tournament, null)).toEqual({})
  expect(validateRounds(draft.rounds, draft.tournament, 1)).toHaveProperty('rounds.countedRounds')
  expect(validateRounds(draft.rounds, draft.tournament, 0)).toHaveProperty('rounds.countedRounds')
})
