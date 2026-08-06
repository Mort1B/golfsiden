import { describe, expect, it } from 'vitest'
import type { OwnerCompletionProgress } from '../../api/scorecards'
import type { Round, RoundStatus, ScoringFormat } from '../../api/types'
import {
  expectedOwnerType,
  preferredScoreRound,
  replaceScoreHistory,
  scoreableRounds,
  selectedOwner,
} from './selection'

function round(number: number, status: RoundStatus, format: ScoringFormat = 'individual_stroke_play'): Round {
  return {
    id: `round-${number}`,
    tournament_id: 'tournament',
    round_number: number,
    name: `Round ${number}`,
    round_date: '2026-09-10',
    course_name: 'Course',
    tee_name: 'Tee',
    number_of_holes: 18,
    status,
    handicap_enabled: true,
    handicap_allowance_percent: 100,
    scoring_format: format,
  }
}

function owner(type: 'player' | 'team', id: string): OwnerCompletionProgress {
  return {
    owner: type === 'player' ? { type, id } : { type, id },
    owner_name: id,
    holes_scored: 0,
    required_holes: 18,
    complete: false,
    confirmed: false,
  }
}

describe('scoring selection', () => {
  it('prefers latest open, then completed, then locked and excludes draft', () => {
    expect(preferredScoreRound([round(1, 'completed'), round(2, 'open'), round(3, 'open')])?.round_number).toBe(3)
    expect(preferredScoreRound([round(2, 'locked'), round(1, 'completed')])?.status).toBe('completed')
    expect(preferredScoreRound([round(4, 'draft'), round(2, 'locked')])?.status).toBe('locked')
    expect(scoreableRounds([round(1, 'draft'), round(2, 'open')]).map((item) => item.round_number)).toEqual([2])
  })

  it('validates requested individual and scramble owners against authority order', () => {
    const owners = [owner('team', 'team-a'), owner('team', 'team-b')]
    expect(selectedOwner(owners, 'team', 'team-b')?.owner.id).toBe('team-b')
    expect(selectedOwner(owners, 'player', 'team-b')?.owner.id).toBe('team-a')
    expect(selectedOwner([], 'team', 'team-b')).toBeUndefined()
    expect(expectedOwnerType(round(1, 'open'))).toBe('player')
    expect(expectedOwnerType(round(1, 'open', 'team_scramble'))).toBe('team')
  })

  it('preserves a requested read-only owner but prefers a writable default', () => {
    const owners = [owner('team', 'team-read'), owner('team', 'team-write')]
    const writable = [{ type: 'team' as const, id: 'team-write' }]

    expect(selectedOwner(owners, 'team', 'team-read', writable)?.owner.id).toBe('team-read')
    expect(selectedOwner(owners, null, null, writable)?.owner.id).toBe('team-write')
  })

  it('replaces automatic and adjacent navigation but pushes explicit choices', () => {
    expect(replaceScoreHistory('automatic')).toBe(true)
    expect(replaceScoreHistory('previous')).toBe(true)
    expect(replaceScoreHistory('next')).toBe(true)
    for (const action of ['tournament', 'round', 'owner', 'hole', 'view'] as const) {
      expect(replaceScoreHistory(action)).toBe(false)
    }
  })
})
