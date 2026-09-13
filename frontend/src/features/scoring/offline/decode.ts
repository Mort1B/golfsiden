import { decodeInteger, decodeObject, decodeString, decodeUuid, invalidData } from '../../../api/decoder'
import { decodeScoreOwner } from '../../../api/scorecards/cardDecoders'
import { decodeFourBallInput } from '../../../api/fourBall/decoders'
import { decodeScoreRevision } from '../../../api/scorecards/revision'
import type { ExpectedScore } from '../../../api/scorecards/conditional'
import { cardKey, queueKey, type ConfirmationLease, type PendingScore } from './model'

const invalid = (): never => invalidData('lokal score', 'lagring')
const str = (value: unknown) => decodeString(value, 'lagring')
const uuid = (value: unknown) => decodeUuid(value, 'lagring')
const integer = (value: unknown, min = 0, max = Number.MAX_SAFE_INTEGER) => decodeInteger(value, 'lagring', min, max)
export function decodeExpected(value: unknown): ExpectedScore {
  const data = decodeObject(value, 'lagring')
  if (data.type === 'absent' && Object.keys(data).length === 1) return { type: 'absent' }
  if (data.type === 'present' && Object.keys(data).length === 3) {
    return { type: 'present', score_id: uuid(data.score_id), revision: decodeScoreRevision(data.revision) }
  }
  return invalid()
}
export function decodeLease(value: unknown): ConfirmationLease {
  const data = decodeObject(value, 'lagring')
  return { key: str(data.key), id: uuid(data.id), until: integer(data.until) }
}
export function decodePending(value: unknown): PendingScore {
  const data = decodeObject(value, 'lagring')
  const head = decodeObject(data.head, 'lagring')
  const lease = data.lease === null ? null : decodeObject(data.lease, 'lagring')
  if (data.phase !== 'queued' && data.phase !== 'blocked' && data.phase !== 'conflict') return invalid()
  const base = {
    key: str(data.key), cardKey: str(data.cardKey), accountId: uuid(data.accountId), tournamentId: uuid(data.tournamentId),
    roundId: uuid(data.roundId), owner: decodeScoreOwner(data.owner, 'lagring'), holeId: uuid(data.holeId),
    holeNumber: integer(data.holeNumber, 1, 18), generation: uuid(data.generation),
    phase: data.phase,
    lease: lease === null ? null : { id: uuid(lease.id), until: integer(lease.until) },
    attempts: integer(data.attempts), retryAt: integer(data.retryAt),
  }
  const common = { request_id: uuid(head.request_id), hole_id: uuid(head.hole_id), owner: decodeScoreOwner(head.owner, 'lagring'), expected_score: decodeExpected(head.expected_score) }
  let item: PendingScore
  if (data.protocol === 'four_ball_v1' || data.protocol === 'stableford_v1') {
    if (base.owner.type !== 'player' || common.owner.type !== 'player' || Object.keys(head).length !== 5) return invalid()
    item = { ...base, phase: data.phase, ...(data.protocol === 'four_ball_v1' ? { protocol: 'four_ball_v1' as const, sideId: uuid(data.sideId) } : { protocol: 'stableford_v1' as const }), owner: base.owner,
      head: { request_id: common.request_id, hole_id: common.hole_id, owner: common.owner, input: decodeFourBallInput(head.input), expected_score: common.expected_score }, desired: decodeFourBallInput(data.desired) }
  } else {
    if (data.protocol !== undefined || Object.keys(head).length !== 5) return invalid()
    item = { ...base, phase: data.phase, head: { request_id: common.request_id, hole_id: common.hole_id, owner: common.owner, gross_strokes: integer(head.gross_strokes, 1, 20), expected_score: common.expected_score }, desired: integer(data.desired, 1, 20) }
  }
  if (item.key !== queueKey(item) || item.cardKey !== cardKey(item) || item.head.hole_id !== item.holeId
    || item.head.owner.id !== item.owner.id || item.head.owner.type !== item.owner.type) return invalid()
  return item
}
