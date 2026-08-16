import { afterEach, describe, expect, it, vi } from 'vitest'
import { decodeAuthSession } from './auth'
import { api } from './client'

const session = {
  user_id: '00000000-0000-0000-0000-000000000001',
  username: 'golf_admin',
  display_name: 'Golf Admin',
  role: 'admin',
  player_id: null,
  expires_at: '2026-08-07T12:00:00Z',
  csrf_token: 'derived-token',
}

afterEach(() => vi.unstubAllGlobals())

describe('session decoder', () => {
  it('accepts the exact authenticated session contract', () => {
    expect(decodeAuthSession(session)).toEqual(session)
  })

  it('rejects unknown roles and malformed linked player ids', () => {
    expect(() => decodeAuthSession({ ...session, role: 'owner' })).toThrow('session.role')
    expect(() => decodeAuthSession({ ...session, player_id: 'player-one' })).toThrow('session.player_id')
  })

  it('posts username and password without an account email', async () => {
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(session), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(api.login('golf_admin', 'hemmelig passord')).resolves.toEqual(session)

    expect(fetchMock).toHaveBeenCalledWith('/api/auth/login', {
      credentials: 'include',
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ username: 'golf_admin', password: 'hemmelig passord' }),
    })
  })
})
