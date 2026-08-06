import { describe, expect, it } from 'vitest'
import { decodeAuthSession } from './auth'

const session = {
  user_id: '00000000-0000-0000-0000-000000000001',
  display_name: 'Golf Admin',
  role: 'admin',
  player_id: null,
  expires_at: '2026-08-07T12:00:00Z',
  csrf_token: 'derived-token',
}

describe('session decoder', () => {
  it('accepts the exact authenticated session contract', () => {
    expect(decodeAuthSession(session)).toEqual(session)
  })

  it('rejects unknown roles and malformed linked player ids', () => {
    expect(() => decodeAuthSession({ ...session, role: 'owner' })).toThrow('session.role')
    expect(() => decodeAuthSession({ ...session, player_id: 'player-one' })).toThrow('session.player_id')
  })
})
