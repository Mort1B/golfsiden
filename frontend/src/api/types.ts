export type TournamentStatus = 'draft' | 'active' | 'completed' | 'archived'
export type ScoringMode = 'individual' | 'team' | 'combined'
export type RoundStatus = 'draft' | 'open' | 'completed' | 'locked'
export type ScoringFormat = 'individual_stroke_play' | 'team_scramble'

export interface Tournament {
  id: string
  name: string
  description: string
  start_date: string
  end_date: string
  number_of_rounds: number
  status: TournamentStatus
  scoring_mode: ScoringMode
  created_at: string
  updated_at: string
}

export interface Player {
  id: string
  display_name: string
  current_handicap_index: number
  email: string | null
  profile_image_ref: string | null
  active: boolean
  created_at: string
  updated_at: string
}

export interface TournamentPlayer {
  tournament_id: string
  player_id: string
  display_name: string
  tournament_handicap: number
  seed: number | null
  status: 'active' | 'withdrawn'
}

export interface Round {
  id: string
  tournament_id: string
  round_number: number
  name: string
  round_date: string
  course_name: string
  tee_name: string
  number_of_holes: number
  status: RoundStatus
  handicap_enabled: boolean
  handicap_allowance_percent: number
  scoring_format: ScoringFormat
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
