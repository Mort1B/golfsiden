import type { Round, TournamentContribution } from '../../api/types'
import type { ScoreVisibilityMode } from '../../api/visibility'
import { mandatoryRoundDisplayState, mandatoryRoundProgressLabel } from './format'

export interface HistoricalContribution {
  contribution: TournamentContribution
  round: Round
}

export function orderedPlayerContributions(
  contributions: TournamentContribution[],
  rounds: Round[],
): HistoricalContribution[] {
  const byId = new Map(rounds.map((round) => [round.id, round]))
  return contributions.flatMap((contribution) => {
    const round = byId.get(contribution.round_id)
    return round === undefined ? [] : [{ contribution, round }]
  }).sort((left, right) => left.round.round_number - right.round.round_number
    || left.round.id.localeCompare(right.round.id))
}

export function contributionStateLabels(contribution: TournamentContribution): string[] {
  return [
    contribution.counted ? 'Tellende' : 'Forkastet',
    contribution.provisional
      ? `Foreløpig · ${contribution.holes_scored} av ${contribution.number_of_holes} hull`
      : 'Fullført',
    contribution.mandatory ? 'Obligatorisk runde' : null,
  ].filter((state): state is string => state !== null)
}

export function mandatoryPlayerHistoryLabel(
  mandatoryRoundId: string | null,
  rounds: Round[],
  visibility: ScoreVisibilityMode,
  contributions: TournamentContribution[],
): string | null {
  if (mandatoryRoundId === null) return null
  const round = rounds.find((candidate) => candidate.id === mandatoryRoundId)
  if (round === undefined) return null
  const contribution = contributions.find((candidate) => candidate.round_id === mandatoryRoundId && candidate.mandatory)
  const state = mandatoryRoundDisplayState(round, rounds, visibility, contribution)
  return mandatoryRoundProgressLabel(
    round.name,
    state,
    contribution?.provisional === true
      ? { holesScored: contribution.holes_scored, numberOfHoles: contribution.number_of_holes }
      : null,
  )
}

export function hasPlayerHistoryBackgroundError(
  roundsError: Error | null,
  roundsData: Round[] | undefined,
  leaderboardError: Error | null,
  leaderboardData: unknown,
): boolean {
  return (roundsError !== null && roundsData !== undefined)
    || (leaderboardError !== null && leaderboardData !== undefined)
}
