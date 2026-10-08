import { jsonRequest, requestDecoded, requestNoContent } from './http'
import { privateWorkspaceKeys } from './privateWorkspace'
import { decodeClaimPreview, decodeClaimReceipt, decodeClaimRegistration, decodePlayerAccounts, type ClaimPreview } from './playerClaimDecoders'
export { decodeClaimPreview, decodeClaimReceipt, decodeClaimRegistration, decodePlayerAccounts, claimTokenPattern } from './playerClaimDecoders'
export type { ClaimPreview, ClaimReceipt, ClaimRegistration, PlayerAccount } from './playerClaimDecoders'
export const playerAccountKey = (user: string, tournament: string) => [...privateWorkspaceKeys.user(user), 'tournaments', tournament, 'player-accounts'] as const
const base = (t: string) => `/api/tournaments/${encodeURIComponent(t)}`
const player = (t: string, p: string) => `${base(t)}/players/${encodeURIComponent(p)}`
const publicPath = (id: string) => `/api/player-claims/${encodeURIComponent(id)}`
const post = (data: unknown, csrf?: string): RequestInit => ({ ...jsonRequest('POST', data, csrf), cache: 'no-store', referrerPolicy: 'no-referrer' })
const remove = (csrf: string): RequestInit => ({ method: 'DELETE', headers: { 'x-csrf-token': csrf }, cache: 'no-store' })
export const playerClaimsApi = {
  accounts: (t: string, signal?: AbortSignal) => requestDecoded(`${base(t)}/player-accounts`, decodePlayerAccounts, { signal, cache: 'no-store' }),
  create: (t: string, display_name: string, handicap_index: number, csrf: string) => requestDecoded(`${base(t)}/players`, decodeClaimReceipt, post({ display_name, handicap_index }, csrf)),
  reissue: (t: string, p: string, csrf: string) => requestDecoded(`${player(t, p)}/claim`, value => decodeClaimReceipt(value, p), post({}, csrf)),
  revoke: (t: string, p: string, csrf: string) => requestNoContent(`${player(t, p)}/claim`, remove(csrf)),
  withdraw: (t: string, p: string, csrf: string) => requestNoContent(player(t, p), remove(csrf)),
  preview: (id: string, token: string, signal?: AbortSignal) => requestDecoded(`${publicPath(id)}/preview`, decodeClaimPreview, { ...post({ token }), signal }),
  register: (id: string, token: string, username: string, password: string, expected: ClaimPreview) => requestDecoded(`${publicPath(id)}/register`, value => decodeClaimRegistration(value, expected), post({ token, account: { username, password } })),
}
