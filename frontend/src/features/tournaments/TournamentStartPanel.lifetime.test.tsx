// @vitest-environment jsdom
import { act, cleanup, fireEvent, screen, waitFor } from '@testing-library/react'
import { QueryClient } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { TournamentStartPanel } from './TournamentStartPanel'
import { tournamentApi, tournamentKeys } from '../../api/tournaments'
import { ApiHttpError } from '../../api/http'
import { authKeys } from '../../api/auth'
import type { Tournament } from '../../api/types'
import { deferred, mountEditor, roster, round, session, settled, tournament, transition } from './__tests__/managementLifetimeFixtures'
const clients: QueryClient[] = []
afterEach(() => { cleanup(); clients.splice(0).forEach(client => client.clear()); vi.restoreAllMocks() })
const started = { ...tournament, status: 'active' as const }
function mount() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } }); clients.push(client)
  client.setQueryData(tournamentKeys.detail(session.user_id, tournament.id), tournament)
  const editor = mountEditor(client, target => {
    const id = target === 'initial' ? tournament.id : '00000000-0000-0000-0000-000000000099'
    return <TournamentStartPanel tournament={{ ...tournament, id }} rounds={{ data: [{ ...round, tournament_id: id }], pending: false, error: null, retry: vi.fn() }}
      roster={{ data: roster, pending: false, error: null, retry: vi.fn() }} />
  })
  return { client, ...editor }
}
const submit = () => fireEvent.click(screen.getByRole('button', { name: /^(Start turneringen|Prøv å starte igjen)$/ }))
for (const kind of ['logout', 'account', 'csrf', 'unmount', 'target']) for (const outcome of ['success', 'stale', 'not-ready', 'failure']) {
  it(`ignores late start ${outcome} after ${kind}`, async () => {
    const pending = deferred<Tournament>(), save = vi.spyOn(tournamentApi, 'start').mockReturnValue(pending.promise)
    const { client, change } = mount(); submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
    await transition(change, kind)
    const before = client.getQueriesData({ queryKey: ['private-workspace'] }), writes = vi.spyOn(client, 'setQueryData'), refresh = vi.spyOn(client, 'invalidateQueries')
    await act(async () => {
      if (outcome === 'success') pending.resolve(started)
      else pending.reject(outcome === 'failure' ? new Error('synthetic error') : new ApiHttpError(409, outcome === 'stale' ? 'tournament_start_stale' : 'tournament_start_not_ready', 'synthetic conflict'))
    })
    await settled(client)
    expect(writes).not.toHaveBeenCalled(); expect(refresh).not.toHaveBeenCalled()
    expect(client.getQueriesData({ queryKey: ['private-workspace'] })).toEqual(before)
    expect(screen.queryByText('Turneringen er startet. Alle rundene er fortsatt i kladd.')).toBeNull()
    expect(screen.queryByRole('alert')).toBeNull()
  })
}
it('rejects an old start after canonical session publication before React replacement', async () => {
  const pending = deferred<Tournament>(), save = vi.spyOn(tournamentApi, 'start').mockReturnValue(pending.promise)
  const { client } = mount(); submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
  act(() => client.setQueryData(authKeys.session, { ...session, csrf_token: 'renewed' }))
  await act(async () => pending.resolve(started)); await settled(client)
  expect(client.getQueryData(tournamentKeys.detail(session.user_id, tournament.id))).toEqual(tournament)
  submit(); expect(save).toHaveBeenCalledOnce()
})
it('keeps a new session start pending through an old completion and blocks duplicates', async () => {
  const old = deferred<Tournament>(), fresh = deferred<Tournament>()
  const save = vi.spyOn(tournamentApi, 'start').mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise)
  const { client, change } = mount(); submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
  await transition(change, 'csrf'); submit(); await waitFor(() => expect(save).toHaveBeenCalledTimes(2))
  await act(async () => old.resolve(started))
  expect(screen.getByRole('button', { name: 'Starter …' })).toHaveProperty('disabled', true)
  await act(async () => fresh.resolve(started)); await settled(client)
  expect(screen.getByText('Turneringen er startet. Alle rundene er fortsatt i kladd.')).toBeTruthy()
  expect(save.mock.calls[1]?.[2]).toBe('renewed')
  expect(client.getQueryData(tournamentKeys.detail(session.user_id, tournament.id))).toEqual(started)
})
it('refreshes current-session stale start and permits explicit retry', async () => {
  const save = vi.spyOn(tournamentApi, 'start').mockRejectedValueOnce(new ApiHttpError(409, 'tournament_start_stale', 'stale')).mockResolvedValueOnce(started)
  const { client } = mount(), refresh = vi.spyOn(client, 'invalidateQueries')
  submit(); await screen.findByRole('alert'); await settled(client)
  expect(refresh).toHaveBeenCalledWith({ queryKey: tournamentKeys.detail(session.user_id, tournament.id) })
  submit(); await settled(client)
  expect(save).toHaveBeenCalledTimes(2)
  expect(client.getQueryData(tournamentKeys.detail(session.user_id, tournament.id))).toEqual(started)
})
