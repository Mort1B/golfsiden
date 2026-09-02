import { describe, expect, it } from 'vitest'
import type { Round, TournamentContribution } from '../../api/types'
import {
  contributionStateLabels,
  hasPlayerHistoryBackgroundError,
  mandatoryPlayerHistoryLabel,
  orderedPlayerContributions,
} from './playerHistory'

const baseRound: Round = {
  id: 'round-two', tournament_id: 'tournament', round_number: 2, name: 'Andre', round_date: '2026-09-02',
  course_id: null, course_name: '', tee_id: null, tee_name: '', number_of_holes: 18, status: 'completed',
  handicap_enabled: true, handicap_allowance_percent: 100, scoring_format: 'individual_stroke_play',
  created_at: '2026-09-01T00:00:00Z', updated_at: '2026-09-01T00:00:00Z',
}
const contribution: TournamentContribution = {
  round_id: 'round-two', owner: { type: 'team', id: 'historical-team' }, owner_name: 'Historisk lag',
  provisional: false, holes_scored: 18, number_of_holes: 18, gross_total: 75, net_total: 70,
  par_total: 72, score_to_par: -2, counted: true, mandatory: false,
}

describe('player history', () => {
  it('orders visible contributions by round and retains historical team ownership', () => {
    const result = orderedPlayerContributions([
      contribution,
      { ...contribution, round_id: 'round-one', owner: { type: 'player', id: 'player' } },
    ], [baseRound, { ...baseRound, id: 'round-one', round_number: 1, name: 'Første' }])
    expect(result.map((item) => [item.round.name, item.contribution.owner])).toEqual([
      ['Første', { type: 'player', id: 'player' }],
      ['Andre', { type: 'team', id: 'historical-team' }],
    ])
  })

  it('labels metric selection, provisional progress, and mandatory state explicitly', () => {
    expect(contributionStateLabels({
      ...contribution, counted: false, provisional: true, holes_scored: 9, mandatory: true,
    })).toEqual(['Forkastet', 'Foreløpig · 9 av 18 hull', 'Obligatorisk runde'])
  })

  it('labels a missing completed mandatory result as missing', () => {
    const completedMandatory = { ...baseRound, id: 'mandatory', round_number: 1, name: 'Obligatorisk' }
    const laterRound = { ...baseRound, id: 'later', round_number: 2, name: 'Senere' }
    expect(mandatoryPlayerHistoryLabel(
      completedMandatory.id, [completedMandatory, laterRound], 'full', [],
    )).toBe('Obligatorisk: mangler')
  })

  it.each(['completed', 'locked'] as const)('labels an embargo-hidden %s final as awaiting release', (status) => {
    const earlierRound = { ...baseRound, id: 'earlier', round_number: 2, name: 'Tidligere' }
    const hiddenFinal = { ...baseRound, id: 'final', round_number: 3, status, name: 'Finalen' }
    expect(mandatoryPlayerHistoryLabel(
      hiddenFinal.id, [earlierRound, hiddenFinal], 'front_nine', [],
    )).toBe('Finalen: avventer frigivelse')
  })

  it('shows background errors only when the same failed query retains data', () => {
    const error = new Error('refetch failed')
    expect(hasPlayerHistoryBackgroundError(null, [baseRound], error, undefined)).toBe(false)
    expect(hasPlayerHistoryBackgroundError(error, undefined, null, { entries: [] })).toBe(false)
    expect(hasPlayerHistoryBackgroundError(error, [baseRound], null, undefined)).toBe(true)
    expect(hasPlayerHistoryBackgroundError(null, undefined, error, { entries: [] })).toBe(true)
  })
})
