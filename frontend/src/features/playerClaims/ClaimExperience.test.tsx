// @vitest-environment jsdom
import { StrictMode } from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { beforeEach, afterEach, expect, it, vi } from 'vitest'
import { authKeys } from '../../api/auth'
import { api } from '../../api/client'
import { ApiHttpError } from '../../api/http'
import { playerClaimsApi, type ClaimRegistration } from '../../api/playerClaims'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { tournament, session } from '../tournaments/lifecycle/__tests__/fixtures'
import { ClaimPage } from '../../pages/ClaimPage'
const id = '00000000-0000-0000-0000-000000000014'
const token = 'a'.repeat(43)
const preview = { tournament: { id: tournament.id, name: tournament.name }, player: { id: session.player_id ?? '', display_name: 'Forberedt spiller med langt navn' }, expires_at: '2099-01-01T12:00:00Z' }
const registration = { tournament_id: tournament.id, player_id: preview.player.id, session }
let client: QueryClient
let auth: AuthContextValue
function tree(fragment = `#token=${token}`) {
  const router = createMemoryRouter([{ path: '/claim/:claimId', element: <ClaimPage /> }], { initialEntries: [`/claim/${id}${fragment}`] })
  return { ...render(<StrictMode><QueryClientProvider client={client}><AuthContext value={auth}><RouterProvider router={router} /></AuthContext></QueryClientProvider></StrictMode>), router }
}
function fill(password = '  sufficiently long password  ') {
  fireEvent.change(screen.getByLabelText('Brukernavn'), { target: { value: 'new_player' } })
  fireEvent.change(screen.getByLabelText('Passord'), { target: { value: password } })
  fireEvent.click(screen.getByRole('button', { name: 'Ta i bruk kontoen' }))
}
beforeEach(() => {
  client = new QueryClient(); client.setQueryData(authKeys.session, null)
  auth = { session: null, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), retry: vi.fn(), establishSession: vi.fn() }
  vi.spyOn(api, 'logout').mockResolvedValue(undefined)
  vi.spyOn(playerClaimsApi, 'preview').mockResolvedValue(preview)
  vi.spyOn(playerClaimsApi, 'register').mockResolvedValue(registration)
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })
it('captures StrictMode fragment, preserves prepared identity, validates password and clears secrets after success', async () => {
  window.history.replaceState(null, '', `/claim/${id}#token=${token}`)
  tree(); await screen.findByLabelText('Passord')
  expect(window.location.hash).toBe('')
  expect(screen.getByText(preview.player.display_name)).toBeTruthy()
  expect(screen.queryByLabelText('Spillerens navn')).toBeNull()
  fill('short'); await screen.findByText(/Passordet er for kort/)
  expect(playerClaimsApi.register).not.toHaveBeenCalled()
  fill(); await screen.findByText(/Kontoen er klar/)
  expect(playerClaimsApi.register).toHaveBeenCalledWith(id, token, 'new_player', '  sufficiently long password  ', preview)
  expect(client.getQueryData(authKeys.session)).toEqual(session)
  expect(screen.queryByLabelText('Passord')).toBeNull()
  await waitFor(() => expect(client.getMutationCache().getAll()).toHaveLength(0))
  expect(client.getQueryCache().findAll({ queryKey: ['player-claim-preview'] }).every(query => query.state.data === undefined)).toBe(true)
})
it.each(['', '#token=short', '#unexpected'])('rejects malformed fragment %s without network', fragment => {
  tree(fragment); expect(screen.getByRole('alert').textContent).toContain('Lenken er ugyldig')
  expect(playerClaimsApi.preview).not.toHaveBeenCalled()
})
it('replaces secret and clears password when same claim route receives new fragment', async () => {
  const { router } = tree(); await screen.findByLabelText('Passord')
  fireEvent.change(screen.getByLabelText('Passord'), { target: { value: 'old private draft' } })
  await act(async () => { await router.navigate(`/claim/${id}#token=${'b'.repeat(43)}`) })
  await waitFor(() => expect(playerClaimsApi.preview).toHaveBeenCalledWith(id, 'b'.repeat(43), expect.any(AbortSignal)))
  expect(screen.getByLabelText('Passord')).toHaveProperty('value', '')
})
it.each(['new-session', 'unmount'])('does not publish held registration after %s', async mode => {
  let finish: (value: ClaimRegistration) => void = () => undefined
  vi.mocked(playerClaimsApi.register).mockImplementation(() => new Promise(resolve => { finish = resolve }))
  const view = tree(); await screen.findByLabelText('Passord'); fill()
  await waitFor(() => expect(playerClaimsApi.register).toHaveBeenCalled())
  expect(screen.getByLabelText('Passord')).toHaveProperty('value', '')
  if (mode === 'unmount') view.unmount()
  else client.setQueryData(authKeys.session, { ...session, csrf_token: 'concurrent-login' })
  await act(async () => { finish(registration) })
  expect(client.getQueryData(authKeys.session)).toEqual(mode === 'unmount' ? null : { ...session, csrf_token: 'concurrent-login' })
  expect(screen.queryByText(/Kontoen er klar/)).toBeNull()
})
it('clears password on username collision and invalidates consumed link without retaining mutation secrets', async () => {
  vi.mocked(playerClaimsApi.register).mockRejectedValue(new ApiHttpError(409, 'username_already_registered', 'internal'))
  tree(); await screen.findByLabelText('Passord'); fill()
  await screen.findByText('Brukernavnet er opptatt. Velg et annet.')
  expect(screen.getByLabelText('Passord')).toHaveProperty('value', '')
  vi.mocked(playerClaimsApi.register).mockRejectedValue(new ApiHttpError(410, 'claim_unavailable', 'internal'))
  fill(); await screen.findByText(/Lenken er ugyldig/)
  expect(screen.queryByLabelText('Passord')).toBeNull()
})
it('offers deliberate sign-out for an existing account', async () => {
  auth = { ...auth, session }; client.setQueryData(authKeys.session, session)
  tree(); await screen.findByRole('button', { name: 'Logg ut og fortsett' })
  expect(screen.queryByLabelText('Passord')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: 'Logg ut og fortsett' }))
  await waitFor(() => expect(api.logout).toHaveBeenCalledWith(session.csrf_token))
})
it('supports preview retry without exposing server details', async () => {
  vi.mocked(playerClaimsApi.preview).mockRejectedValue(new Error('internal private details'))
  tree(); await screen.findByRole('button', { name: 'Prøv igjen' })
  expect(screen.queryByText('internal private details')).toBeNull()
  vi.mocked(playerClaimsApi.preview).mockResolvedValue(preview)
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
  await screen.findByLabelText('Passord')
})

it('does not clear a concurrent login when held sign-out finishes', async () => {
  let finish: () => void = () => undefined
  vi.mocked(api.logout).mockImplementation(() => new Promise(resolve => { finish = () => resolve(undefined) }))
  auth = { ...auth, session }; client.setQueryData(authKeys.session, session)
  tree(); await screen.findByRole('button', { name: 'Logg ut og fortsett' })
  fireEvent.click(screen.getByRole('button', { name: 'Logg ut og fortsett' }))
  await waitFor(() => expect(api.logout).toHaveBeenCalled())
  const replacement = { ...session, csrf_token: 'replacement-login' }
  client.setQueryData(authKeys.session, replacement)
  await act(async () => { finish() })
  expect(client.getQueryData(authKeys.session)).toEqual(replacement)
})

it.each([true,false])('recovers committed registration with lost response, cookie=%s',async cookie=>{
 vi.mocked(playerClaimsApi.register).mockRejectedValue(new TypeError('lost response'))
 vi.spyOn(api,'session').mockResolvedValue(cookie?{...session,username:'new_player'}:null)
 tree();await screen.findByLabelText('Passord');fill()
 if(cookie){await screen.findByText(/Kontoen er klar/);expect(client.getQueryData(authKeys.session)).toEqual({...session,username:'new_player'})}
 else {await screen.findByText(/Kontoen kan allerede være opprettet/);expect(screen.getByRole('link',{name:/Logg inn med new_player/})).toBeTruthy();expect(screen.queryByLabelText('Passord')).toBeNull()}
 expect(playerClaimsApi.register).toHaveBeenCalledTimes(1)
})

it.each(['mismatch','read-failure'])('offers normal login when session recovery gives %s',async mode=>{
 vi.mocked(playerClaimsApi.register).mockRejectedValue(new TypeError('lost'))
 const read=vi.spyOn(api,'session')
 if(mode==='mismatch')read.mockResolvedValue({...session,player_id:id,username:'new_player'})
 else read.mockRejectedValue(new TypeError('unreachable'))
 tree();await screen.findByLabelText('Passord');fill()
 await screen.findByText(/Kontoen kan allerede være opprettet/)
 expect(client.getQueryData(authKeys.session)).toBeNull()
 expect(screen.queryByLabelText('Passord')).toBeNull()
})
it.each(['concurrent-login','unmount'])('fences held lost-response reconciliation after %s',async mode=>{
 vi.mocked(playerClaimsApi.register).mockRejectedValue(new TypeError('lost'))
 let finish:(value:typeof session)=>void=()=>{}
 const read=vi.spyOn(api,'session').mockImplementation(()=>new Promise(resolve=>{finish=resolve}))
 const mounted=tree();await screen.findByLabelText('Passord');fill();await waitFor(()=>expect(read).toHaveBeenCalled())
 const replacement={...session,csrf_token:'concurrent-login'}
 if(mode==='unmount')mounted.unmount();else client.setQueryData(authKeys.session,replacement)
 await act(async()=>finish({...session,username:'new_player'}))
 expect(client.getQueryData(authKeys.session)).toEqual(mode==='unmount'?null:replacement)
 expect(screen.queryByText(/Kontoen er klar/)).toBeNull()
})
