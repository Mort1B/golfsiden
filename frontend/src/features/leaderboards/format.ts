import type { LeaderboardMetric, ScoringFormat } from '../../api/types'

const scoringFormatLabels = {
  individual_stroke_play: 'Individuelt slagspill',
  team_scramble: 'Lag-scramble',
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
