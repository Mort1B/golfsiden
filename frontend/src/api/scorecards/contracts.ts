import type { RoundStatus } from '../types'
import type { ScoreVisibility } from '../visibility'

export type ScoreOwnerType = 'player' | 'team'
export type ScoreOwner = { type: 'player'; id: string } | { type: 'team'; id: string }

export interface ScoreEntry {
  id: string
  round_id: string
  hole_id: string
  owner: ScoreOwner
  gross_strokes: number
  submitted_by: string
  submitted_at: string
  updated_at: string
}

export interface ReadScoreEntry { id: string; gross_strokes: number }

interface ScorecardHoleBase {
  hole_id: string
  hole_number: number
  par: number
  stroke_index: number
  net_strokes: number | null
}

export interface ScoringScorecardHole extends ScorecardHoleBase { score: ScoreEntry | null }
export interface ReadScorecardHole extends ScorecardHoleBase { score: ReadScoreEntry | null }
export type ScorecardHole = ScoringScorecardHole | ReadScorecardHole

interface ScorecardBase {
  round_id: string
  owner: ScoreOwner
  holes: ScorecardHole[]
  gross_total: number
  net_total: number
  playing_handicap: number
  holes_scored: number
  number_of_holes: number
}

export interface ScoringScorecard extends ScorecardBase {
  projection: 'scoring'
  holes: ScoringScorecardHole[]
  complete: boolean
  confirmed: boolean
  confirmed_by: string | null
  confirmed_at: string | null
}

export interface ReadScorecard extends ScorecardBase {
  projection: 'read'
  holes: ReadScorecardHole[]
  visible_hole_count: number
  complete: boolean | null
  confirmed: boolean | null
  confirmed_at: string | null
  visibility: ScoreVisibility
}

export type ScorecardSummary = ScoringScorecard | ReadScorecard

export interface OwnerCompletionProgress {
  owner: ScoreOwner
  owner_name: string
  holes_scored: number
  required_holes: number
  complete: boolean | null
  confirmed: boolean | null
}

export interface ScoreAccess { round_id: string; writable_owners: ScoreOwner[] }

export type CompletionIssueCode = 'no_required_owners' | 'incomplete_scorecards'
  | 'unconfirmed_scorecards' | 'round_not_open' | 'round_not_completed'

export interface CompletionIssue { code: CompletionIssueCode; message: string }

export interface RoundCompletionValidation {
  round_id: string
  status: RoundStatus
  owners: OwnerCompletionProgress[]
  ready_to_complete: boolean | null
  ready_to_lock: boolean | null
  issues: CompletionIssue[]
  visibility: ScoreVisibility
}

export function ownerEquals(left: ScoreOwner, right: ScoreOwner): boolean {
  return left.type === right.type && left.id === right.id
}
