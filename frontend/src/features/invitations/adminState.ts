import type { InvitationMetadata } from '../../api/invitations'

export type InvitationStatus = 'active' | 'expired' | 'revoked' | 'exhausted'
export interface RevealedInvitation { invitationId: string; token: string }

export function invitationStatus(invitation: InvitationMetadata, now: number): InvitationStatus {
  if (invitation.revoked_at !== null) return 'revoked'
  if (Date.parse(invitation.expires_at) <= now) return 'expired'
  if (invitation.max_uses !== null && invitation.redemption_count >= invitation.max_uses) return 'exhausted'
  return 'active'
}

export function toExpiryTimestamp(localValue: string, now: number): string | null {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/.test(localValue)) return null
  const date = new Date(localValue)
  if (!Number.isFinite(date.getTime()) || date.getTime() <= now) return null
  return date.toISOString()
}

export function parseMaximumUses(value: string): number | null | 'invalid' {
  if (!value.trim()) return null
  const parsed = Number(value)
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : 'invalid'
}

export function revealedInvitationKey(revealed: RevealedInvitation): string {
  return revealed.invitationId
}

export function revealedAfterRevoke(
  current: RevealedInvitation | null,
  revokedInvitationId: string,
): RevealedInvitation | null {
  return current?.invitationId === revokedInvitationId ? null : current
}
