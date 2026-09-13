import type { FourBallScoringCard } from './contracts'
export const fourBallIds = { round: '00000000-0000-0000-0000-000000000001', team: '00000000-0000-0000-0000-000000000002',
  first: '00000000-0000-0000-0000-000000000003', second: '00000000-0000-0000-0000-000000000004',
  actor: '00000000-0000-0000-0000-000000000005' }
export function fourBallFixture(complete = false): FourBallScoringCard {
  const ids = fourBallIds
  return { format: 'four_ball_stroke_play', projection: 'scoring', round_id: ids.round, owner: { type: 'team', id: ids.team },
    owner_name: 'Et langt lagnavn med to selvstendige partnere',
    partners: [{ player_id: ids.first, display_name: 'Andreas Med Et Svært Langt Spillernavn', playing_handicap: 0 },
      { player_id: ids.second, display_name: 'Bjørn Partner Med Ekstra Slag', playing_handicap: 18 }],
    holes: Array.from({ length: 18 }, (_, index) => {
      const hole_id = `00000000-0000-0000-0000-${String(100 + index).padStart(12, '0')}`
      return { hole_id, hole_number: index + 1, par: 4, stroke_index: index + 1,
        players: [{ player_id: ids.first, handicap_strokes: 0, net_strokes: complete ? 4 : null,
          score: complete ? { id: `00000000-0000-0000-0000-${String(200 + index).padStart(12, '0')}`, revision: '1', round_id: ids.round,
            hole_id, owner: { type: 'player', id: ids.first }, input: { type: 'numeric', gross_strokes: 4 }, submitted_by: ids.actor,
            submitted_at: '2026-09-13T12:00:00Z', updated_at: '2026-09-13T12:00:00Z' } : null },
        { player_id: ids.second, handicap_strokes: 1, net_strokes: null, score: null }],
        gross: complete ? { strokes: 4, player_ids: [ids.first] } : null, net: complete ? { strokes: 4, player_ids: [ids.first] } : null }
    }), gross_total: complete ? 72 : null, net_total: complete ? 72 : null, par_played: complete ? 72 : 0,
    holes_scored: complete ? 18 : 0, number_of_holes: 18, visible_hole_count: 18,
    complete, confirmed: false, confirmed_at: null, visibility: { mode: 'full' } }
}
