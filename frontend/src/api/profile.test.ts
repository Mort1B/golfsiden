import { expect, it } from 'vitest'
import { decodeProfile, profileKeys } from './profile'
const user = '00000000-0000-0000-0000-000000000001'
const value = { user_id: user, username: 'test-user', display_name: 'Test', version: 0, player_id: null, handicap: null, player_active: null, player_updated_at: null }
it('decodes unlinked profiles and roots cache by owner', () => {
  expect(decodeProfile(value, user)).toEqual(value)
  expect(profileKeys.me(user)).toEqual(['private-workspace', user, 'profile'])
})
it('rejects mismatched owners and inconsistent player/version/handicap data', () => {
  expect(() => decodeProfile(value, '00000000-0000-0000-0000-000000000002')).toThrow()
  for (const invalid of [{ ...value, version: -1 }, { ...value, handicap: 14.4 }, { ...value, player_id: user },
    { ...value, player_id: user, player_active: true, player_updated_at: '2026-09-06T12:00:00Z', handicap: 2.22 }]) {
    expect(() => decodeProfile(invalid, user)).toThrow()
  }
})
