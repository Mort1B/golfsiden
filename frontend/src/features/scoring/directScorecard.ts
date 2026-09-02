import type { Round, RoundLeaderboard, RoundLeaderboardEntry } from '../../api/types'
import type { ScoreOwner } from '../../api/scorecards'

export function matchingRound(rounds: Round[], tournamentId: string, roundId: string): Round | null {
  return rounds.find((round) => round.id === roundId && round.tournament_id === tournamentId) ?? null
}

export function projectedOwner(
  leaderboard: RoundLeaderboard,
  tournamentId: string,
  roundId: string,
  owner: ScoreOwner,
): RoundLeaderboardEntry | null {
  if (leaderboard.tournament_id !== tournamentId || leaderboard.round_id !== roundId) return null
  return leaderboard.entries.find((entry) => entry.owner.type === owner.type && entry.owner.id === owner.id) ?? null
}
