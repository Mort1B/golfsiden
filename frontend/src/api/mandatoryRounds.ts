import { invalidData } from './decoder'
import type { Round } from './types'

type MandatoryRoundDataLabel = 'opprettingsdata' | 'resultatdata' | 'turneringsdata'

export type MandatoryRoundMatch =
  | { state: 'none'; round: null }
  | { state: 'matched'; round: Round }
  | { state: 'missing'; round: null }

export function matchMandatoryRound(mandatoryRoundId: string | null, rounds: Round[]): MandatoryRoundMatch {
  if (mandatoryRoundId === null) return { state: 'none', round: null }
  const round = rounds.find((candidate) => candidate.id === mandatoryRoundId)
  return round === undefined ? { state: 'missing', round: null } : { state: 'matched', round }
}

export function validateMandatoryRound(
  mandatoryRoundId: string | null,
  rounds: Round[],
  label: MandatoryRoundDataLabel,
  path: string,
): Round | null {
  const match = matchMandatoryRound(mandatoryRoundId, rounds)
  if (match.state === 'missing') return invalidData(label, path)
  return match.round
}
