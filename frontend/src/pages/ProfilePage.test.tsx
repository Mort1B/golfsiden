// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { MemoryRouter, useLocation } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys, type AuthSession } from '../api/auth'
import { api } from '../api/client'
import { ApiHttpError } from '../api/http'
import type { Profile } from '../api/profile'
import { profileApi } from '../api/profileRequests'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { session, tournament } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { ProfilePage } from './ProfilePage'

let client: QueryClient
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
const original: Profile = { user_id: session.user_id, username: session.username, display_name: 'Navn med langt etternavn', version: 2,
  player_id: session.player_id, handicap: 8.2, player_active: true, player_updated_at: '2026-09-06T12:00:00Z' }
let profile: Profile
function Location() { return <p data-testid="location">{useLocation().pathname}</p> }
function Workspace({ value }: { value: AuthContextValue }) {
  const { data } = useQuery<AuthSession | null>({ queryKey: authKeys.session, queryFn: api.session, enabled: false })
  return <AuthContext value={{ ...value, session: data ?? null }}><Location /><ProfilePage /></AuthContext>
}
function tree(value = auth) {
  return <QueryClientProvider client={client}><MemoryRouter initialEntries={['/profile']}><Workspace value={value} /></MemoryRouter></QueryClientProvider>
}
function expand(heading: string) {
  const details = screen.getByRole('heading', { name: heading }).closest('details')
  if (!details) throw new Error('Missing disclosure')
  details.open = true
}
function form(heading: string) {
  const node = screen.getByRole('heading', { name: heading }).closest('form')
  if (!node) throw new Error('Missing form')
  return node
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  client.setQueryData(authKeys.session, session)
  profile = { ...original }
  vi.spyOn(profileApi, 'get').mockImplementation(async () => profile)
  vi.spyOn(api, 'session').mockResolvedValue(session)
  vi.spyOn(api, 'myTournaments').mockResolvedValue([{ tournament: { ...tournament, status: 'archived', name: 'Tidligere turnering' }, role: 'player', player_id: session.player_id }])
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })

it('lists historical memberships and saves comma handicap without a typed reason and with authoritative refetch', async () => {
  const save = vi.spyOn(profileApi, 'details').mockImplementation(async (_user, input) => {
    profile = { ...profile, display_name: input.display_name, handicap: input.handicap, version: 3 }; return profile
  })
  render(tree())
  await screen.findByRole('heading', { name: 'Navn og handicap' })
  expect(screen.getByRole('link', { name: /Tidligere turnering/ }).getAttribute('href')).toBe(`/tournaments/${tournament.id}`)
  fireEvent.change(screen.getByLabelText('Handicap'), { target: { value: '14,4' } })
  expect(screen.queryByLabelText('Begrunnelse for handicapendring')).toBeNull()
  fireEvent.submit(form('Navn og handicap'))
  await screen.findByText('Endringen er lagret og profilen er oppdatert.')
  expect(save).toHaveBeenCalledWith(session.user_id, { version: 2, player_updated_at: original.player_updated_at, display_name: original.display_name, handicap: 14.4 }, session.csrf_token)
  expect(profileApi.get).toHaveBeenCalledTimes(2)
  expect(screen.getByLabelText('Handicap')).toHaveProperty('value', '14,4')
})

it('shows duplicate username, clears passwords, and reloads stale forms deliberately', async () => {
  vi.spyOn(profileApi, 'username').mockRejectedValue(new ApiHttpError(409, 'username_unavailable', 'duplicate'))
  render(tree()); await screen.findByRole('heading', { name: 'Endre brukernavn' })
  expand('Endre brukernavn')
  const fields = within(form('Endre brukernavn'))
  fireEvent.change(fields.getByLabelText('Nåværende passord'), { target: { value: 'current-password' } })
  fireEvent.submit(form('Endre brukernavn'))
  await screen.findByText('Brukernavnet er opptatt. Velg et annet.')
  expect(fields.getByLabelText('Nåværende passord')).toHaveProperty('value', '')
  vi.spyOn(profileApi, 'details').mockRejectedValue(new ApiHttpError(409, 'profile_stale', 'stale'))
  fireEvent.submit(form('Navn og handicap'))
  await screen.findByText(/Profilen er endret siden/)
  profile = { ...profile, version: 4, display_name: 'Navn fra en annen enhet' }
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater profil og turneringer' }))
  await waitFor(() => expect(screen.getByLabelText('Navn')).toHaveProperty('value', profile.display_name))
})

it('shows unlinked/empty and inactive states without removing the name editor', async () => {
  profile = { ...profile, player_id: null, handicap: null, player_active: null, player_updated_at: null }
  vi.mocked(api.myTournaments).mockResolvedValue([])
  render(tree()); await screen.findByText(/Du har ingen spillerprofil ennå/)
  expect(screen.getByText('Du er ikke med i noen turneringer ennå.')).toBeTruthy()
  profile = { ...original, player_active: false, version: 3 }
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater profil og turneringer' }))
  await screen.findByText(/Spillerprofilen er deaktivert/)
  expect(screen.getByLabelText('Handicap')).toHaveProperty('disabled', true)
  expect(screen.getByLabelText('Navn')).toHaveProperty('disabled', false)
})

it('hides stale profile data on refresh failure, offers retry, and restores forms', async () => {
  render(tree()); await screen.findByRole('heading', { name: 'Navn og handicap' })
  vi.mocked(profileApi.get).mockRejectedValue(new Error('offline'))
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater profil og turneringer' }))
  await screen.findByRole('alert')
  expect(screen.queryByRole('heading', { name: 'Navn og handicap' })).toBeNull()
  vi.mocked(profileApi.get).mockResolvedValue(profile)
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
  await screen.findByRole('heading', { name: 'Navn og handicap' })
})

function passwordSubmit() {
  expand('Endre passord')
  const fields = within(form('Endre passord'))
  fireEvent.change(fields.getByLabelText('Nytt passord'), { target: { value: 'my-new-password' } })
  fireEvent.change(fields.getByLabelText('Gjenta nytt passord'), { target: { value: 'my-new-password' } })
  fireEvent.change(fields.getByLabelText('Nåværende passord'), { target: { value: 'my-old-password' } })
  fireEvent.submit(form('Endre passord'))
}
it('clears authentication and private caches after password success without calling logout', async () => {
  vi.spyOn(profileApi, 'password').mockResolvedValue()
  render(tree()); await screen.findByRole('heading', { name: 'Endre passord' })
  passwordSubmit()
  await waitFor(() => expect(client.getQueryData(authKeys.session)).toBeNull())
  await waitFor(() => expect(screen.queryByRole('heading', { name: 'Endre passord' })).toBeNull())
  for (const query of client.getQueryCache().findAll({ queryKey: ['private-workspace'] })) expect(query.state.data).toBeUndefined()
  expect(auth.signOut).not.toHaveBeenCalled()
  expect(screen.getByTestId('location').textContent).toBe('/login')
})

it('prevents double submission and reconciles password success even after page departure', async () => {
  let finish: () => void = () => { throw new Error('Not started') }
  const save = vi.spyOn(profileApi, 'password').mockImplementation(() => new Promise<void>((resolve) => { finish = resolve }))
  const view = render(tree()); await screen.findByRole('heading', { name: 'Endre passord' })
  passwordSubmit(); fireEvent.submit(form('Endre passord'))
  await waitFor(() => expect(save).toHaveBeenCalledTimes(1))
  expect(screen.getByRole('button', { name: 'Lagre navn og handicap' }).closest('fieldset')).toHaveProperty('disabled', true)
  view.unmount()
  await act(async () => { finish() })
  expect(client.getQueryData(authKeys.session)).toBeNull()
  expect(client.getQueryCache().findAll({ queryKey: ['private-workspace'] })).toHaveLength(0)
})

it('does not let an old password response clear a fresh session for the same account', async () => {
  let finish: () => void = () => { throw new Error('Not started') }
  vi.spyOn(profileApi, 'password').mockImplementation(() => new Promise<void>((resolve) => { finish = resolve }))
  const view = render(tree()); await screen.findByRole('heading', { name: 'Endre passord' }); passwordSubmit()
  await waitFor(() => expect(profileApi.password).toHaveBeenCalledTimes(1)); view.unmount()
  const fresh = { ...session, csrf_token: 'new-login-token' }
  client.setQueryData(authKeys.session, fresh)
  await act(async () => { finish() })
  expect(client.getQueryData(authKeys.session)).toEqual(fresh)
})

it('late name response invalidates the original session workspace after departure', async () => {
  let finish: (value: Profile) => void = () => { throw new Error('Not started') }
  vi.spyOn(profileApi, 'details').mockImplementation(() => new Promise<Profile>((resolve) => { finish = resolve }))
  const invalidate = vi.spyOn(client, 'invalidateQueries')
  const view = render(tree()); await screen.findByRole('heading', { name: 'Navn og handicap' })
  fireEvent.submit(form('Navn og handicap'))
  await waitFor(() => expect(profileApi.details).toHaveBeenCalledTimes(1)); view.unmount()
  await act(async () => { finish(profile) })
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ['private-workspace', session.user_id] }, { throwOnError: true })
})

it('rejects an in-flight profile read after its session is replaced', async () => {
  let finish: (value: Profile) => void = () => { throw new Error('Not started') }
  vi.mocked(profileApi.get).mockImplementation(() => new Promise<Profile>((resolve) => { finish = resolve }))
  render(tree())
  await waitFor(() => expect(profileApi.get).toHaveBeenCalledTimes(1))
  const fresh = { ...session, csrf_token: 'replacement-session' }
  await act(async () => { client.setQueryData(authKeys.session, fresh); finish(profile) })
  // A replacement session may already be loading its own authoritative read;
  // the previous request must never supply that page or cache with data.
  expect(screen.queryByRole('heading', { name: 'Navn og handicap' })).toBeNull()
  for (const query of client.getQueryCache().findAll({ queryKey: ['private-workspace', session.user_id, 'profile'] })) expect(query.state.data).toBeUndefined()
})

it('puts tournaments first and keeps credential forms collapsed initially', async () => {
  render(tree()); await screen.findByRole('heading', { name: 'Navn og handicap' })
  const headings = screen.getAllByRole('heading').map(node => node.textContent)
  expect(headings.indexOf('Mine turneringer')).toBeLessThan(headings.indexOf('Navn og handicap'))
  for (const title of ['Endre brukernavn', 'Endre passord']) {
    expect(screen.getByRole('heading', { name: title }).closest('details')).toHaveProperty('open', false)
  }
})

it('keeps pending and failed mutation feedback visible after collapsing its section', async () => {
  let fail: (error: Error) => void = () => { throw new Error('Not started') }
  vi.spyOn(profileApi, 'username').mockImplementation(() => new Promise<void>((_resolve, reject) => { fail = reject }))
  render(tree()); await screen.findByRole('heading', { name: 'Endre brukernavn' })
  expand('Endre brukernavn')
  fireEvent.change(within(form('Endre brukernavn')).getByLabelText('Nåværende passord'), { target: { value: 'current-password' } })
  fireEvent.submit(form('Endre brukernavn'))
  const details = form('Endre brukernavn').querySelector('details')
  if (!details) throw new Error('Missing disclosure')
  details.open = false
  await screen.findByText('Lagrer og kontrollerer endringen …')
  await act(async () => fail(new ApiHttpError(409, 'username_unavailable', 'duplicate')))
  expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Brukernavnet er opptatt. Velg et annet.')
  expect(screen.getByRole('alert').closest('details')).toBeNull()
})

it('shows precise password overflow outside the collapsed section and accepts multibyte minimum', async () => {
  const save = vi.spyOn(profileApi, 'password').mockResolvedValue()
  render(tree()); await screen.findByRole('heading', { name: 'Endre passord' }); expand('Endre passord')
  const fields = within(form('Endre passord'))
  expect(fields.getByText(/Bruk et langt passord/)).toBeTruthy()
  for (const label of ['Nytt passord', 'Gjenta nytt passord']) fireEvent.change(fields.getByLabelText(label), { target: { value: 'ø'.repeat(65) } })
  fireEvent.change(fields.getByLabelText('Nåværende passord'), { target: { value: 'current-password' } })
  fireEvent.submit(form('Endre passord'))
  expect(await screen.findByRole('alert')).toHaveProperty('textContent', expect.stringContaining('Grensen er 128 byte'))
  expect(screen.getByRole('alert').closest('details')).toBeNull()
  expect(save).not.toHaveBeenCalled()
  for (const label of ['Nytt passord', 'Gjenta nytt passord']) fireEvent.change(fields.getByLabelText(label), { target: { value: 'ø'.repeat(6) } })
  fireEvent.submit(form('Endre passord'))
  await waitFor(() => expect(save).toHaveBeenCalledWith(2, 'ø'.repeat(6), 'current-password', session.csrf_token))
})
