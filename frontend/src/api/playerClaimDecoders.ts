import { decodeAuthSession, type AuthSession } from './auth'
import { decodeArray, decodeBoolean, decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from './decoder'

export interface PlayerAccount { player_id: string; has_account: boolean; claim_id: string | null; claim_expires_at: string | null; claim_revoked_at: string | null; claimed_at: string | null }
export interface ClaimReceipt { player_id: string; claim_id: string; expires_at: string; token: string }
export interface ClaimPreview { tournament: { id: string; name: string }; player: { id: string; display_name: string }; expires_at: string }
export interface ClaimRegistration { tournament_id: string; player_id: string; session: AuthSession }
export const claimTokenPattern = /^[A-Za-z0-9_-]{43}$/
const label = 'spillerkonto'
function timestamp(value: unknown, path: string): string {
  const iso = decodeTimestamp(value, path, label)
  if (!Number.isFinite(Date.parse(iso))) invalidData(label, path)
  return iso
}
export function decodePlayerAccounts(value: unknown): PlayerAccount[] {
  const result = decodeArray(value, 'accounts', (item, path) => {
    const data = decodeObject(item, path, label)
    return { player_id: decodeUuid(data.player_id, `${path}.player_id`, label), has_account: decodeBoolean(data.has_account, `${path}.has_account`, label),
      claim_id: data.claim_id === null ? null : decodeUuid(data.claim_id, `${path}.claim_id`, label),
      claim_expires_at: data.claim_expires_at === null ? null : timestamp(data.claim_expires_at, `${path}.claim_expires_at`),
      claim_revoked_at: data.claim_revoked_at === null ? null : timestamp(data.claim_revoked_at, `${path}.claim_revoked_at`),
      claimed_at: data.claimed_at === null ? null : timestamp(data.claimed_at, `${path}.claimed_at`) }
  }, label)
  if (new Set(result.map(item => item.player_id)).size !== result.length) invalidData(label, 'accounts.duplicate')
  return result
}
export function decodeClaimReceipt(value: unknown, expectedPlayer?: string): ClaimReceipt {
  const data = decodeObject(value, 'claim', label)
  const player_id = decodeUuid(data.player_id, 'claim.player_id', label)
  if (expectedPlayer && player_id !== expectedPlayer) invalidData(label, 'claim.player_id')
  const token = decodeString(data.token, 'claim.token', label)
  if (!claimTokenPattern.test(token)) invalidData(label, 'claim.token')
  return { player_id, claim_id: decodeUuid(data.claim_id, 'claim.claim_id', label), expires_at: timestamp(data.expires_at, 'claim.expires_at'), token }
}
export function decodeClaimPreview(value: unknown): ClaimPreview {
  const data = decodeObject(value, 'preview', label)
  const tournament = decodeObject(data.tournament, 'preview.tournament', label)
  const player = decodeObject(data.player, 'preview.player', label)
  return { tournament: { id: decodeUuid(tournament.id, 'tournament.id', label), name: decodeString(tournament.name, 'tournament.name', label) },
    player: { id: decodeUuid(player.id, 'player.id', label), display_name: decodeString(player.display_name, 'player.display_name', label) }, expires_at: timestamp(data.expires_at, 'preview.expires_at') }
}
export function decodeClaimRegistration(value: unknown, expected: ClaimPreview): ClaimRegistration {
  const data = decodeObject(value, 'registration', label)
  const tournament_id = decodeUuid(data.tournament_id, 'registration.tournament_id', label)
  const player_id = decodeUuid(data.player_id, 'registration.player_id', label)
  const session = decodeAuthSession(data.session)
  if (tournament_id !== expected.tournament.id || player_id !== expected.player.id || session.player_id !== player_id) invalidData(label, 'registration.identity')
  return { tournament_id, player_id, session }
}
