import type { LeaderboardMetric, ScoringFormat, TournamentLeaderboardEntry } from '../../api/types'

const scoringFormatLabels = {
  individual_stroke_play: 'Individuelt slagspill',
  team_scramble: 'Lag-scramble',
  two_player_foursomes: 'Foursomes (to spillere)',
} satisfies Record<ScoringFormat, string>

export function scoringFormatLabel(format: ScoringFormat): string {
  return scoringFormatLabels[format]
}

export function metricLabel(metric: LeaderboardMetric): string {
  return metric === 'gross' ? 'Brutto' : 'Netto'
}

export function positionLabel(position: number | null, tied: boolean): string {
  if (position === null) return '–'
  return tied ? `T${position}` : String(position)
}

export function scoreToParLabel(score: number): string {
  if (score === 0) return 'E'
  return score > 0 ? `+${score}` : String(score)
}

export function roundsLabel(count: number): string {
  return `${count} ${count === 1 ? 'fullført runde' : 'fullførte runder'}`
}

export function bestRoundsProgressLabel(entry: TournamentLeaderboardEntry, requiredCount: number): string {
  const completed = entry.completed_rounds === 0 ? 'Ingen fullførte runder' : roundsLabel(entry.completed_rounds)
  const eligibility = entry.eligible ? 'Kvalifisert' : 'Ikke kvalifisert ennå'
  return `Beste ${requiredCount} · ${entry.counted_contributions} av ${requiredCount} tellende · ${completed} · ${eligibility}`
}
