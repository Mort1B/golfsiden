// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, useLocation } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { TournamentOnboardingPage } from './TournamentOnboardingPage'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { session, tournament } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { creationApi, type TournamentCreationReceipt } from '../api/tournamentCreation'
import { ApiHttpError } from '../api/http'
import { tournamentKeys } from '../api/tournaments'
import { clearPrivateWorkspace, privateWorkspaceKeys } from '../api/privateWorkspace'
let client: QueryClient
let auth: AuthContextValue
function Location() { return <output aria-label="Location">{useLocation().pathname}</output> }
function tree() { return <QueryClientProvider client={client}><AuthContext value={auth}><MemoryRouter initialEntries={['/create']}><TournamentOnboardingPage /><Location /></MemoryRouter></AuthContext></QueryClientProvider> }
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  auth = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
  vi.spyOn(creationApi, 'create').mockImplementation(async (input) => ({ request_id: input.request_id, tournament_id: tournament.id, created: true }))
  vi.spyOn(window, 'scrollTo').mockImplementation(() => {})
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })
async function review() {
  fireEvent.change(screen.getByLabelText('Turneringsnavn'), { target: { value: 'Testtur' } })
  fireEvent.click(screen.getByRole('button', { name: 'Neste' }))
  fireEvent.click(screen.getByRole('button', { name: 'Neste' }))
  await screen.findByRole('heading', { name: 'Kontroller opplysningene' })
}
it('signed-in wizard skips account entry, preserves previous memberships and creates through CSRF API', async () => {
  const previous = [{ tournament, role: 'player', player_id: session.player_id }]
  client.setQueryData(tournamentKeys.mine(session.user_id), previous)
  render(tree()); await review()
  expect(screen.queryByLabelText(/Passord/)).toBeNull()
  expect(screen.getByText('Steg 3 av 3')).toBeTruthy()
  expect(screen.getByText(/Du blir administrator med den eksisterende kontoen din/)).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Opprett turnering' }))
  await waitFor(() => expect(screen.getByLabelText('Location').textContent).toBe('/manage/tournaments/' + tournament.id))
  expect(creationApi.create).toHaveBeenCalledTimes(1)
  const call = vi.mocked(creationApi.create).mock.calls[0]
  if (!call) throw new Error('No request')
  expect(call[0]).not.toHaveProperty('creator')
  expect(call[0].tournament.name).toBe('Testtur')
  expect(call[1]).toBe(session.csrf_token)
  expect(auth.establishSession).not.toHaveBeenCalled()
  expect(client.getQueryData(tournamentKeys.mine(session.user_id))).toEqual(previous)
})
it('retains frozen retry key through lost success, throttling and authoritative retry', async () => {
  vi.mocked(creationApi.create).mockRejectedValueOnce(new Error('lost response')).mockRejectedValueOnce(new ApiHttpError(429, 'rate_limited', 'wait'))
  render(tree()); await review()
  for (const expected of [/Vi fikk ikke bekreftet/, /For mange opprettingsforsøk/]) {
    fireEvent.click(screen.getByRole('button', { name: 'Opprett turnering' }))
    await screen.findByText(expected)
    expect(screen.getByRole('button', { name: 'Tilbake' }).hasAttribute('disabled')).toBe(true)
  }
  fireEvent.click(screen.getByRole('button', { name: 'Opprett turnering' }))
  await waitFor(() => expect(screen.getByLabelText('Location').textContent).toContain('/manage/'))
  const calls = vi.mocked(creationApi.create).mock.calls
  expect(calls).toHaveLength(3)
  expect(calls[0]?.[0]).toEqual(calls[1]?.[0])
  expect(calls[0]?.[0]).toEqual(calls[2]?.[0])
})
it.each([null, { ...session, user_id: '00000000-0000-0000-0000-000000000099' }])('ignores late success after account transition to %j', async (next) => {
  let resolve: ((value: TournamentCreationReceipt) => void) | undefined
  vi.mocked(creationApi.create).mockReturnValue(new Promise((r) => { resolve = r }))
  const view = render(tree()); await review()
  fireEvent.click(screen.getByRole('button', { name: 'Opprett turnering' }))
  await screen.findByRole('button', { name: 'Oppretter …' })
  const form = screen.getByRole('button', { name: 'Oppretter …' }).closest('form')
  if (!form) throw new Error('Missing creation form')
  fireEvent.submit(form)
  expect(creationApi.create).toHaveBeenCalledTimes(1)
  const request = vi.mocked(creationApi.create).mock.calls[0]?.[0]
  if (!resolve || !request) throw new Error('Missing pending request')
  auth = { ...auth, session: next }; clearPrivateWorkspace(client); view.rerender(tree())
  const complete = resolve
  await act(async () => complete({ request_id: request.request_id, tournament_id: tournament.id, created: true }))
  expect(screen.getByLabelText('Location').textContent).toBe('/create')
  expect(client.getQueriesData({ queryKey: privateWorkspaceKeys.root })).toEqual([])
  expect(auth.establishSession).not.toHaveBeenCalled()
})
it('allows editing after an unambiguous validation rejection', async () => {
  vi.mocked(creationApi.create).mockRejectedValueOnce(new ApiHttpError(400, 'bad_request', 'invalid'))
  render(tree()); await review(); fireEvent.click(screen.getByRole('button', { name: 'Opprett turnering' }))
  await screen.findByText(/Opprettingen ble avvist/)
  expect(screen.getByRole('button', { name: 'Tilbake' }).hasAttribute('disabled')).toBe(false)
})
it('keeps anonymous account onboarding and exposes auth loading and retry states', async () => {
  auth = { ...auth, loading: true }; const view = render(tree())
  expect(screen.getByText('Laster …')).toBeTruthy()
  auth = { ...auth, loading: false, error: new Error('auth offline') }; view.rerender(tree())
  expect(screen.getByRole('alert').textContent).toContain('auth offline')
  auth = { ...auth, error: null, session: null }; view.rerender(tree())
  fireEvent.change(screen.getByLabelText('Turneringsnavn'), { target: { value: 'Første tur' } })
  fireEvent.click(screen.getByRole('button', { name: 'Neste' }))
  fireEvent.click(screen.getByRole('button', { name: 'Neste' }))
  expect(screen.getByRole('heading', { name: 'Din spillerkonto' })).toBeTruthy()
  expect(screen.getByLabelText(/Passord/)).toBeTruthy()
  expect(creationApi.create).not.toHaveBeenCalled()
})
