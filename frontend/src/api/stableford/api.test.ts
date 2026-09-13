import { afterEach, expect, it, vi } from 'vitest'
import { stablefordApi } from '../stableford'
import { stablefordFixture } from './fixtures'
import { round } from '../../features/tournaments/lifecycle/__tests__/fixtures'
import { defaultHandicapAllowanceForFormat, ownerTypeForScoringFormat, inputOwnerTypeForScoringFormat } from '../scoringFormats'
afterEach(() => vi.unstubAllGlobals())
it('keeps competition and input ownership individual with a 100 percent default', () => {
  expect(ownerTypeForScoringFormat('individual_stableford')).toBe('player')
  expect(inputOwnerTypeForScoringFormat('individual_stableford')).toBe('player')
  expect(defaultHandicapAllowanceForFormat('individual_stableford')).toBe(100)
})
it('sends only the dedicated conditional player input and keeps positive revisions as strings', async () => {
  const card = stablefordFixture('pickup'); const hole = card.holes[0]; if (!hole?.score) throw new Error('fixture')
  const operation = { request_id: crypto.randomUUID(), hole_id: hole.hole_id, owner: card.owner, input: { type: 'no_score' as const },
    expected_score: { type: 'present' as const, score_id: hole.score.id, revision: '9007199254740993' } }
  const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({ request_id: operation.request_id, applied_score: { score_id: hole.score.id, revision: '9007199254740994' } }), { status: 200 }))
  vi.stubGlobal('fetch', fetcher)
  await stablefordApi.save(card.round_id, operation, 'csrf')
  expect(fetcher.mock.calls[0]?.[0]).toBe(`/api/rounds/${card.round_id}/stableford/inputs/conditional`)
  expect(fetcher.mock.calls[0]?.[1].body).toBe(JSON.stringify(operation))
  expect(fetcher.mock.calls[0]?.[1].headers['x-csrf-token']).toBe('csrf')
})
it('draft settings submit their expected timestamp and explicit zero allowance', async () => {
  const configured = { ...round, scoring_format: 'individual_stableford' as const, handicap_allowance_percent: 0, handicap_enabled: false }
  const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify(configured), { status: 200 }))
  vi.stubGlobal('fetch', fetcher)
  expect(await stablefordApi.settings(configured, false, 0, 'csrf')).toEqual(configured)
  expect(fetcher.mock.calls[0]?.[0]).toBe(`/api/rounds/${round.id}/stableford/settings`)
  expect(JSON.parse(fetcher.mock.calls[0]?.[1].body)).toEqual({ expected_round_updated_at: round.updated_at, handicap_enabled: false, handicap_allowance_percent: 0 })
})
