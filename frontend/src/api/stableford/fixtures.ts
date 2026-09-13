import { fourBallFixture } from '../fourBall/fixtures'
import type { StablefordScoringCard } from './contracts'
export function stablefordFixture(state: 'blank' | 'numeric' | 'pickup' = 'blank'): StablefordScoringCard {
  const source = fourBallFixture(state !== 'blank')
  return { format: 'individual_stableford', projection: 'scoring', round_id: source.round_id,
    owner: { type: 'player', id: source.partners[0].player_id }, owner_name: source.partners[0].display_name, playing_handicap: 0,
    holes: source.holes.map(h => ({ hole_id: h.hole_id, hole_number: h.hole_number, par: h.par, stroke_index: h.stroke_index,
      handicap_strokes: 0, net_strokes: state === 'numeric' ? 4 : null, gross_points: state === 'blank' ? null : state === 'pickup' ? 0 : 2,
      net_points: state === 'blank' ? null : state === 'pickup' ? 0 : 2,
      score: h.players[0].score ? { ...h.players[0].score, input: state === 'pickup' ? { type: 'no_score' } : { type: 'numeric', gross_strokes: 4 } } : null })),
    values: state === 'blank' ? null : { gross_points: state === 'pickup' ? 0 : 36, net_points: state === 'pickup' ? 0 : 36,
      gross_equivalent: state === 'pickup' ? 36 : 0, net_equivalent: state === 'pickup' ? 36 : 0,
      actual_gross_total: state === 'numeric' ? 72 : null, actual_net_total: state === 'numeric' ? 72 : null },
    holes_scored: state === 'blank' ? 0 : 18, number_of_holes: 18, visible_hole_count: 18, complete: state !== 'blank',
    confirmed: false, confirmed_at: null, visibility: { mode: 'full' } }
}
