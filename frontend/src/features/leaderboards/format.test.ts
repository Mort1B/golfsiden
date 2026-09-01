import { describe, expect, it } from 'vitest'
import type { Round, TournamentContribution, TournamentLeaderboardEntry } from '../../api/types'
import {
  bestRoundsProgressLabel,
  mandatoryRoundDisplayState,
  mandatoryRoundProgressLabel,
  provisionalProgressLabel,
  scoreToParLabel,
  scoringFormatLabel,
  selectedProvisional,
} from './format'

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

const provisional: TournamentContribution = {
  round_id: '00000000-0000-0000-0000-000000004004',
  owner: { type: 'team', id: '00000000-0000-0000-0000-000000003001' },
  owner_name: 'Et lag med et svært langt navn som må få brytes på mobil',
  provisional: true,
  holes_scored: 7,
  number_of_holes: 18,
  gross_total: 30,
  net_total: 27,
  par_total: 28,
  score_to_par: -1,
  counted: true,
  mandatory: false,
}

const mandatoryRound: Round = {
  id: provisional.round_id,
  tournament_id: '00000000-0000-0000-0000-000000002001',
  round_number: 4,
  name: 'Finalerunden med et svært langt banenavn',
  round_date: '2026-09-04',
  course_id: null,
  course_name: 'Testbane',
  tee_id: null,
  tee_name: 'Gul',
  number_of_holes: 18,
  status: 'completed',
  handicap_enabled: true,
  handicap_allowance_percent: 100,
  scoring_format: 'individual_stroke_play',
  created_at: '2026-09-01T10:00:00Z',
  updated_at: '2026-09-04T15:00:00Z',
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
      .toBe('2 av 3 fullførte tellende · 2 fullførte runder · Ikke kvalifisert ennå')
    expect(bestRoundsProgressLabel({
      ...entry,
      completed_rounds: 4,
      counted_contributions: 3,
      eligible: true,
    }, 3)).toBe('3 av 3 fullførte tellende · 4 fullførte runder · Kvalifisert')
    expect(bestRoundsProgressLabel({
      ...entry,
      completed_rounds: 0,
      counted_contributions: 0,
    }, 3)).toContain('Ingen fullførte runder')
  })

  it('labels completed and missing mandatory rounds without truncating the name', () => {
    const name = 'Finalerunden med et svært langt banenavn'
    expect(mandatoryRoundProgressLabel(name, 'completed')).toBe(`${name}: fullført`)
    expect(mandatoryRoundProgressLabel(name, 'missing')).toBe(`${name}: mangler`)
    expect(mandatoryRoundProgressLabel(name, 'open')).toBe(`${name}: pågår · ingen score ennå`)
    expect(mandatoryRoundProgressLabel(name, 'open', { holesScored: 7, numberOfHoles: 18 }))
      .toBe(`${name}: pågår · 7 av 18 hull`)
  })

  it('keeps an embargoed mandatory final neutral until its result is released', () => {
    const completedContribution = { ...provisional, provisional: false }
    const earlierRound = { ...mandatoryRound, id: '00000000-0000-0000-0000-000000004003', round_number: 3 }
    const rounds = [earlierRound, mandatoryRound]
    expect(mandatoryRoundDisplayState(mandatoryRound, rounds, 'front_nine', undefined)).toBe('awaiting_release')
    expect(mandatoryRoundProgressLabel(mandatoryRound.name, 'awaiting_release'))
      .toBe(`${mandatoryRound.name}: avventer frigivelse`)
    expect(mandatoryRoundDisplayState({ ...mandatoryRound, status: 'locked' }, rounds, 'front_nine', undefined))
      .toBe('awaiting_release')
    expect(mandatoryRoundDisplayState(earlierRound, rounds, 'front_nine', undefined)).toBe('missing')
    expect(mandatoryRoundDisplayState(mandatoryRound, rounds, 'full', undefined)).toBe('missing')
    expect(mandatoryRoundDisplayState({ ...mandatoryRound, status: 'open' }, rounds, 'front_nine', undefined)).toBe('open')
    expect(mandatoryRoundDisplayState(mandatoryRound, rounds, 'front_nine', completedContribution)).toBe('completed')
  })

  it('marks only a displayed provisional result and preserves long team labels', () => {
    expect(selectedProvisional({ ...entry, contributions: [provisional] })).toBe(provisional)
    expect(selectedProvisional({ ...entry, contributions: [{ ...provisional, counted: false }] })).toBeNull()
    expect(provisionalProgressLabel(provisional))
      .toBe(`Foreløpig · 7 av 18 hull · ${provisional.owner_name}`)
  })
})
