import { expect, it } from 'vitest'
import { decodeCreationReceipt } from './tournamentCreation'
import { createInitialDraft, toTournamentPlanRequest } from '../features/onboarding/wizardState'
const key = '22000000-0000-0000-0000-000000000001'
const trip = '22000000-0000-0000-0000-000000000002'
it('decodes new and replay receipts and rejects wrong request identity', () => {
  for (const created of [true,false]) expect(decodeCreationReceipt({ request_id: key, tournament_id: trip, created }, key)).toEqual({ request_id:key,tournament_id:trip,created })
  expect(() => decodeCreationReceipt({ request_id:trip,tournament_id:trip,created:true },key)).toThrow()
  expect(() => decodeCreationReceipt({ request_id:key,tournament_id:'bad',created:true },key)).toThrow()
  expect(() => decodeCreationReceipt({ request_id:key,tournament_id:trip,created:'true' },key)).toThrow()
})
it('builds a plan without requiring or serializing new-account credentials', () => {
  const draft = createInitialDraft('2026-09-10')
  draft.tournament.name = ' Test '
  const plan = toTournamentPlanRequest(draft)
  expect(plan.tournament.name).toBe('Test')
  expect(Object.keys(plan)).toEqual(['tournament','rounds'])
  expect(plan.rounds).toHaveLength(1)
})
