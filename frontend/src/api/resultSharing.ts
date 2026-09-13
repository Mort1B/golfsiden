import { ApiHttpError, jsonRequest, requestDecoded, requestNoContent } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import type { LeaderboardMetric } from './types'
import { decodePublicResults, decodeShareReceipt, decodeShareStatus } from './resultSharing/decoders'
export type * from './resultSharing/contracts'

export const resultShareKey = (userId: string, tournamentId: string) => [...privateWorkspaceKeys.user(userId), 'result-share', tournamentId] as const
const target = (id: string) => `/api/tournaments/${encodeURIComponent(id)}/result-share`
export const resultSharingApi = {
  status: (id: string) => requestDecoded(target(id), value => decodeShareStatus(value, id), { cache: 'no-store' }),
  issue: (id: string, expectedGrantId: string | null, csrf: string) => requestDecoded(target(id), value => decodeShareReceipt(value, id),
    { ...jsonRequest('POST', { expected_grant_id: expectedGrantId }, csrf), cache: 'no-store', referrerPolicy: 'no-referrer' }),
  revoke: (id: string, grantId: string, csrf: string) => requestNoContent(`${target(id)}/${encodeURIComponent(grantId)}`,
    { method: 'DELETE', headers: { 'x-csrf-token': csrf }, cache: 'no-store', referrerPolicy: 'no-referrer' }),
  read: async (id: string, token: string, metric: LeaderboardMetric, signal: AbortSignal) => {
    const timeout = new AbortController()
    const timer = setTimeout(() => timeout.abort(), 12_000)
    try {
      const result = await requestDecoded(`/api/public/results/${encodeURIComponent(id)}`, value => decodePublicResults(value, id, metric), {
        ...jsonRequest('POST', { token, metric }), credentials: 'omit', cache: 'no-store', referrerPolicy: 'no-referrer',
        signal: AbortSignal.any([signal, timeout.signal]),
      })
      signal.throwIfAborted()
      return result
    } catch (error) {
      if (error instanceof ApiHttpError && error.status === 404) throw new ApiHttpError(404, 'result_share_unavailable', 'Resultatlenken er ikke tilgjengelig.')
      throw new Error('Kunne ikke hente delte resultater. Prøv igjen.')
    } finally { clearTimeout(timer) }
  },
}
