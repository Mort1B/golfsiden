import type { RoundStatus } from '../types'
import type { ScoreVisibility } from '../visibility'
export type MatchOutcome = 'first' | 'second' | 'halved'
export type MatchBasis =
  | { type: 'numeric'; first_gross: number; second_gross: number; agreed: boolean }
  | { type: 'next_stroke_concession'; first_gross: number; second_gross: number; conceding_player_id: string; communicated: boolean; agreed: boolean }
  | { type: 'hole_concession'; conceding_player_id: string; communicated: boolean }
  | { type: 'agreed_halve'; play_begun: boolean; mutual_agreement: boolean }
  | { type: 'organizer_ruling'; reason: string }
export type MatchEvent =
  | { type: 'hole'; hole_number: number; outcome: MatchOutcome; basis: MatchBasis }
  | { type: 'concession'; conceding_player_id: string; communicated: boolean; after_hole: number }
  | { type: 'award'; winner_player_id: string; reason: string; after_hole: number }
export type MatchNoteCommand = { type: 'note'; player_id: string; hole_number: number; gross_strokes: number }
  | { type: 'clear_note'; player_id: string; hole_number: number }
export type MatchCommand = MatchNoteCommand | { type: 'report'; event: MatchEvent }
  | { type: 'confirm'; result_agreed_or_awarded: boolean }
  | { type: 'correct'; kind: 'recording_error' | 'organizer_ruling'; reason: string; superseded_event_ids: string[]; replacement: MatchEvent[] }
export interface MatchRequest { request_id: string; expected_revision: string; command: MatchCommand }
export interface MatchAcknowledgement { request_id: string; match_id: string; applied_revision: string }
export type MatchFinish = { type: 'on_holes'; winner: 'first' | 'second'; margin: number; holes_remaining: number }
  | { type: 'draw' } | { type: 'conceded' | 'awarded'; winner: 'first' | 'second' }
export interface MatchOpponent { player_id: string; display_name: string; playing_handicap: number | null }
export interface MatchNote { player_id: string; hole_number: number; gross_strokes: number | null }
export interface MatchHole { hole_number: number; par: number; stroke_index: number }
export interface MatchCard {
  format: 'singles_match_play'; match_id: string; round_id: string; tournament_id: string; round_status: RoundStatus
  mode: 'gross' | 'net'; opponents: [MatchOpponent, MatchOpponent]; relative_handicaps: [number, number]
  holes: MatchHole[]; notes: MatchNote[]; events: MatchEvent[]; resolved_holes: number; lead: number
  finish: MatchFinish | null; confirmed: boolean | null; correction_pending: boolean | null
  half_points: [number, number] | null; visibility: ScoreVisibility
}
export interface MatchScoringCard extends MatchCard { revision: string; accepted_events: { id: string; event: MatchEvent }[] }
export interface MatchListing { round_id: string; matches: MatchCard[]; writable_match_ids: string[] }
export interface MatchPlayerListing extends MatchListing { player_id: string }
export interface MatchCompletion {
  format: 'singles_match_play'; round_id: string; status: RoundStatus
  matches: { match_id: string; terminal: boolean | null; confirmed: boolean | null }[]
  ready_to_complete: boolean | null; ready_to_lock: boolean | null
}
export interface MatchTableEntry {
  player_id: string; display_name: string; half_points: number; played: number; wins: number; draws: number; losses: number; position: number | null
}
export interface MatchTable { tournament_id: string; entries: MatchTableEntry[] }
export interface MatchPair { first_player_id: string; second_player_id: string }
