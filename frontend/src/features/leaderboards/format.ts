import type {
  LeaderboardMetric,
  Round,
  ScoringFormat,
  TournamentContribution,
  TournamentLeaderboardEntry,
} from '../../api/types'
import type { ScoreVisibilityMode } from '../../api/visibility'

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
  return `${entry.counted_contributions} av ${requiredCount} fullførte tellende · ${completed} · ${eligibility}`
}

export function selectedProvisional(entry: TournamentLeaderboardEntry): TournamentContribution | null {
  return entry.contributions.find((contribution) => contribution.provisional && contribution.counted) ?? null
}

export function provisionalProgressLabel(contribution: TournamentContribution): string {
  const owner = contribution.owner.type === 'team' ? ` · ${contribution.owner_name}` : ''
  return `Foreløpig · ${contribution.holes_scored} av ${contribution.number_of_holes} hull${owner}`
}

export type MandatoryRoundDisplayState = 'completed' | 'open' | 'awaiting_release' | 'missing'

export function mandatoryRoundDisplayState(
  round: Round,
  rounds: Round[],
  visibility: ScoreVisibilityMode,
  contribution: TournamentContribution | undefined,
): MandatoryRoundDisplayState {
  if (contribution?.provisional === false) return 'completed'
  if (round.status === 'open') return 'open'
  const configuredFinal = rounds.reduce<Round | undefined>((latest, candidate) => {
    if (latest === undefined || candidate.round_number > latest.round_number) return candidate
    if (candidate.round_number < latest.round_number) return latest
    return candidate.id.toLowerCase() > latest.id.toLowerCase() ? candidate : latest
  }, undefined)
  if (configuredFinal?.id === round.id
    && visibility === 'front_nine'
    && (round.status === 'completed' || round.status === 'locked')) {
    return 'awaiting_release'
  }
  return 'missing'
}

export function mandatoryRoundProgressLabel(
  roundName: string,
  state: MandatoryRoundDisplayState,
  progress: { holesScored: number; numberOfHoles: number } | null = null,
): string {
  if (state === 'completed') return `${roundName}: fullført`
  if (state === 'missing') return `${roundName}: mangler`
  if (state === 'awaiting_release') return `${roundName}: avventer frigivelse`
  return progress === null
    ? `${roundName}: pågår · ingen score ennå`
    : `${roundName}: pågår · ${progress.holesScored} av ${progress.numberOfHoles} hull`
}
