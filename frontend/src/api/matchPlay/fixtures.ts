import type { MatchScoringCard } from './contracts'
export const matchIds = { user: '00000000-0000-0000-0000-000000000001', tournament: '00000000-0000-0000-0000-000000000002', round: '00000000-0000-0000-0000-000000000003', match: '00000000-0000-0000-0000-000000000004', first: '00000000-0000-0000-0000-000000000005', second: '00000000-0000-0000-0000-000000000006' }
export function matchFixture(): MatchScoringCard {
  return { format: 'singles_match_play', match_id: matchIds.match, round_id: matchIds.round, tournament_id: matchIds.tournament, round_status: 'open', mode: 'net',
    opponents: [{ player_id: matchIds.first, display_name: 'Andreas med et langt navn', playing_handicap: 0 }, { player_id: matchIds.second, display_name: 'Bjørn med et annet langt navn', playing_handicap: 0 }], relative_handicaps: [0, 0],
    holes: Array.from({ length: 18 }, (_, i) => ({ hole_number: i + 1, par: 4, stroke_index: i + 1 })), notes: [], events: [], resolved_holes: 0, lead: 0, finish: null, confirmed: false, correction_pending: false, half_points: null, visibility: { mode: 'full' }, revision: '1', accepted_events: [] }
}
