import type { Round } from './types'
import { decodeExpectedRound } from './tournaments/decoders'
import { jsonRequest, requestDecoded } from './http'
import { decodeAcknowledgement } from './scorecards/conditional'
import { scoreRequest } from './scorecards/timeout'
import { decodeStablefordRead, decodeStablefordScoring } from './stableford/decoders'
import type { FourBallOperation } from './fourBall/contracts'
export * from './stableford/contracts'
const path = (round: string, team: string) => `/api/rounds/${round}/stableford/scorecards/${team}`
export const stablefordApi = {
  settings: (round: Round, enabled: boolean, allowance: number, csrf: string) => requestDecoded(`/api/rounds/${round.id}/stableford/settings`,
    value => decodeExpectedRound(value, round.id), jsonRequest('PUT', { expected_round_updated_at: round.updated_at, handicap_enabled: enabled, handicap_allowance_percent: allowance }, csrf)),
  read: (round: string, team: string) => requestDecoded(path(round, team), value => decodeStablefordRead(value, round, team)),
  scoring: (round: string, team: string, signal?: AbortSignal) => scoreRequest(child =>
    requestDecoded(`${path(round, team)}/scoring`, value => decodeStablefordScoring(value, round, team), { signal: child }), signal),
  confirm: (round: string, team: string, csrf: string, signal?: AbortSignal) => requestDecoded(`${path(round, team)}/confirm`,
    value => decodeStablefordScoring(value, round, team), { ...jsonRequest('POST', {}, csrf), signal }),
  save: (round: string, input: FourBallOperation, csrf: string, signal?: AbortSignal) => requestDecoded(
    `/api/rounds/${round}/stableford/inputs/conditional`, value => decodeAcknowledgement(value, input.request_id),
    { ...jsonRequest('PUT', input, csrf), signal }),
}
