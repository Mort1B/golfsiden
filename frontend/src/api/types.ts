import type { ScoreVisibility } from './visibility'

export type TournamentStatus = 'draft' | 'active' | 'completed' | 'archived'
export type ScoringMode = 'individual' | 'team' | 'combined'
export type RoundStatus = 'draft' | 'open' | 'completed' | 'locked'
export type ScoringFormat = 'individual_stroke_play' | 'team_scramble' | 'two_player_foursomes'
export type ParticipantStatus = 'active' | 'withdrawn'
export type LeaderboardMetric = 'gross' | 'net'

export interface Tournament {
  id: string
  name: string
  description: string
  start_date: string
  end_date: string
  number_of_rounds: number
  counted_rounds: number
  mandatory_round_id: string | null
  status: TournamentStatus
  scoring_mode: ScoringMode
  created_at: string
  updated_at: string
}

export interface TournamentPlayer {
  tournament_id: string
  player_id: string
  display_name: string
  player_active: boolean
  tournament_handicap: number
  seed: number | null
  status: ParticipantStatus
  created_at: string
  updated_at: string
}

export type TournamentHandicapCorrectionState =
  | { state: 'editable' }
  | { state: 'locked'; reason: 'round_opened' | 'snapshot_captured' }

export interface TournamentPlayerRoster {
  handicap_correction: TournamentHandicapCorrectionState
  players: TournamentPlayer[]
}

export interface TournamentHandicapAudit {
  id: string
  tournament_id: string
  player_id: string
  handicap_index: number
  effective_from: string
  changed_by: string | null
  reason: string | null
  created_at: string
}

export interface TournamentHandicapCorrection {
  player: TournamentPlayer
  audit: TournamentHandicapAudit
}

export interface Round {
  id: string
  tournament_id: string
  round_number: number
  name: string
  round_date: string
  course_id: string | null
  course_name: string
  tee_id: string | null
  tee_name: string
  number_of_holes: number
  status: RoundStatus
  handicap_enabled: boolean
  handicap_allowance_percent: number
  scoring_format: ScoringFormat
  created_at: string
  updated_at: string
}

export interface TeamMember {
  player_id: string
  display_name: string
  display_order: number | null
}

export interface Team {
  id: string
  round_id: string
  tournament_id: string
  name: string
  starting_hole: number | null
  tee_time: string | null
  members: TeamMember[]
}

export type LeaderboardOwner =
  | { type: 'player'; id: string }
  | { type: 'team'; id: string }

export interface LeaderboardMember {
  player_id: string
  display_name: string
  display_order: number | null
}

export interface RoundLeaderboardEntry {
  position: number | null
  tied: boolean
  owner: LeaderboardOwner
  owner_name: string
  members: LeaderboardMember[]
  holes_scored: number
  number_of_holes: number
  complete: boolean | null
  confirmed: boolean | null
  playing_handicap: number
  gross_total: number
  net_total: number
  par_played: number
  score_to_par: number
}

export interface RoundLeaderboard {
  round_id: string
  tournament_id: string
  status: RoundStatus
  scoring_format: ScoringFormat
  metric: LeaderboardMetric
  number_of_holes: number
  visible_hole_count: number
  visibility: ScoreVisibility
  entries: RoundLeaderboardEntry[]
}

export interface CurrentTeam {
  round_id: string
  team_id: string
  team_name: string
}

export interface TournamentContribution {
  round_id: string
  owner: LeaderboardOwner
  owner_name: string
  provisional: boolean
  holes_scored: number
  number_of_holes: number
  gross_total: number
  net_total: number
  par_total: number
  score_to_par: number
  counted: boolean
  mandatory: boolean
}

export interface TournamentLeaderboardEntry {
  position: number | null
  tied: boolean
  player_id: string
  display_name: string
  status: ParticipantStatus
  completed_rounds: number
  counted_contributions: number
  eligible: boolean
  gross_total: number
  net_total: number
  par_total: number
  score_to_par: number
  contributions: TournamentContribution[]
  current_team: CurrentTeam | null
}

export interface TournamentLeaderboard {
  tournament_id: string
  metric: LeaderboardMetric
  required_counted_rounds: number
  mandatory_round_id: string | null
  current_round_id: string | null
  included_round_ids: string[]
  visibility: ScoreVisibility
  entries: TournamentLeaderboardEntry[]
}
