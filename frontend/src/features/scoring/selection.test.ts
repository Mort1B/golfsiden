import { describe, expect, it } from 'vitest'
import type { OwnerCompletionProgress } from '../../api/scorecards'
import type { Round, RoundStatus, ScoringFormat } from '../../api/types'
import {
  expectedOwnerType,
  adjacentWritableOwners,
  preferredScoreRound,
  quickOwnerSelection,
  replaceScoreHistory,
  scoreableRounds,
  selectedOwner,
  writableOwnerProgress,
  canonicalVisibleHole,
  ownerProgressLabel,
} from './selection'

function round(number: number, status: RoundStatus, format: ScoringFormat = 'individual_stroke_play'): Round {
  return {
    id: `round-${number}`,
    tournament_id: 'tournament',
    round_number: number,
    name: `Round ${number}`,
    round_date: '2026-09-10',
    course_id: null,
    course_name: 'Course',
    tee_id: null,
    tee_name: 'Tee',
    number_of_holes: 18,
    status,
    handicap_enabled: true,
    handicap_allowance_percent: 100,
    scoring_format: format,
    created_at: '2026-08-16T12:00:00Z',
    updated_at: '2026-08-16T12:00:00Z',
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

  it('validates requested owners for all formats against authority order', () => {
    const owners = [owner('team', 'team-a'), owner('team', 'team-b')]
    expect(selectedOwner(owners, 'team', 'team-b')?.owner.id).toBe('team-b')
    expect(selectedOwner(owners, 'player', 'team-b')?.owner.id).toBe('team-a')
    expect(selectedOwner([], 'team', 'team-b')).toBeUndefined()
    expect(expectedOwnerType(round(1, 'open'))).toBe('player')
    expect(expectedOwnerType(round(1, 'open', 'team_scramble'))).toBe('team')
    expect(expectedOwnerType(round(1, 'open', 'two_player_foursomes'))).toBe('team')
  })

  it('preserves a requested read-only owner but prefers a writable default', () => {
    const owners = [owner('team', 'team-read'), owner('team', 'team-write')]
    const writable = [{ type: 'team' as const, id: 'team-write' }]

    expect(selectedOwner(owners, 'team', 'team-read', writable)?.owner.id).toBe('team-read')
    expect(selectedOwner(owners, null, null, writable)?.owner.id).toBe('team-write')
  })

  it('maps writable owners in access order and keeps exact tagged identities', () => {
    const owners = [
      owner('team', 'shared'),
      owner('player', 'shared'),
      owner('team', 'team-last'),
    ]
    const writable = [
      { type: 'team' as const, id: 'team-last' },
      { type: 'player' as const, id: 'shared' },
      { type: 'team' as const, id: 'missing' },
    ]

    expect(writableOwnerProgress(owners, writable).map((item) => item.owner)).toEqual([
      { type: 'team', id: 'team-last' },
      { type: 'player', id: 'shared' },
    ])
  })

  it('labels restricted progress without inferring completion or confirmation', () => {
    expect(ownerProgressLabel({
      ...owner('player', 'hidden-player'),
      holes_scored: 9,
      required_holes: 9,
      complete: null,
      confirmed: null,
    })).toBe('9/9 synlige hull')
  })

  it('limits post-selection prefetch candidates to adjacent writable cards', () => {
    const owners = [owner('team', 'team-a'), owner('team', 'team-b'), owner('team', 'team-c')]

    expect(adjacentWritableOwners(owners, { type: 'team', id: 'team-b' })).toEqual([
      { type: 'team', id: 'team-a' },
      { type: 'team', id: 'team-c' },
    ])
    expect(adjacentWritableOwners(owners, { type: 'team', id: 'team-a' })).toEqual([
      { type: 'team', id: 'team-b' },
    ])
    expect(adjacentWritableOwners(owners, { type: 'team', id: 'missing' })).toEqual([])
  })

  it('switches the tagged owner while preserving round and hole in hole view', () => {
    expect(quickOwnerSelection({
      tournamentId: 'tournament-1',
      roundId: 'round-2',
      owner: { type: 'team', id: 'team-a' },
      holeNumber: 7,
      view: 'summary',
    }, { type: 'team', id: 'team-b' })).toEqual({
      tournamentId: 'tournament-1',
      roundId: 'round-2',
      owner: { type: 'team', id: 'team-b' },
      holeNumber: 7,
      view: 'hole',
    })
  })

  it('replaces automatic and adjacent navigation but pushes explicit choices', () => {
    expect(replaceScoreHistory('automatic')).toBe(true)
    expect(replaceScoreHistory('previous')).toBe(true)
    expect(replaceScoreHistory('next')).toBe(true)
    expect(replaceScoreHistory('quick-owner')).toBe(true)
    for (const action of ['tournament', 'round', 'owner', 'hole', 'view'] as const) {
      expect(replaceScoreHistory(action)).toBe(false)
    }
  })

  it('canonicalizes hidden or malformed hole targets into the visible prefix', () => {
    expect(canonicalVisibleHole([1, 2, 3, 4, 5, 6, 7, 8, 9], 18)).toBe(9)
    expect(canonicalVisibleHole([1, 2, 3, 4, 5, 6, 7, 8, 9], 6)).toBe(6)
    expect(canonicalVisibleHole([1, 2, 3], undefined)).toBe(1)
    expect(canonicalVisibleHole([], 10)).toBeUndefined()
  })
})
