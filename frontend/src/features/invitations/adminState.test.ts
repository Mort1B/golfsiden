import { describe, expect, it } from 'vitest'
import type { InvitationMetadata } from '../../api/invitations'
import {
  invitationStatus,
  parseMaximumUses,
  revealedAfterRevoke,
  revealedInvitationKey,
  toExpiryTimestamp,
} from './adminState'

const base: InvitationMetadata = {
  id: '00000000-0000-0000-0000-000000000001',
  tournament_id: '00000000-0000-0000-0000-000000000002',
  series_id: '00000000-0000-0000-0000-000000000003',
  predecessor_id: null,
  created_by_user_id: '00000000-0000-0000-0000-000000000004',
  created_at: '2026-08-16T10:00:00Z',
  expires_at: '2026-08-17T10:00:00Z',
  revoked_at: null,
  revoked_by_user_id: null,
  revocation_actor_known: false,
  max_uses: 3,
  redemption_count: 1,
}

describe('invitation admin state', () => {
  it('derives lifecycle states with revocation precedence', () => {
    expect(invitationStatus(base, Date.parse('2026-08-16T12:00:00Z'))).toBe('active')
    expect(invitationStatus({ ...base, redemption_count: 3 }, Date.parse('2026-08-16T12:00:00Z'))).toBe('exhausted')
    expect(invitationStatus(base, Date.parse('2026-08-18T12:00:00Z'))).toBe('expired')
    expect(invitationStatus({ ...base, revoked_at: '2026-08-16T11:00:00Z' }, Date.parse('2026-08-18T12:00:00Z'))).toBe('revoked')
  })

  it('validates nullable positive maximum uses', () => {
    expect(parseMaximumUses('')).toBeNull()
    expect(parseMaximumUses('12')).toBe(12)
    expect(parseMaximumUses('0')).toBe('invalid')
    expect(parseMaximumUses('1.5')).toBe('invalid')
  })

  it('accepts only a valid future local expiry', () => {
    const future = toExpiryTimestamp('2026-08-17T12:30', Date.parse('2026-08-16T12:00:00Z'))
    expect(future).toMatch(/^2026-08-17T/)
    expect(toExpiryTimestamp('2026-08-15T12:30', Date.parse('2026-08-16T12:00:00Z'))).toBeNull()
    expect(toExpiryTimestamp('not-a-date', Date.parse('2026-08-16T12:00:00Z'))).toBeNull()
  })

  it('remounts a changed one-time link and clears the revoked revealed secret', () => {
    const revealed = { invitationId: base.id, token: 'A'.repeat(43) }
    const successor = { invitationId: base.series_id, token: 'B'.repeat(43) }
    expect(revealedInvitationKey(revealed)).not.toBe(revealedInvitationKey(successor))
    expect(revealedAfterRevoke(revealed, base.id)).toBeNull()
    expect(revealedAfterRevoke(revealed, base.series_id)).toBe(revealed)
  })
})
