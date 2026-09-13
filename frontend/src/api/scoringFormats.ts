import type { ScoringFormat } from './types'

export const SCORING_FORMATS = [
  'individual_stroke_play',
  'individual_stableford',
  'singles_match_play',
  'team_scramble',
  'two_player_foursomes',
  'four_ball_stroke_play',
] as const satisfies readonly ScoringFormat[]

const formatOwnership = {
  individual_stroke_play: 'player',
  individual_stableford: 'player',
  singles_match_play: 'player',
  team_scramble: 'team',
  two_player_foursomes: 'team',
  four_ball_stroke_play: 'team',
} as const satisfies Record<ScoringFormat, 'player' | 'team'>

const defaultHandicapAllowance = {
  individual_stroke_play: 100,
  individual_stableford: 100,
  singles_match_play: 100,
  team_scramble: 100,
  two_player_foursomes: 50,
  four_ball_stroke_play: 85,
} as const satisfies Record<ScoringFormat, number>

export function isScoringFormat(value: unknown): value is ScoringFormat {
  return typeof value === 'string' && SCORING_FORMATS.some((format) => format === value)
}

export function ownerTypeForScoringFormat(format: ScoringFormat): 'player' | 'team' {
  return formatOwnership[format]
}

export function isTeamScoringFormat(format: ScoringFormat): boolean {
  return ownerTypeForScoringFormat(format) === 'team'
}

export function defaultHandicapAllowanceForFormat(format: ScoringFormat): number {
  return defaultHandicapAllowance[format]
}

export function inputOwnerTypeForScoringFormat(format: ScoringFormat): 'player' | 'team' {
  return format === 'four_ball_stroke_play' ? 'player' : ownerTypeForScoringFormat(format)
}

export function contributesToOverall(format: ScoringFormat): boolean { return format !== 'singles_match_play' }
