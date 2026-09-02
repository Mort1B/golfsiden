import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  decodeFinalRoundVisibility,
  finalRoundVisibilityApi,
  finalRoundVisibilityKeys,
} from './finalRoundVisibility'

const tournamentId = '00000000-0000-0000-0000-000000000001'
const visibility = {
  tournament_id: tournamentId,
  back_nine_hidden: true,
  visibility_updated_at: '2026-09-02T10:00:00Z',
}

afterEach(() => vi.unstubAllGlobals())

describe('final-round visibility API', () => {
  it('decodes the exact tournament resource and rejects target drift', () => {
    expect(decodeFinalRoundVisibility(visibility, tournamentId)).toEqual(visibility)
    expect(() => decodeFinalRoundVisibility({
      ...visibility,
      tournament_id: '00000000-0000-0000-0000-000000000099',
    }, tournamentId)).toThrow('identity')
    expect(() => decodeFinalRoundVisibility({ ...visibility, back_nine_hidden: 'yes' }, tournamentId))
      .toThrow('back_nine_hidden')
  })

  it('uses a user and tournament-owned cache key', () => {
    expect(finalRoundVisibilityKeys.detail('user-one', tournamentId)).toEqual([
      'private-workspace', 'user-one', 'tournaments', tournamentId, 'final-round-visibility',
    ])
    expect(finalRoundVisibilityKeys.detail('user-two', tournamentId))
      .not.toEqual(finalRoundVisibilityKeys.detail('user-one', tournamentId))
  })

  it('maps the desired state, optimistic timestamp, and CSRF token to PATCH', async () => {
    const released = {
      ...visibility,
      back_nine_hidden: false,
      visibility_updated_at: '2026-09-02T10:05:00Z',
    }
    const fetchMock = vi.fn(async () => new Response(JSON.stringify(released), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(finalRoundVisibilityApi.update(tournamentId, {
      back_nine_hidden: false,
      expected_visibility_updated_at: visibility.visibility_updated_at,
    }, 'csrf-token')).resolves.toEqual(released)

    expect(fetchMock).toHaveBeenCalledWith(
      `/api/tournaments/${tournamentId}/final-round-visibility`,
      {
        credentials: 'include',
        method: 'PATCH',
        headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf-token' },
        body: JSON.stringify({
          back_nine_hidden: false,
          expected_visibility_updated_at: visibility.visibility_updated_at,
        }),
      },
    )
  })
})
