import type { ScoringFormat } from './types'

export const SCORING_FORMATS = [
  'individual_stroke_play',
  'team_scramble',
  'two_player_foursomes',
] as const satisfies readonly ScoringFormat[]

const formatOwnership = {
  individual_stroke_play: 'player',
  team_scramble: 'team',
  two_player_foursomes: 'team',
} as const satisfies Record<ScoringFormat, 'player' | 'team'>

const defaultHandicapAllowance = {
  individual_stroke_play: 100,
  team_scramble: 100,
  two_player_foursomes: 50,
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
