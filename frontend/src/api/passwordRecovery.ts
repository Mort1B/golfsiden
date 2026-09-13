import { decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from './decoder'
import { jsonRequest, requestDecoded, requestNoContent } from './http'

export interface RecoveryPreview { id: string; expires_at: string }
export interface RecoveryReceipt extends RecoveryPreview { reset_url: string }
const label = 'passordlenke'
export const recoveryTokenPattern = /^[A-Za-z0-9_-]{43}$/

export function decodeRecoveryPreview(value: unknown, expectedId?: string): RecoveryPreview {
  const data = decodeObject(value, 'recovery', label)
  const id = decodeUuid(data.id, 'recovery.id', label)
  if (expectedId && id !== expectedId) invalidData(label, 'recovery.id')
  const expires_at = decodeTimestamp(data.expires_at, 'recovery.expires_at', label)
  if (!Number.isFinite(Date.parse(expires_at))) invalidData(label, 'recovery.expires_at')
  return { id, expires_at }
}

export function decodeRecoveryReceipt(value: unknown): RecoveryReceipt {
  const data = decodeObject(value, 'recovery', label)
  const preview = decodeRecoveryPreview(data)
  const raw = decodeString(data.reset_url, 'recovery.reset_url', label)
  let url: URL
  try { url = new URL(raw) } catch { return invalidData(label, 'recovery.reset_url') }
  const loopback = ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)
  const token = /^#token=([A-Za-z0-9_-]{43})$/.exec(url.hash)?.[1]
  if ((url.protocol !== 'https:' && !(url.protocol === 'http:' && loopback))
    || url.username || url.password || url.search || url.pathname !== `/reset-password/${preview.id}`
    || !token || raw !== url.href) invalidData(label, 'recovery.reset_url')
  return { ...preview, reset_url: raw }
}

function adminPath(tournamentId: string, playerId: string): string {
  return `/api/tournaments/${encodeURIComponent(tournamentId)}/players/${encodeURIComponent(playerId)}/password-recovery`
}
function publicPath(id: string): string { return `/api/auth/password-recovery/${encodeURIComponent(id)}` }
function post(body: unknown, csrf?: string): RequestInit {
  return { ...jsonRequest('POST', body, csrf), cache: 'no-store', referrerPolicy: 'no-referrer' }
}
export const passwordRecoveryApi = {
  issue: (tournament: string, player: string, password: string, csrf: string): Promise<RecoveryReceipt> =>
    requestDecoded(adminPath(tournament, player), decodeRecoveryReceipt, post({ current_password: password }, csrf)),
  revoke: (tournament: string, player: string, password: string, csrf: string): Promise<void> =>
    requestNoContent(`${adminPath(tournament, player)}/revoke`, post({ current_password: password }, csrf)),
  preview: (id: string, token: string): Promise<RecoveryPreview> =>
    requestDecoded(`${publicPath(id)}/preview`, (value) => decodeRecoveryPreview(value, id), post({ token })),
  redeem: (id: string, token: string, newPassword: string, confirmPassword: string): Promise<void> =>
    requestNoContent(`${publicPath(id)}/redeem`, post({ token, new_password: newPassword, confirm_password: confirmPassword })),
}
