import type { FourBallEntry, FourBallReadEntry } from '../fourBall/contracts'
import type { ScoreVisibility } from '../visibility'
export interface StablefordValues {
  gross_points: number
  net_points: number
  gross_equivalent: number
  net_equivalent: number
  actual_gross_total: number | null
  actual_net_total: number | null
}
export interface StablefordHole<E = FourBallReadEntry> {
  hole_id: string
  hole_number: number
  par: number
  stroke_index: number
  handicap_strokes: number
  score: E | null
  net_strokes: number | null
  gross_points: number | null
  net_points: number | null
}
interface StablefordBase<E> {
  format: 'individual_stableford'
  round_id: string
  owner: { type: 'player'; id: string }
  owner_name: string
  playing_handicap: number
  holes: StablefordHole<E>[]
  values: StablefordValues | null
  holes_scored: number
  number_of_holes: number
  visible_hole_count: number
  complete: boolean | null
  confirmed: boolean | null
  confirmed_at: string | null
  visibility: ScoreVisibility
}
export interface StablefordReadCard extends StablefordBase<FourBallReadEntry> { projection: 'read' }
export interface StablefordScoringCard extends StablefordBase<FourBallEntry> { projection: 'scoring' }
export type StablefordCard = StablefordReadCard | StablefordScoringCard
