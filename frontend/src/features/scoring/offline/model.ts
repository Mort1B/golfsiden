import type { ConditionalScore, ExpectedScore, ScoreAcknowledgement } from '../../../api/scorecards/conditional'
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
export interface PendingScore extends QueueTarget {
  key: string
  cardKey: string
  generation: string
  head: ConditionalScore
  desired: number
  phase: QueuePhase
  lease: { id: string; until: number } | null
  attempts: number
  retryAt: number
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
export function enqueue(current: PendingScore | null, target: QueueTarget, desired: number, expected: ExpectedScore): PendingScore {
  if (!Number.isInteger(desired) || desired < 1 || desired > 20) throw new Error('Ugyldig antall slag')
  // Even before dispatch, a persisted operation is immutable. Further input is a successor.
  return current ? { ...current, desired, generation: crypto.randomUUID() } : {
    ...target, key: queueKey(target), cardKey: cardKey(target), generation: crypto.randomUUID(),
    head: operation(target, desired, expected), desired, phase: 'queued', lease: null, attempts: 0, retryAt: 0,
  }
}
export function acknowledge(current: PendingScore, ack: ScoreAcknowledgement): PendingScore | null {
  if (current.head.request_id !== ack.request_id) return current
  if (current.desired === current.head.gross_strokes) return null
  return { ...current, head: operation(current, current.desired, { type: 'present', ...ack.applied_score }),
    generation: crypto.randomUUID(), phase: 'queued', lease: null, attempts: 0, retryAt: 0 }
}
export function hasLease(item: PendingScore, now = Date.now()): boolean { return item.lease !== null && item.lease.until > now }
