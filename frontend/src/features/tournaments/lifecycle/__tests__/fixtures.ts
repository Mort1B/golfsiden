import type { AuthSession } from '../../../../api/auth'
import type { PairingValidation } from '../../../../api/roundLifecycle'
import type { RoundCompletionValidation } from '../../../../api/scorecards'
import type { Round, Tournament } from '../../../../api/types'

export const tournament: Tournament = {
  id: '00000000-0000-0000-0000-000000000001', name: 'Testturnering', description: '',
  start_date: '2026-09-06', end_date: '2026-09-07', number_of_rounds: 1,
  counted_rounds: 1, mandatory_round_id: null, status: 'active', scoring_mode: 'individual',
  created_at: '2026-09-06T10:00:00Z', updated_at: '2026-09-06T10:00:00Z',
}

export const round: Round = {
  id: '00000000-0000-0000-0000-000000000002', tournament_id: tournament.id,
  round_number: 1, name: 'En runde med et langt navn for alle deltakerne', round_date: '2026-09-06',
  course_id: null, course_name: '', tee_id: null, tee_name: '', number_of_holes: 18,
  status: 'draft', handicap_enabled: true, handicap_allowance_percent: 100,
  scoring_format: 'individual_stroke_play', created_at: tournament.created_at, updated_at: tournament.updated_at,
}

export const session: AuthSession = {
  user_id: '00000000-0000-0000-0000-000000000003', username: 'admin', role: 'admin',
  player_id: '00000000-0000-0000-0000-000000000004', csrf_token: 'test-csrf',
  display_name: 'Administrator', expires_at: '2026-09-07T10:00:00Z',
}

export const opening: PairingValidation = {
  round_id: round.id, ready: true, issues: [], missing_players: [], ineligible_players: [],
  team_sizes: [], missing_flight_players: [], ineligible_flight_players: [], flight_sizes: [],
  legacy_individual_groups: [], split_teams: [],
}

export function completion(status: Round['status'] = 'open'): RoundCompletionValidation {
  return {
    round_id: round.id, status, visibility: { mode: 'full' },
    owners: [{ owner: { type: 'player', id: '00000000-0000-0000-0000-000000000004' },
      owner_name: 'Spiller med et langt navn', holes_scored: 18, required_holes: 18, complete: true, confirmed: true }],
    ready_to_complete: status === 'open', ready_to_lock: status === 'completed',
    issues: [{ code: status === 'open' ? 'round_not_completed' : 'round_not_open', message: 'wrong source' }],
  }
}
