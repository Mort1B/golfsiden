import { decodeObject, decodeUuid, invalidData } from '../decoder'
import { jsonRequest, requestDecoded } from '../http'
import type { ScoreEntry, ScoreOwner } from './contracts'
import { decodeScoreRevision } from './revision'

export type ExpectedScore = { type: 'absent' } | { type: 'present'; score_id: string; revision: string }
export interface ConditionalScore {
  request_id: string
  hole_id: string
  owner: ScoreOwner
  gross_strokes: number
  expected_score: ExpectedScore
}
export interface ScoreAcknowledgement {
  request_id: string
  applied_score: { score_id: string; revision: string }
}
export function expectedScore(score: ScoreEntry | null): ExpectedScore {
  return score === null ? { type: 'absent' } : { type: 'present', score_id: score.id, revision: score.revision }
}
export function decodeAcknowledgement(value: unknown, requestId: string): ScoreAcknowledgement {
  const data = decodeObject(value, 'acknowledgement', 'scorekortdata')
  const applied = decodeObject(data.applied_score, 'applied_score', 'scorekortdata')
  const request_id = decodeUuid(data.request_id, 'request_id', 'scorekortdata')
  if (request_id !== requestId) invalidData('scorekortdata', 'request_id')
  return { request_id, applied_score: {
    score_id: decodeUuid(applied.score_id, 'score_id', 'scorekortdata'),
    revision: decodeScoreRevision(applied.revision),
  } }
}
export function saveConditionalScore(roundId: string, input: ConditionalScore, csrfToken: string, signal?: AbortSignal): Promise<ScoreAcknowledgement> {
  return requestDecoded(`/api/rounds/${roundId}/scores/conditional`, value => decodeAcknowledgement(value, input.request_id),
    { ...jsonRequest('PUT', input, csrfToken), signal })
}
