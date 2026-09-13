import { invalidData } from './decoder'
import type { TournamentTieBreakPolicy } from './types'

export function isTieBreakPolicy(value: unknown): value is TournamentTieBreakPolicy {
  return value === 'shared_positions' || value === 'final_round_score'
}

export function decodeTieBreakPolicy(value: unknown, path: string, label: string): TournamentTieBreakPolicy {
  return isTieBreakPolicy(value) ? value : invalidData(label, path)
}
