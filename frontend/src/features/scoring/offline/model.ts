import type { ConditionalScore, ExpectedScore, ScoreAcknowledgement } from '../../../api/scorecards/conditional'
import { equalInput, inputLabel, type FourBallInput, type FourBallOperation } from '../../../api/fourBall/contracts'
import type { ScoreOwner } from '../../../api/scorecards'

export interface QueueTarget {
  accountId: string
  tournamentId: string
  roundId: string
  owner: ScoreOwner
  holeId: string
  holeNumber: number
}
export type QueuePhase = 'queued' | 'conflict' | 'blocked'
interface PendingBase extends QueueTarget {
  key: string
  cardKey: string
  generation: string
  phase: QueuePhase
  lease: { id: string; until: number } | null
  attempts: number
  retryAt: number
}
export interface LegacyPendingScore extends PendingBase {
  protocol?: undefined
  head: ConditionalScore
  desired: number
}
export interface FourBallTarget extends QueueTarget {
  protocol: 'four_ball_v1'
  sideId: string
  owner: { type: 'player'; id: string }
}
export interface FourBallPendingScore extends PendingBase {
  protocol: 'four_ball_v1'
  sideId: string
  owner: { type: 'player'; id: string }
  head: FourBallOperation
  desired: FourBallInput
}
export type PendingScore = LegacyPendingScore | FourBallPendingScore
export function pendingOwner(item: PendingScore): ScoreOwner {
  return item.protocol === 'four_ball_v1' ? { type: 'team', id: item.sideId } : item.owner
}
export function pendingLabel(item: PendingScore): string {
  return item.protocol === 'four_ball_v1' ? inputLabel(item.desired) : `${item.desired} slag`
}
export function fourBallOperation(target: FourBallTarget, desired: FourBallInput, expected: ExpectedScore): FourBallOperation {
  return { request_id: crypto.randomUUID(), hole_id: target.holeId, owner: target.owner, input: desired, expected_score: expected }
}
export function enqueueFourBall(current: PendingScore | null, target: FourBallTarget, desired: FourBallInput, expected: ExpectedScore): FourBallPendingScore {
  if (desired.type === 'numeric' && (!Number.isInteger(desired.gross_strokes) || desired.gross_strokes < 1 || desired.gross_strokes > 20)) throw new Error('Ugyldig antall slag')
  if (current && (current.protocol !== 'four_ball_v1' || current.sideId !== target.sideId)) throw new Error('Uforenlig lokal score')
  return current ? { ...current, desired, generation: crypto.randomUUID() } : {
    ...target, key: queueKey(target), cardKey: cardKey(target), generation: crypto.randomUUID(),
    head: fourBallOperation(target, desired, expected), desired, phase: 'queued', lease: null, attempts: 0, retryAt: 0,
  }
}
export function resolveOperation(item: PendingScore, expected: ExpectedScore): PendingScore {
  const state = { generation: crypto.randomUUID(), phase: 'queued' as const, lease: null, attempts: 0, retryAt: 0 }
  return item.protocol === 'four_ball_v1'
    ? { ...item, ...state, head: fourBallOperation(item, item.desired, expected) }
    : { ...item, ...state, head: operation(item, item.desired, expected) }
}
export interface ConfirmationLease { key: string; id: string; until: number }
export const LEASE_MS = 20_000
export const REQUEST_MS = 12_000
export function cardKey(target: Pick<QueueTarget, 'accountId' | 'roundId' | 'owner'>): string {
  return `${target.accountId}:${target.roundId}:${target.owner.type}:${target.owner.id}`
}
export function queueKey(target: Pick<QueueTarget, 'accountId' | 'roundId' | 'owner' | 'holeId'>): string {
  return `${cardKey(target)}:${target.holeId}`
}
export function operation(target: QueueTarget, desired: number, expected: ExpectedScore): ConditionalScore {
  return { request_id: crypto.randomUUID(), hole_id: target.holeId, owner: target.owner, gross_strokes: desired, expected_score: expected }
}
export function enqueue(current: PendingScore | null, target: QueueTarget, desired: number, expected: ExpectedScore): LegacyPendingScore {
  if (current?.protocol === 'four_ball_v1') throw new Error('Uforenlig lokal score')
  if (!Number.isInteger(desired) || desired < 1 || desired > 20) throw new Error('Ugyldig antall slag')
  // Even before dispatch, a persisted operation is immutable. Further input is a successor.
  return current ? { ...current, desired, generation: crypto.randomUUID() } : {
    ...target, key: queueKey(target), cardKey: cardKey(target), generation: crypto.randomUUID(),
    head: operation(target, desired, expected), desired, phase: 'queued', lease: null, attempts: 0, retryAt: 0,
  }
}
export function acknowledge(current: PendingScore, ack: ScoreAcknowledgement): PendingScore | null {
  if (current.head.request_id !== ack.request_id) return current
  if (current.protocol === 'four_ball_v1' ? equalInput(current.desired, current.head.input) : current.desired === current.head.gross_strokes) return null
  return resolveOperation(current, { type: 'present', ...ack.applied_score })
}
export function hasLease(item: PendingScore, now = Date.now()): boolean { return item.lease !== null && item.lease.until > now }
