import type { Round, TournamentLeaderboard, TournamentLeaderboardEntry } from '../types'
import { invalidLeaderboard } from './shared'

function hasSelected(entry: TournamentLeaderboardEntry): boolean {
  return entry.contributions.some((item) => item.counted)
}

function samePrimary(left: TournamentLeaderboardEntry, right: TournamentLeaderboardEntry): boolean {
  return hasSelected(left) === hasSelected(right)
    && left.counted_contributions === right.counted_contributions
    && left.score_to_par === right.score_to_par
}

function sameRank(left: TournamentLeaderboardEntry, right: TournamentLeaderboardEntry): boolean {
  return samePrimary(left, right) && left.tie_break_score_to_par === right.tie_break_score_to_par
}

function groups(entries: TournamentLeaderboardEntry[]): TournamentLeaderboardEntry[][] {
  const result: TournamentLeaderboardEntry[][] = []
  for (const entry of entries) {
    const group = result.at(-1)
    const previous = group?.[0]
    if (group && previous && samePrimary(previous, entry)) group.push(entry)
    else result.push([entry])
  }
  return result
}

function selectedProgress(entry: TournamentLeaderboardEntry): number {
  return entry.contributions.filter((item) => item.counted && item.provisional)
    .reduce((sum, item) => sum + item.holes_scored, 0)
}

export function validateTournamentRanks(leaderboard: TournamentLeaderboard): void {
  const entries = leaderboard.entries
  let rankedCount = entries.findIndex((entry) => !hasSelected(entry))
  if (rankedCount === -1) rankedCount = entries.length
  if (entries.slice(rankedCount).some((entry) => hasSelected(entry) || entry.position !== null || entry.tied)) {
    invalidLeaderboard('leaderboard.entries.position')
  }
  for (const group of groups(entries)) {
    const compared = group.filter((entry) => entry.tie_break_score_to_par !== null)
    if (compared.length > 0 && (leaderboard.tie_break_policy !== 'final_round_score'
      || group.length < 2 || compared.length !== group.length
      || group.some((entry) => !entry.eligible || !hasSelected(entry)
        || entry.contributions.some((item) => item.counted && item.provisional)))) {
      invalidLeaderboard('leaderboard.entries.tie_break_score_to_par')
    }
  }
  for (let index = 0; index < rankedCount; index += 1) {
    const entry = entries[index]
    if (!entry) invalidLeaderboard('leaderboard.entries.position')
    const previous = entries[index - 1]
    const next = index + 1 < rankedCount ? entries[index + 1] : undefined
    const samePrevious = previous !== undefined && sameRank(previous, entry)
    const sameNext = next !== undefined && sameRank(entry, next)
    if (entry.position !== (samePrevious ? previous.position : index + 1)
      || entry.tied !== (samePrevious || sameNext)) invalidLeaderboard(`leaderboard.entries[${index}].position`)
    if (previous && (previous.counted_contributions < entry.counted_contributions
      || (previous.counted_contributions === entry.counted_contributions
        && (previous.score_to_par > entry.score_to_par
          || (previous.score_to_par === entry.score_to_par
            && ((previous.tie_break_score_to_par !== null && entry.tie_break_score_to_par !== null
              && previous.tie_break_score_to_par > entry.tie_break_score_to_par)
              || (samePrevious && selectedProgress(previous) < selectedProgress(entry)))))))) {
      invalidLeaderboard(`leaderboard.entries[${index}].position`)
    }
  }
}

// Validate explanatory metadata against the same visible scheduled final that
// the server used. This validates ranks; it never sorts or ranks client-side.
export function validateTournamentTieBreakRounds(leaderboard: TournamentLeaderboard, rounds: Round[]): void {
  const final = rounds.find((round) => round.round_number === leaderboard.final_round_number)
  const finalVisible = leaderboard.visibility.mode === 'full'
    && final !== undefined && (final.status === 'completed' || final.status === 'locked')
  for (const group of groups(leaderboard.entries)) {
    const comparable = leaderboard.tie_break_policy === 'final_round_score' && group.length > 1 && finalVisible
      && group.every((entry) => entry.eligible && hasSelected(entry)
        && !entry.contributions.some((item) => item.counted && item.provisional)
        && entry.contributions.some((item) => item.round_id === final.id && !item.provisional
          && item.holes_scored === item.number_of_holes))
    for (const entry of group) {
      const expected = comparable
        ? entry.contributions.find((item) => item.round_id === final?.id)?.score_to_par
        : null
      if (entry.tie_break_score_to_par !== expected) invalidLeaderboard('leaderboard.entries.tie_break_score_to_par final round')
    }
  }
}
