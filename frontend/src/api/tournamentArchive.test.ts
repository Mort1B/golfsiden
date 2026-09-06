import { afterEach, expect, it, vi } from 'vitest'
import { tournamentApi } from './tournaments'
import { tournament } from '../features/tournaments/lifecycle/__tests__/fixtures'

afterEach(() => vi.unstubAllGlobals())
it('posts only the expected version with CSRF and validates identity and terminal status', async () => {
  const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify({ ...tournament, status: 'archived' })))
  vi.stubGlobal('fetch', fetch)
  await expect(tournamentApi.archive(tournament.id, tournament.updated_at, 'csrf')).resolves.toMatchObject({ status: 'archived' })
  expect(fetch).toHaveBeenCalledWith(`/api/tournaments/${tournament.id}/archive`, expect.objectContaining({
    method: 'POST', credentials: 'include', headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf' },
    body: JSON.stringify({ expected_tournament_updated_at: tournament.updated_at }),
  }))
  fetch.mockResolvedValue(new Response(JSON.stringify(tournament)))
  await expect(tournamentApi.archive(tournament.id, tournament.updated_at, 'csrf')).rejects.toThrow(/arkiveringsstatus/)
  fetch.mockResolvedValue(new Response(JSON.stringify({ ...tournament, id: '00000000-0000-0000-0000-000000000099', status: 'archived' })))
  await expect(tournamentApi.archive(tournament.id, tournament.updated_at, 'csrf')).rejects.toThrow()
})
