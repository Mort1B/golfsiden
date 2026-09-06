import { afterEach, expect, it, vi } from 'vitest'
import { tournamentApi } from './tournaments'
import { tournament } from '../features/tournaments/lifecycle/__tests__/fixtures'

afterEach(() => vi.unstubAllGlobals())
it('posts only the expected version with CSRF and validates identity and terminal status', async () => {
  const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify({ ...tournament, status: 'completed' })))
  vi.stubGlobal('fetch', fetch)
  await expect(tournamentApi.complete(tournament.id, tournament.updated_at, 'csrf')).resolves.toMatchObject({ status: 'completed' })
  expect(fetch).toHaveBeenCalledWith(`/api/tournaments/${tournament.id}/complete`, expect.objectContaining({
    method: 'POST', credentials: 'include', headers: { 'content-type': 'application/json', 'x-csrf-token': 'csrf' },
    body: JSON.stringify({ expected_tournament_updated_at: tournament.updated_at }),
  }))
  fetch.mockResolvedValue(new Response(JSON.stringify(tournament)))
  await expect(tournamentApi.complete(tournament.id, tournament.updated_at, 'csrf')).rejects.toThrow(/fullføringsstatus/)
  fetch.mockResolvedValue(new Response(JSON.stringify({ ...tournament, id: '00000000-0000-0000-0000-000000000099', status: 'completed' })))
  await expect(tournamentApi.complete(tournament.id, tournament.updated_at, 'csrf')).rejects.toThrow()
})
