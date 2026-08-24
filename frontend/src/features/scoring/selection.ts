import type { OwnerCompletionProgress, ScoreOwner, ScoreOwnerType } from '../../api/scorecards'
import type { Round } from '../../api/types'
import { ownerTypeForScoringFormat } from '../../api/scoringFormats'

export type ScoreView = 'hole' | 'summary'
export type ScoreHistoryAction = 'automatic' | 'previous' | 'next' | 'tournament' | 'round' | 'owner' | 'hole' | 'view'

export interface ScoreSelection {
  tournamentId: string
  roundId?: string
  owner?: ScoreOwner
  holeNumber?: number
  view: ScoreView
}

export function preferredScoreRound(rounds: Round[]): Round | undefined {
  const latest = (status: Round['status']) => rounds
    .filter((round) => round.status === status)
    .sort((left, right) => right.round_number - left.round_number)[0]
  return latest('open') ?? latest('completed') ?? latest('locked')
}

export function scoreableRounds(rounds: Round[]): Round[] {
  return rounds.filter((round) => round.status !== 'draft')
}

export function selectedOwner(
  owners: OwnerCompletionProgress[],
  requestedType: string | null,
  requestedId: string | null,
  writableOwners: ScoreOwner[] = [],
): OwnerCompletionProgress | undefined {
  const requested = owners.find((item) =>
    item.owner.type === requestedType && item.owner.id === requestedId)
  if (requested) return requested
  return owners.find((item) => writableOwners.some((writable) =>
    writable.type === item.owner.type && writable.id === item.owner.id)) ?? owners[0]
}

export function parseScoreView(value: string | null): ScoreView {
  return value === 'summary' ? 'summary' : 'hole'
}

export function parseHoleNumber(value: string | null): number | undefined {
  if (!value || !/^\d+$/.test(value)) return undefined
  const parsed = Number(value)
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : undefined
}

export function scoringSearch(selection: ScoreSelection): URLSearchParams {
  const params = new URLSearchParams()
  params.set('tournament', selection.tournamentId)
  if (selection.roundId) params.set('round', selection.roundId)
  if (selection.owner) {
    params.set('owner_type', selection.owner.type)
    params.set('owner', selection.owner.id)
  }
  if (selection.holeNumber !== undefined) params.set('hole', String(selection.holeNumber))
  params.set('view', selection.view)
  return params
}

export function replaceScoreHistory(action: ScoreHistoryAction): boolean {
  return action === 'automatic' || action === 'previous' || action === 'next'
}

export function expectedOwnerType(round: Round): ScoreOwnerType {
  return ownerTypeForScoringFormat(round.scoring_format)
}
