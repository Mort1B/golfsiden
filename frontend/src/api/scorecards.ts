import {
  decodeArray,
  decodeBoolean,
  decodeInteger,
  decodeObject,
  decodeString,
  decodeTimestamp,
  decodeUuid,
  invalidData,
} from './decoder'
import type { RoundStatus, ScoringFormat } from './types'
import { ownerTypeForScoringFormat } from './scoringFormats'

export type ScoreOwnerType = 'player' | 'team'
export type ScoreOwner = { type: 'player'; id: string } | { type: 'team'; id: string }

export interface ScoreEntry {
  id: string
  round_id: string
  hole_id: string
  owner: ScoreOwner
  gross_strokes: number
  submitted_by: string
  submitted_at: string
  updated_at: string
}

export interface ScorecardHole {
  hole_id: string
  hole_number: number
  par: number
  stroke_index: number
  score: ScoreEntry | null
  net_strokes: number | null
}

export interface ScorecardSummary {
  round_id: string
  owner: ScoreOwner
  holes: ScorecardHole[]
  gross_total: number
  net_total: number
  playing_handicap: number
  holes_scored: number
  number_of_holes: number
  complete: boolean
  confirmed: boolean
  confirmed_by: string | null
  confirmed_at: string | null
}

export interface OwnerCompletionProgress {
  owner: ScoreOwner
  owner_name: string
  holes_scored: number
  required_holes: number
  complete: boolean
  confirmed: boolean
}

export interface ScoreAccess {
  round_id: string
  writable_owners: ScoreOwner[]
}

export type CompletionIssueCode =
  | 'no_required_owners'
  | 'incomplete_scorecards'
  | 'unconfirmed_scorecards'
  | 'round_not_open'
  | 'round_not_completed'

export interface CompletionIssue {
  code: CompletionIssueCode
  message: string
}

export interface RoundCompletionValidation {
  round_id: string
  status: RoundStatus
  owners: OwnerCompletionProgress[]
  ready_to_complete: boolean
  ready_to_lock: boolean
  issues: CompletionIssue[]
}

export const scoringKeys = {
  access: (roundId: string) => ['rounds', roundId, 'score-access'] as const,
  completion: (roundId: string) => ['rounds', roundId, 'completion-validation'] as const,
  scorecard: (userId: string, roundId: string, owner: ScoreOwner) =>
    ['private-workspace', userId, 'rounds', roundId, 'scorecards', owner.type, owner.id] as const,
}

export function decodeScoreAccess(value: unknown, expectedRoundId: string): ScoreAccess {
  const data = decodeObject(value, 'access', 'scorekortdata')
  const roundId = decodeUuid(data.round_id, 'access.round_id', 'scorekortdata')
  const writableOwners = decodeArray(
    data.writable_owners,
    'access.writable_owners',
    owner,
    'scorekortdata',
  )
  const identities = new Set(writableOwners.map((item) => `${item.type}:${item.id}`))
  if (roundId !== expectedRoundId || identities.size !== writableOwners.length) invalid('access.identity')
  return { round_id: roundId, writable_owners: writableOwners }
}

export function ownerEquals(left: ScoreOwner, right: ScoreOwner): boolean {
  return left.type === right.type && left.id === right.id
}

export function ownerTypeForFormat(format: ScoringFormat): ScoreOwnerType {
  return ownerTypeForScoringFormat(format)
}

function invalid(path: string): never {
  return invalidData('scorekortdata', path)
}

function owner(value: unknown, path: string): ScoreOwner {
  const data = decodeObject(value, path, 'scorekortdata')
  if (data.type === 'player') return { type: 'player', id: decodeUuid(data.id, `${path}.id`, 'scorekortdata') }
  if (data.type === 'team') return { type: 'team', id: decodeUuid(data.id, `${path}.id`, 'scorekortdata') }
  return invalid(`${path}.type`)
}

function roundStatus(value: unknown, path: string): RoundStatus {
  if (value === 'draft' || value === 'open' || value === 'completed' || value === 'locked') return value
  return invalid(path)
}

function scoreEntry(value: unknown, path: string, expectedRoundId: string, expectedHoleId: string, expectedOwner: ScoreOwner): ScoreEntry {
  const data = decodeObject(value, path, 'scorekortdata')
  const decoded: ScoreEntry = {
    id: decodeUuid(data.id, `${path}.id`, 'scorekortdata'),
    round_id: decodeUuid(data.round_id, `${path}.round_id`, 'scorekortdata'),
    hole_id: decodeUuid(data.hole_id, `${path}.hole_id`, 'scorekortdata'),
    owner: owner(data.owner, `${path}.owner`),
    gross_strokes: decodeInteger(data.gross_strokes, `${path}.gross_strokes`, 1, 20, 'scorekortdata'),
    submitted_by: decodeUuid(data.submitted_by, `${path}.submitted_by`, 'scorekortdata'),
    submitted_at: decodeTimestamp(data.submitted_at, `${path}.submitted_at`, 'scorekortdata'),
    updated_at: decodeTimestamp(data.updated_at, `${path}.updated_at`, 'scorekortdata'),
  }
  if (decoded.round_id !== expectedRoundId || decoded.hole_id !== expectedHoleId || !ownerEquals(decoded.owner, expectedOwner)) {
    invalid(`${path}.identity`)
  }
  return decoded
}

function scorecardHole(value: unknown, path: string, roundId: string, expectedOwner: ScoreOwner): ScorecardHole {
  const data = decodeObject(value, path, 'scorekortdata')
  const holeId = decodeUuid(data.hole_id, `${path}.hole_id`, 'scorekortdata')
  const score = data.score === null ? null : scoreEntry(data.score, `${path}.score`, roundId, holeId, expectedOwner)
  const netStrokes = data.net_strokes === null
    ? null
    : decodeInteger(data.net_strokes, `${path}.net_strokes`, undefined, undefined, 'scorekortdata')
  if ((score === null) !== (netStrokes === null)) invalid(`${path}.net_strokes`)
  return {
    hole_id: holeId,
    hole_number: decodeInteger(data.hole_number, `${path}.hole_number`, 1, undefined, 'scorekortdata'),
    par: decodeInteger(data.par, `${path}.par`, 1, undefined, 'scorekortdata'),
    stroke_index: decodeInteger(data.stroke_index, `${path}.stroke_index`, 1, undefined, 'scorekortdata'),
    score,
    net_strokes: netStrokes,
  }
}

export function decodeScorecard(
  value: unknown,
  expectedRoundId: string,
  expectedOwner: ScoreOwner,
): ScorecardSummary {
  const data = decodeObject(value, 'scorecard', 'scorekortdata')
  const decodedOwner = owner(data.owner, 'scorecard.owner')
  const holes = decodeArray(
    data.holes,
    'scorecard.holes',
    (item, path) => scorecardHole(item, path, expectedRoundId, expectedOwner),
    'scorekortdata',
  )
  const decoded: ScorecardSummary = {
    round_id: decodeUuid(data.round_id, 'scorecard.round_id', 'scorekortdata'),
    owner: decodedOwner,
    holes,
    gross_total: decodeInteger(data.gross_total, 'scorecard.gross_total', 0, undefined, 'scorekortdata'),
    net_total: decodeInteger(data.net_total, 'scorecard.net_total', undefined, undefined, 'scorekortdata'),
    playing_handicap: decodeInteger(data.playing_handicap, 'scorecard.playing_handicap', undefined, undefined, 'scorekortdata'),
    holes_scored: decodeInteger(data.holes_scored, 'scorecard.holes_scored', 0, holes.length, 'scorekortdata'),
    number_of_holes: decodeInteger(data.number_of_holes, 'scorecard.number_of_holes', 1, undefined, 'scorekortdata'),
    complete: decodeBoolean(data.complete, 'scorecard.complete', 'scorekortdata'),
    confirmed: decodeBoolean(data.confirmed, 'scorecard.confirmed', 'scorekortdata'),
    confirmed_by: data.confirmed_by === null ? null : decodeUuid(data.confirmed_by, 'scorecard.confirmed_by', 'scorekortdata'),
    confirmed_at: data.confirmed_at === null ? null : decodeTimestamp(data.confirmed_at, 'scorecard.confirmed_at', 'scorekortdata'),
  }
  const scored = holes.filter((hole) => hole.score !== null).length
  const contiguous = holes.every((hole, index) => hole.hole_number === index + 1)
  const confirmationConsistent = decoded.confirmed
    ? decoded.confirmed_by !== null && decoded.confirmed_at !== null
    : decoded.confirmed_by === null && decoded.confirmed_at === null
  const grossTotal = holes.reduce((total, hole) => total + (hole.score?.gross_strokes ?? 0), 0)
  const netTotal = holes.reduce((total, hole) => total + (hole.net_strokes ?? 0), 0)
  const strokeIndexes = new Set(holes.map((hole) => hole.stroke_index))
  if (
    decoded.round_id !== expectedRoundId
    || !ownerEquals(decoded.owner, expectedOwner)
    || decoded.number_of_holes !== holes.length
    || decoded.holes_scored !== scored
    || decoded.complete !== (holes.length > 0 && scored === holes.length)
    || decoded.gross_total !== grossTotal
    || decoded.net_total !== netTotal
    || !confirmationConsistent
    || (decoded.confirmed && !decoded.complete)
    || !contiguous
    || strokeIndexes.size !== holes.length
    || holes.some((hole) => hole.stroke_index > holes.length)
  ) invalid('scorecard.identity')
  return decoded
}

function completionIssue(value: unknown, path: string): CompletionIssue {
  const data = decodeObject(value, path, 'scorekortdata')
  const code = data.code
  if (
    code !== 'no_required_owners'
    && code !== 'incomplete_scorecards'
    && code !== 'unconfirmed_scorecards'
    && code !== 'round_not_open'
    && code !== 'round_not_completed'
  ) invalid(`${path}.code`)
  return { code, message: decodeString(data.message, `${path}.message`, 'scorekortdata') }
}

function completionOwner(value: unknown, path: string, expectedType: ScoreOwnerType): OwnerCompletionProgress {
  const data = decodeObject(value, path, 'scorekortdata')
  const decodedOwner = owner(data.owner, `${path}.owner`)
  if (decodedOwner.type !== expectedType) invalid(`${path}.owner.type`)
  const required = decodeInteger(data.required_holes, `${path}.required_holes`, 1, undefined, 'scorekortdata')
  const scored = decodeInteger(data.holes_scored, `${path}.holes_scored`, 0, required, 'scorekortdata')
  const complete = decodeBoolean(data.complete, `${path}.complete`, 'scorekortdata')
  const confirmed = decodeBoolean(data.confirmed, `${path}.confirmed`, 'scorekortdata')
  if (complete !== (scored === required) || (confirmed && !complete)) invalid(`${path}.progress`)
  return {
    owner: decodedOwner,
    owner_name: decodeString(data.owner_name, `${path}.owner_name`, 'scorekortdata'),
    holes_scored: scored,
    required_holes: required,
    complete,
    confirmed,
  }
}

export function decodeCompletionValidation(
  value: unknown,
  expectedRoundId: string,
  expectedType: ScoreOwnerType,
): RoundCompletionValidation {
  const data = decodeObject(value, 'completion', 'scorekortdata')
  const decoded: RoundCompletionValidation = {
    round_id: decodeUuid(data.round_id, 'completion.round_id', 'scorekortdata'),
    status: roundStatus(data.status, 'completion.status'),
    owners: decodeArray(data.owners, 'completion.owners', (item, path) => completionOwner(item, path, expectedType), 'scorekortdata'),
    ready_to_complete: decodeBoolean(data.ready_to_complete, 'completion.ready_to_complete', 'scorekortdata'),
    ready_to_lock: decodeBoolean(data.ready_to_lock, 'completion.ready_to_lock', 'scorekortdata'),
    issues: decodeArray(data.issues, 'completion.issues', completionIssue, 'scorekortdata'),
  }
  const ownerIds = new Set(decoded.owners.map((item) => `${item.owner.type}:${item.owner.id}`))
  if (decoded.round_id !== expectedRoundId || ownerIds.size !== decoded.owners.length) invalid('completion.identity')
  return decoded
}

export function decodeSavedScore(
  value: unknown,
  roundId: string,
  holeId: string,
  expectedOwner: ScoreOwner,
  expectedStrokes: number,
): ScoreEntry {
  const decoded = scoreEntry(value, 'score', roundId, holeId, expectedOwner)
  if (decoded.gross_strokes !== expectedStrokes) invalid('score.gross_strokes')
  return decoded
}
