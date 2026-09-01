import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeUuid, invalidData } from './decoder'
import type { RoundStatus, ScoringFormat } from './types'
import { ownerTypeForScoringFormat } from './scoringFormats'
import { decodeScoreEntry, decodeScoreOwner } from './scorecards/cardDecoders'
import type { CompletionIssue, OwnerCompletionProgress, RoundCompletionValidation, ScoreAccess, ScoreOwner, ScoreOwnerType } from './scorecards/contracts'
import { decodeScoreVisibility, type ScoreVisibility } from './visibility'

export * from './scorecards/contracts'
export { decodeReadScorecard, decodeScoringScorecard } from './scorecards/cardDecoders'

export const scoringKeys = {
  access: (roundId: string) => ['rounds', roundId, 'score-access'] as const,
  completion: (roundId: string) => ['rounds', roundId, 'completion-validation'] as const,
  read: (userId: string, roundId: string, owner: ScoreOwner) =>
    ['private-workspace', userId, 'rounds', roundId, 'scorecards', 'read', owner.type, owner.id] as const,
  scoring: (userId: string, roundId: string, owner: ScoreOwner) =>
    ['private-workspace', userId, 'rounds', roundId, 'scorecards', 'scoring', owner.type, owner.id] as const,
}

function invalid(path: string): never { return invalidData('scorekortdata', path) }

export function decodeScoreAccess(value: unknown, expectedRoundId: string): ScoreAccess {
  const data = decodeObject(value, 'access', 'scorekortdata')
  const roundId = decodeUuid(data.round_id, 'access.round_id', 'scorekortdata')
  const writableOwners = decodeArray(data.writable_owners, 'access.writable_owners', decodeScoreOwner, 'scorekortdata')
  const identities = new Set(writableOwners.map((item) => `${item.type}:${item.id}`))
  if (roundId !== expectedRoundId || identities.size !== writableOwners.length) invalid('access.identity')
  return { round_id: roundId, writable_owners: writableOwners }
}

export function ownerTypeForFormat(format: ScoringFormat): ScoreOwnerType { return ownerTypeForScoringFormat(format) }

function roundStatus(value: unknown, path: string): RoundStatus {
  if (value === 'draft' || value === 'open' || value === 'completed' || value === 'locked') return value
  return invalid(path)
}

function completionIssue(value: unknown, path: string): CompletionIssue {
  const data = decodeObject(value, path, 'scorekortdata')
  const code = data.code
  if (code !== 'no_required_owners' && code !== 'incomplete_scorecards' && code !== 'unconfirmed_scorecards'
    && code !== 'round_not_open' && code !== 'round_not_completed') invalid(`${path}.code`)
  return { code, message: decodeString(data.message, `${path}.message`, 'scorekortdata') }
}

function nullableBoolean(value: unknown, path: string): boolean | null {
  return value === null ? null : decodeBoolean(value, path, 'scorekortdata')
}

function completionOwner(value: unknown, path: string, expectedType: ScoreOwnerType, visibility: ScoreVisibility): OwnerCompletionProgress {
  const data = decodeObject(value, path, 'scorekortdata')
  const decodedOwner = decodeScoreOwner(data.owner, `${path}.owner`)
  if (decodedOwner.type !== expectedType) invalid(`${path}.owner.type`)
  const required = decodeInteger(data.required_holes, `${path}.required_holes`, 1, undefined, 'scorekortdata')
  const scored = decodeInteger(data.holes_scored, `${path}.holes_scored`, 0, required, 'scorekortdata')
  const complete = nullableBoolean(data.complete, `${path}.complete`)
  const confirmed = nullableBoolean(data.confirmed, `${path}.confirmed`)
  if (visibility.mode === 'front_nine') {
    if (required !== 9 || complete !== null || confirmed !== null) invalid(`${path}.progress`)
  } else if (complete === null || confirmed === null
    || complete !== (scored === required) || (confirmed && !complete)) invalid(`${path}.progress`)
  return { owner: decodedOwner, owner_name: decodeString(data.owner_name, `${path}.owner_name`, 'scorekortdata'), holes_scored: scored, required_holes: required, complete, confirmed }
}

export function decodeCompletionValidation(value: unknown, expectedRoundId: string, expectedType: ScoreOwnerType): RoundCompletionValidation {
  const data = decodeObject(value, 'completion', 'scorekortdata')
  const visibility = decodeScoreVisibility(data.visibility, 'completion.visibility', 'scorekortdata')
  const decoded: RoundCompletionValidation = {
    round_id: decodeUuid(data.round_id, 'completion.round_id', 'scorekortdata'),
    status: roundStatus(data.status, 'completion.status'),
    owners: decodeArray(data.owners, 'completion.owners', (item, path) => completionOwner(item, path, expectedType, visibility), 'scorekortdata'),
    ready_to_complete: nullableBoolean(data.ready_to_complete, 'completion.ready_to_complete'),
    ready_to_lock: nullableBoolean(data.ready_to_lock, 'completion.ready_to_lock'),
    issues: decodeArray(data.issues, 'completion.issues', completionIssue, 'scorekortdata'),
    visibility,
  }
  const ownerIds = new Set(decoded.owners.map((item) => `${item.owner.type}:${item.owner.id}`))
  if (decoded.round_id !== expectedRoundId || ownerIds.size !== decoded.owners.length) invalid('completion.identity')
  validateCompletionProjection(decoded)
  return decoded
}

function validateCompletionProjection(decoded: RoundCompletionValidation): void {
  if (decoded.visibility.mode === 'front_nine') {
    if (decoded.status === 'draft' || decoded.ready_to_complete !== null
      || decoded.ready_to_lock !== null || decoded.issues.length !== 0) invalid('completion.visibility')
    return
  }
  if (decoded.ready_to_complete === null || decoded.ready_to_lock === null) invalid('completion.visibility')
  const hasOwners = decoded.owners.length > 0
  const allComplete = hasOwners && decoded.owners.every((owner) => owner.complete === true)
  const allConfirmed = hasOwners && decoded.owners.every((owner) => owner.confirmed === true)
  if (decoded.ready_to_complete !== (decoded.status === 'open' && allComplete && allConfirmed)
    || decoded.ready_to_lock !== (decoded.status === 'completed' && allComplete && allConfirmed)) invalid('completion.readiness')
  const actualCodes = decoded.issues.map((issue) => issue.code)
  if (new Set(actualCodes).size !== actualCodes.length) invalid('completion.issues')
  const expectedCodes: CompletionIssue['code'][] = []
  if (!hasOwners) expectedCodes.push('no_required_owners')
  if (hasOwners && !allComplete) expectedCodes.push('incomplete_scorecards')
  if (hasOwners && !allConfirmed) expectedCodes.push('unconfirmed_scorecards')
  if (decoded.status !== 'open') expectedCodes.push('round_not_open')
  if (decoded.status !== 'completed') expectedCodes.push('round_not_completed')
  if (actualCodes.length !== expectedCodes.length || expectedCodes.some((code) => !actualCodes.includes(code))) invalid('completion.issues')
}

export function decodeSavedScore(value: unknown, roundId: string, holeId: string, expectedOwner: ScoreOwner, expectedStrokes: number) {
  const decoded = decodeScoreEntry(value, 'score', roundId, holeId, expectedOwner)
  if (decoded.gross_strokes !== expectedStrokes) invalid('score.gross_strokes')
  return decoded
}
