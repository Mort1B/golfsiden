import { decodeInteger, decodeObject } from '../decoder'
import type { LeaderboardMetric, OverallResult, RoundLeaderboard, StablefordResult, TournamentContribution } from '../types'
import { decodeStablefordValues } from '../stableford/decoders'
import { invalidLeaderboard } from './shared'
function exclude(data: Record<string, unknown>, names: string[], path: string): void {
  if (names.some(key => key in data)) invalidLeaderboard(path)
}
export function decodeNativeValue(data: Record<string, unknown>, path: string): StablefordResult {
  exclude(data, ['gross_total', 'net_total', 'par_total', 'par_played', 'score_to_par'], path)
  const d = decodeObject(data.value, path)
  if (d.type !== 'stableford' || d.version !== 1 || data.number_of_holes !== 18) invalidLeaderboard(path)
  const values = Object.fromEntries(Object.entries(d).filter(([key]) => key !== 'type' && key !== 'version'))
  const resolved = decodeInteger(data.holes_scored, path, 0, 18)
  return { type: 'stableford', version: 1, ...decodeStablefordValues(values, resolved, path) }
}
export function decodeOverallValue(data: Record<string, unknown>, path: string): OverallResult {
  exclude(data, ['gross_total', 'net_total', 'par_total', 'score_to_par', 'tie_break_score_to_par'], path)
  const d = decodeObject(data.value, path)
  if (d.type !== 'overall_equivalent' || d.version !== 1 || Object.keys(d).length !== 6) invalidLeaderboard(path)
  return { type: 'overall_equivalent', version: 1, gross: decodeInteger(d.gross, path), net: decodeInteger(d.net, path),
    selected: decodeInteger(d.selected, path), tie_break: d.tie_break === null ? null : decodeInteger(d.tie_break, path) }
}
export function contributionEquivalent(item: TournamentContribution, metric: LeaderboardMetric): number {
  return item.value ? metric === 'gross' ? item.value.gross_equivalent : item.value.net_equivalent
    : (metric === 'gross' ? item.gross_total : item.net_total) - item.par_total
}
export function overallSelected(item: { value?: { selected: number } } | { score_to_par: number }): number {
  return 'score_to_par' in item ? item.score_to_par : item.value?.selected ?? invalidLeaderboard('value.selected')
}
export function overallTieBreak(item: { value?: { tie_break: number | null } } | { tie_break_score_to_par: number | null }): number | null {
  return 'tie_break_score_to_par' in item ? item.tie_break_score_to_par : item.value?.tie_break ?? null
}
export function validateNativeRanks(board: RoundLeaderboard): void {
  const points = (entry: RoundLeaderboard['entries'][number]) => entry.value
    ? board.metric === 'gross' ? entry.value.gross_points : entry.value.net_points : invalidLeaderboard('value')
  let unranked = false
  board.entries.forEach((entry, index, all) => {
    if (entry.holes_scored === 0) {
      unranked = true
      if (entry.position !== null || entry.tied || points(entry) !== 0) invalidLeaderboard('unstarted position')
      return
    }
    if (unranked) invalidLeaderboard('position')
    const prev = all[index - 1]; const next = all[index + 1]
    const samePrev = prev !== undefined && prev.holes_scored > 0 && points(prev) === points(entry)
    const sameNext = next !== undefined && next.holes_scored > 0 && points(next) === points(entry)
    if (entry.position !== (samePrev ? prev.position : index + 1) || entry.tied !== (samePrev || sameNext)
      || (prev && points(prev) < points(entry))) invalidLeaderboard('points position')
  })
}
