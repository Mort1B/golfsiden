import type { ExpectedScore } from '../scorecards/conditional'
import type { ScoreVisibility } from '../visibility'

export type FourBallInput = { type: 'numeric'; gross_strokes: number } | { type: 'no_score' }
export interface FourBallOperation {
  request_id: string
  hole_id: string
  owner: { type: 'player'; id: string }
  input: FourBallInput
  expected_score: ExpectedScore
}
export interface FourBallReadEntry { id: string; input: FourBallInput }
export interface FourBallEntry extends FourBallReadEntry {
  revision: string
  round_id: string
  hole_id: string
  owner: { type: 'player'; id: string }
  submitted_by: string
  submitted_at: string
  updated_at: string
}
export interface FourBallPartner { player_id: string; display_name: string; playing_handicap: number }
export interface FourBallPlayer<E = FourBallReadEntry> {
  player_id: string
  handicap_strokes: number
  score: E | null
  net_strokes: number | null
}
export interface FourBallSelected { strokes: number; player_ids: string[] }
export interface FourBallHole<E = FourBallReadEntry> {
  hole_id: string
  hole_number: number
  par: number
  stroke_index: number
  players: [FourBallPlayer<E>, FourBallPlayer<E>]
  gross: FourBallSelected | null
  net: FourBallSelected | null
}
interface FourBallBase<E> {
  format: 'four_ball_stroke_play'
  round_id: string
  owner: { type: 'team'; id: string }
  owner_name: string
  partners: [FourBallPartner, FourBallPartner]
  holes: FourBallHole<E>[]
  gross_total: number | null
  net_total: number | null
  par_played: number
  holes_scored: number
  number_of_holes: number
  visible_hole_count: number
  complete: boolean | null
  confirmed: boolean | null
  confirmed_at: string | null
  visibility: ScoreVisibility
}
export interface FourBallReadCard extends FourBallBase<FourBallReadEntry> { projection: 'read' }
export interface FourBallScoringCard extends FourBallBase<FourBallEntry> { projection: 'scoring' }
export type FourBallCard = FourBallReadCard | FourBallScoringCard
export function equalInput(left: FourBallInput | null, right: FourBallInput | null): boolean {
  return left === null || right === null ? left === right : left.type === right.type
    && (left.type === 'no_score' || (right.type === 'numeric' && left.gross_strokes === right.gross_strokes))
}
export function inputLabel(input: FourBallInput | null): string {
  return input === null ? 'Ikke registrert' : input.type === 'no_score' ? 'Plukket opp' : `${input.gross_strokes} slag`
}
export function expectedFourBall(score: FourBallEntry | null): ExpectedScore {
  return score === null ? { type: 'absent' } : { type: 'present', score_id: score.id, revision: score.revision }
}
