import { describe, expect, it } from 'vitest'
import { decodeScoreRevision } from './revision'
import { decodeAcknowledgement } from './conditional'
const id = '00000000-0000-0000-0000-000000000001'
describe('conditional score boundary', () => {
  it.each(['1', '9007199254740993', '9223372036854775807'])('preserves revision %s losslessly', value => {
    expect(decodeScoreRevision(value)).toBe(value)
    expect(decodeAcknowledgement({ request_id: id, applied_score: { score_id: id, revision: value } }, id).applied_score.revision).toBe(value)
  })
  it.each([undefined, null, 1, '0', '01', '-1', '+1', ' 1', '1e2', '9223372036854775808'])('rejects malformed revision %#', value => {
    expect(() => decodeScoreRevision(value)).toThrow('revision')
  })
  it('requires exact request identity', () => {
    expect(() => decodeAcknowledgement({ request_id: id, applied_score: { score_id: id, revision: '1' } }, crypto.randomUUID())).toThrow('request_id')
  })
})
