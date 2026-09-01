import type { LeaderboardMetric, Round, TournamentContribution, TournamentLeaderboard } from './types'
import { privateWorkspaceKeys } from './privateWorkspace'
import { validateMandatoryRound } from './mandatoryRounds'
import { invalidData } from './decoder'

export { decodeRoundLeaderboard } from './leaderboards/roundDecoder'
export { decodeTournamentLeaderboard } from './leaderboards/tournamentDecoder'

export const leaderboardKeys = {
  round: (userId: string, roundId: string, metric: LeaderboardMetric) =>
    [...privateWorkspaceKeys.user(userId), 'leaderboards', 'round', roundId, metric] as const,
  tournament: (userId: string, tournamentId: string, metric: LeaderboardMetric) =>
    [...privateWorkspaceKeys.user(userId), 'leaderboards', 'tournament', tournamentId, metric] as const,
}

function contributionScore(contribution: TournamentContribution, metric: LeaderboardMetric): number {
  return (metric === 'gross' ? contribution.gross_total : contribution.net_total) - contribution.par_total
}

function compareRoundOrder(left: Round, right: Round): number {
  if (left.round_number !== right.round_number) return left.round_number - right.round_number
  const leftId = left.id.toLowerCase()
  const rightId = right.id.toLowerCase()
  return leftId < rightId ? -1 : leftId > rightId ? 1 : 0
}

function validateDisplayedSelection(
  leaderboard: TournamentLeaderboard,
  roundsById: Map<string, Round>,
): void {
  const optionalSlots = leaderboard.required_counted_rounds - Number(leaderboard.mandatory_round_id !== null)
  leaderboard.entries.forEach((entry, entryIndex) => {
    const optional = entry.contributions
      .filter((contribution) => !contribution.mandatory)
      .sort((left, right) => {
        const leftRound = roundsById.get(left.round_id)
        const rightRound = roundsById.get(right.round_id)
        if (leftRound === undefined || rightRound === undefined) invalidData('resultatdata', 'leaderboard.round identity')
        return contributionScore(left, leaderboard.metric) - contributionScore(right, leaderboard.metric)
          || compareRoundOrder(leftRound, rightRound)
      })
    const expected = new Set(optional.slice(0, optionalSlots).map((contribution) => contribution.round_id))
    const displayed = entry.contributions.filter((contribution) => contribution.counted && !contribution.mandatory)
    if (displayed.length !== expected.size || displayed.some((contribution) => !expected.has(contribution.round_id))) {
      invalidData('resultatdata', `leaderboard.entries[${entryIndex}].contributions counted selection`)
    }
  })
}

export function validateTournamentLeaderboardRounds(leaderboard: TournamentLeaderboard, rounds: Round[]): TournamentLeaderboard {
  validateMandatoryRound(leaderboard.mandatory_round_id, rounds, 'resultatdata', 'leaderboard.mandatory_round_id round identity')
  const roundsById = new Map(rounds.map((round) => [round.id, round]))
  for (const roundId of leaderboard.included_round_ids) {
    const round = roundsById.get(roundId)
    if (round === undefined || (round.status !== 'completed' && round.status !== 'locked')) {
      invalidData('resultatdata', 'leaderboard.included_round_ids status')
    }
  }
  const expectedCurrent = rounds
    .filter((round) => round.status === 'open')
    .reduce<Round | null>((highest, round) =>
      highest === null || compareRoundOrder(highest, round) < 0 ? round : highest, null)
  if (leaderboard.current_round_id !== (expectedCurrent?.id ?? null)) {
    invalidData('resultatdata', 'leaderboard.current_round_id status')
  }
  for (const entry of leaderboard.entries) {
    for (const contribution of entry.contributions) {
      const round = roundsById.get(contribution.round_id)
      if (round === undefined || contribution.number_of_holes !== round.number_of_holes) {
        invalidData('resultatdata', 'leaderboard.contribution round identity')
      }
    }
  }
  if (leaderboard.visibility.mode === 'front_nine') {
    const finalRound = rounds.reduce<Round | undefined>((latest, round) =>
      latest === undefined || round.round_number > latest.round_number ? round : latest, undefined)
    if (finalRound !== undefined && leaderboard.included_round_ids.includes(finalRound.id)) {
      invalidData('resultatdata', 'leaderboard.hidden final round')
    }
    if (finalRound !== undefined && leaderboard.current_round_id === finalRound.id
      && leaderboard.entries.some((entry) => entry.contributions.some((contribution) =>
        contribution.provisional && contribution.holes_scored > 9))) {
      invalidData('resultatdata', 'leaderboard.hidden final progress')
    }
  }
  validateDisplayedSelection(leaderboard, roundsById)
  return leaderboard
}
