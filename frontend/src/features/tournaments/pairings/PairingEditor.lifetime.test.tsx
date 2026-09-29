// @vitest-environment jsdom
import { act, cleanup, fireEvent, screen, waitFor } from '@testing-library/react'
import { QueryClient } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { PairingEditor } from './PairingEditor'
import { pairingApi, pairingKeys, type RoundPairings } from '../../../api/pairings'
import { ApiHttpError } from '../../../api/http'
import { authKeys } from '../../../api/auth'
import { deferred, mountEditor, pairings, round, savedPairings, session, settled, tournament, transition } from '../__tests__/managementLifetimeFixtures'
const clients: QueryClient[] = []
afterEach(() => { cleanup(); clients.splice(0).forEach(client => client.clear()); vi.restoreAllMocks() })
function mount() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity }, mutations: { retry: false } } }); clients.push(client)
  client.setQueryData(pairingKeys.detail(session.user_id, round.id), pairings)
  const get = vi.spyOn(pairingApi, 'get').mockImplementation(async (roundId, tournamentId) => ({ ...pairings, round_id: roundId, tournament_id: tournamentId }))
  const editor = mountEditor(client, target => <PairingEditor expanded tournamentId={target === 'tournament' ? '00000000-0000-0000-0000-000000000099' : tournament.id}
    round={target === 'round' ? { ...round, id: '00000000-0000-0000-0000-000000000098' } : round} />)
  return { client, get, ...editor }
}
async function edit(name = 'Lagret flight') {
  fireEvent.change(await screen.findByLabelText('Navn'), { target: { value: name } })
}
async function submit() {
  await edit(); fireEvent.click(screen.getByRole('button', { name: 'Lagre hele oppsettet' }))
}
for (const kind of ['logout', 'account', 'csrf', 'unmount', 'round', 'tournament']) for (const outcome of ['success', 'stale', 'access', 'failure']) {
  it(`ignores late pairing ${outcome} after ${kind}`, async () => {
    const pending = deferred<RoundPairings>(), save = vi.spyOn(pairingApi, 'replace').mockReturnValue(pending.promise)
    const { client, change, get } = mount(); await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
    await transition(change, kind)
    const before = client.getQueriesData({ queryKey: ['private-workspace'] }), writes = vi.spyOn(client, 'setQueryData'), refresh = vi.spyOn(client, 'invalidateQueries'), reads = get.mock.calls.length
    await act(async () => {
      if (outcome === 'success') pending.resolve(savedPairings)
      else pending.reject(outcome === 'failure' ? new Error('synthetic error') : new ApiHttpError(outcome === 'access' ? 403 : 409, outcome === 'stale' ? 'round_pairings_stale' : 'forbidden', 'synthetic rejection'))
    })
    await settled(client)
    expect(writes).not.toHaveBeenCalled(); expect(refresh).not.toHaveBeenCalled(); expect(get).toHaveBeenCalledTimes(reads)
    expect(client.getQueriesData({ queryKey: ['private-workspace'] })).toEqual(before)
    expect(screen.queryByRole('alert')).toBeNull()
  })
}
it('rejects old pairing dispatch and response after canonical renewal before React replacement', async () => {
  const pending = deferred<RoundPairings>(), save = vi.spyOn(pairingApi, 'replace').mockReturnValue(pending.promise)
  const { client } = mount(); await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
  act(() => client.setQueryData(authKeys.session, { ...session, csrf_token: 'renewed' }))
  await act(async () => pending.resolve(savedPairings)); await settled(client)
  expect(client.getQueryData(pairingKeys.detail(session.user_id, round.id))).toEqual(pairings)
  fireEvent.click(screen.getByRole('button', { name: 'Lagre hele oppsettet' }))
  expect(save).toHaveBeenCalledOnce()
})
it('preserves the replacement draft and pending save when an old response completes', async () => {
  const old = deferred<RoundPairings>(), fresh = deferred<RoundPairings>()
  const save = vi.spyOn(pairingApi, 'replace').mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise)
  const { client, change, get } = mount(); await submit(); await waitFor(() => expect(save).toHaveBeenCalledOnce())
  await transition(change, 'csrf')
  await edit('Nytt utkast')
  fireEvent.click(screen.getByRole('button', { name: 'Lagre hele oppsettet' }))
  await waitFor(() => expect(save).toHaveBeenCalledTimes(2))
  await act(async () => old.resolve(savedPairings))
  expect(screen.getByLabelText('Navn')).toHaveProperty('value', 'Nytt utkast')
  expect(screen.getByRole('button', { name: 'Lagrer…' })).toHaveProperty('disabled', true)
  const freshResult = { ...savedPairings, flights: savedPairings.flights.map(f => ({ ...f, name: 'Nytt utkast' })) }
  get.mockResolvedValue(freshResult)
  await act(async () => fresh.resolve(freshResult)); await settled(client)
  expect(screen.getByLabelText('Navn')).toHaveProperty('value', 'Nytt utkast')
  expect(screen.getByText('Synkronisert med serveren')).toBeTruthy()
  expect(save.mock.calls[1]?.[3]).toBe('renewed')
})
it('preserves current dirty input on conflict and explicitly discards to fresh server state', async () => {
  vi.spyOn(pairingApi, 'replace').mockRejectedValue(new ApiHttpError(409, 'round_pairings_stale', 'stale'))
  const { get } = mount(); get.mockResolvedValue({ ...savedPairings, flights: pairings.flights.map(f => ({ ...f, name: 'Ny serverflight' })) })
  await submit()
  await screen.findByText('En nyere versjon finnes')
  expect(screen.getByLabelText('Navn')).toHaveProperty('value', 'Lagret flight')
  fireEvent.click(screen.getByRole('button', { name: 'Forkast og last på nytt' }))
  await waitFor(() => expect(screen.getByLabelText('Navn')).toHaveProperty('value', 'Ny serverflight'))
  expect(screen.queryByRole('alert')).toBeNull()
})
it('keeps a new session draft when a departed discard refresh completes', async () => {
  vi.spyOn(pairingApi, 'replace').mockRejectedValue(new ApiHttpError(409, 'round_pairings_stale', 'stale'))
  const { change, get } = mount(); await submit(); await screen.findByText('En nyere versjon finnes')
  const held = deferred<RoundPairings>(); get.mockReturnValueOnce(held.promise)
  const reads = get.mock.calls.length
  fireEvent.click(screen.getByRole('button', { name: 'Forkast og last på nytt' }))
  await waitFor(() => expect(get).toHaveBeenCalledTimes(reads + 1))
  await transition(change, 'csrf'); await edit('Utkast i ny økt')
  await act(async () => held.resolve(savedPairings))
  expect(screen.getByLabelText('Navn')).toHaveProperty('value', 'Utkast i ny økt')
})
it('retains failed current-session input and permits explicit retry', async () => {
  const save = vi.spyOn(pairingApi, 'replace').mockRejectedValueOnce(new Error('synthetic network error')).mockResolvedValueOnce(savedPairings)
  const { client, get } = mount(); await submit(); await screen.findByRole('alert'); await settled(client)
  expect(screen.getByLabelText('Navn')).toHaveProperty('value', 'Lagret flight')
  get.mockResolvedValue(savedPairings)
  fireEvent.click(screen.getByRole('button', { name: 'Lagre hele oppsettet' })); await settled(client)
  await waitFor(() => expect(save).toHaveBeenCalledTimes(2))
  expect(client.getQueryData(pairingKeys.detail(session.user_id, round.id))).toEqual(savedPairings)
})
