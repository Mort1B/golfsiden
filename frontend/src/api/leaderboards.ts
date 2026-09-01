import type { LeaderboardMetric, Round, TournamentLeaderboard } from './types'
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

export function validateTournamentLeaderboardRounds(leaderboard: TournamentLeaderboard, rounds: Round[]): TournamentLeaderboard {
  validateMandatoryRound(leaderboard.mandatory_round_id, rounds, 'resultatdata', 'leaderboard.mandatory_round_id round identity')
  const roundIds = new Set(rounds.map((round) => round.id))
  if (leaderboard.included_round_ids.some((roundId) => !roundIds.has(roundId))
    || (leaderboard.current_round_id !== null && !roundIds.has(leaderboard.current_round_id))) {
    invalidData('resultatdata', 'leaderboard.round identity')
  }
  if (leaderboard.visibility.mode === 'front_nine') {
    const finalRound = rounds.reduce<Round | undefined>((latest, round) =>
      latest === undefined || round.round_number > latest.round_number ? round : latest, undefined)
    if (finalRound !== undefined && leaderboard.included_round_ids.includes(finalRound.id)) {
      invalidData('resultatdata', 'leaderboard.hidden final round')
    }
  }
  return leaderboard
}
