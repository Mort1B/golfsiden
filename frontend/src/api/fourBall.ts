import { jsonRequest, requestDecoded } from './http'
import { decodeAcknowledgement } from './scorecards/conditional'
import { scoreRequest } from './scorecards/timeout'
import { decodeFourBallRead, decodeFourBallScoring } from './fourBall/decoders'
import type { FourBallOperation } from './fourBall/contracts'
export * from './fourBall/contracts'
const path = (round: string, team: string) => `/api/rounds/${round}/four-ball/scorecards/${team}`
export const fourBallApi = {
  read: (round: string, team: string) => requestDecoded(path(round, team), value => decodeFourBallRead(value, round, team)),
  scoring: (round: string, team: string, signal?: AbortSignal) => scoreRequest(child =>
    requestDecoded(`${path(round, team)}/scoring`, value => decodeFourBallScoring(value, round, team), { signal: child }), signal),
  confirm: (round: string, team: string, csrf: string, signal?: AbortSignal) => requestDecoded(`${path(round, team)}/confirm`,
    value => decodeFourBallScoring(value, round, team), { ...jsonRequest('POST', {}, csrf), signal }),
  save: (round: string, input: FourBallOperation, csrf: string, signal?: AbortSignal) => requestDecoded(
    `/api/rounds/${round}/four-ball/inputs/conditional`, value => decodeAcknowledgement(value, input.request_id),
    { ...jsonRequest('PUT', input, csrf), signal }),
}
