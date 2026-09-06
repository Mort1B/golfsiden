import type { PairingGroup, RoundPairings } from '../../api/pairings'
import type { RoundCompletionValidation } from '../../api/scorecards'

export const group = (id: string, members: string[]): PairingGroup => ({
  id, name: id, starting_hole: 10, tee_time: '09:30:00', created_at: '', updated_at: '',
  members: members.map((player_id) => ({ player_id, display_name: player_id, display_order: null })),
})
export function pairings(): RoundPairings {
  return { round_id: 'round', tournament_id: 'trip', status: 'open', scoring_format: 'individual_stroke_play',
    updated_at: '', active_entrants: [], inactive_entrants: [], teams: [], legacy_individual_groups: [],
    flights: [group('flight1', ['a', 'b']), group('flight2', ['c', 'd'])] }
}
export function progress(): RoundCompletionValidation {
  return { round_id: 'round', status: 'open', visibility: { mode: 'full' },
    ready_to_complete: false, ready_to_lock: false, issues: [],
    owners: ['a', 'b', 'c', 'd'].map((id, index) => ({ owner: { type: 'player', id }, owner_name: id,
      holes_scored: index === 0 ? 18 : 2, required_holes: 18, complete: index === 0, confirmed: index === 0 })) }
}
