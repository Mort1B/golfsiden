// @vitest-environment jsdom
import { StrictMode } from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys } from '../../api/auth'
import { api } from '../../api/client'
import { ApiHttpError } from '../../api/http'
import { passwordRecoveryApi, type RecoveryPreview } from '../../api/passwordRecovery'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { session } from '../tournaments/lifecycle/__tests__/fixtures'
import { ResetPasswordPage } from '../../pages/ResetPasswordPage'
import { SignInPage } from '../../pages/SignInPage'
const id = '00000000-0000-0000-0000-000000000099'
const token = 'a'.repeat(43)
let client: QueryClient
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
function tree(fragment = `#token=${token}`) {
  const router = createMemoryRouter([{ path: '/reset-password/:grantId', element: <ResetPasswordPage /> }, { path: '/login', element: <SignInPage /> }], { initialEntries: [`/reset-password/${id}${fragment}`] })
  const result = render(<StrictMode><QueryClientProvider client={client}><AuthContext value={auth}><RouterProvider router={router} /></AuthContext></QueryClientProvider></StrictMode>)
  return { ...result, router }
}
function fill(password = ' long password ', repeat = password) {
  fireEvent.change(screen.getByLabelText('Nytt passord'), { target: { value: password } })
  fireEvent.change(screen.getByLabelText('Gjenta nytt passord'), { target: { value: repeat } })
  fireEvent.click(screen.getByRole('button', { name: 'Lagre nytt passord' }))
}
beforeEach(() => {
  client = new QueryClient(); client.setQueryData(authKeys.session, session)
  vi.spyOn(passwordRecoveryApi, 'preview').mockImplementation(async grantId => ({ id: grantId, expires_at: '2099-01-01T12:00:00Z' }))
  vi.spyOn(passwordRecoveryApi, 'redeem').mockResolvedValue()
  vi.spyOn(api, 'session').mockResolvedValue(session)
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })
it('captures fragments in StrictMode, validates repeats and byte length, preserves spaces and another signed-in account on success', async () => {
  window.history.replaceState(null, '', `/reset-password/${id}#token=${token}`)
  const { router } = tree()
  await screen.findByLabelText('Nytt passord')
  expect(window.location.hash).toBe('')
  expect(passwordRecoveryApi.preview).toHaveBeenCalledWith(id, token)
  fill('short')
  await screen.findByText(/Passordet er for kort/)
  fill('ø'.repeat(65))
  await screen.findByText(/Passordet er for langt/)
  fill('valid password', 'different password')
  await screen.findByText('Passordene må være like.')
  expect(passwordRecoveryApi.redeem).not.toHaveBeenCalled()
  fill()
  await screen.findByText(/Kontoens tidligere økter/)
  expect(passwordRecoveryApi.redeem).toHaveBeenCalledWith(id, token, ' long password ', ' long password ')
  expect(auth.signOut).not.toHaveBeenCalled()
  expect(client.getQueryData(authKeys.session)).toEqual(session)
  expect(screen.queryByLabelText('Nytt passord')).toBeNull()
  await waitFor(() => expect(client.getMutationCache().getAll()).toHaveLength(0))
  fireEvent.click(screen.getByRole('link', { name: 'Gå til innlogging' }))
  await screen.findByText(/Logg inn på den gjenopprettede kontoen/)
  expect(router.state.location.pathname).toBe('/login')
  fireEvent.click(screen.getByText('Glemt passord?'))
  expect(screen.getByText(/Kontakt en administrator for turneringen din/)).toBeTruthy()
})
it.each(['', '#bad', '#token=short'])('shows generic invalid state without requests for missing/malformed %s', (fragment) => {
  tree(fragment)
  expect(screen.getByRole('alert').textContent).toContain('Lenken er ugyldig')
  expect(passwordRecoveryApi.preview).not.toHaveBeenCalled()
})
it('handles preview loading, transient retry and used-link responses without revealing server messages', async () => {
  let finish: (result: RecoveryPreview) => void = () => { throw new Error('missing') }
  vi.mocked(passwordRecoveryApi.preview).mockRejectedValue(new Error('sensitive'))
  tree()
  await screen.findByRole('alert')
  expect(screen.queryByText('sensitive')).toBeNull()
  vi.mocked(passwordRecoveryApi.preview).mockImplementation(() => new Promise(resolve => { finish = resolve }))
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
  await screen.findByText('Kontrollerer lenken …')
  await act(async () => { finish({ id, expires_at: '2099-01-01T12:00:00Z' }) })
  await screen.findByLabelText('Nytt passord')
  vi.mocked(passwordRecoveryApi.redeem).mockRejectedValue(new ApiHttpError(409, 'password_recovery_invalid', 'sensitive'))
  fill()
  await screen.findByText(/Lenken er ugyldig, utløpt/)
  expect(screen.queryByLabelText('Nytt passord')).toBeNull()
})
it('replaces secret and form state for a new fragment on the same grant and a new grant route', async () => {
  const { router } = tree()
  await screen.findByLabelText('Nytt passord')
  fireEvent.change(screen.getByLabelText('Nytt passord'), { target: { value: 'old draft secret' } })
  const next = 'b'.repeat(43)
  await act(async () => { await router.navigate(`/reset-password/${id}#token=${next}`) })
  await waitFor(() => expect(passwordRecoveryApi.preview).toHaveBeenCalledWith(id, next))
  expect(screen.getByLabelText('Nytt passord')).toHaveProperty('value', '')
  const otherId = '00000000-0000-0000-0000-000000000098'
  await act(async () => { await router.navigate(`/reset-password/${otherId}#token=${token}`) })
  await waitFor(() => expect(passwordRecoveryApi.preview).toHaveBeenCalledWith(otherId, token))
  fill()
  await waitFor(() => expect(passwordRecoveryApi.redeem).toHaveBeenCalledWith(otherId, token, ' long password ', ' long password '))
})
it('treats reopening the identical browser fragment as a fresh visit after history removal', async () => {
  tree()
  await screen.findByLabelText('Nytt passord')
  fireEvent.change(screen.getByLabelText('Nytt passord'), { target: { value: 'discard this draft' } })
  const before = vi.mocked(passwordRecoveryApi.preview).mock.calls.length
  await act(async () => {
    window.history.replaceState(null, '', `/reset-password/${id}#token=${token}`)
    window.dispatchEvent(new HashChangeEvent('hashchange'))
  })
  await waitFor(() => expect(vi.mocked(passwordRecoveryApi.preview).mock.calls.length).toBeGreaterThan(before))
  expect(screen.getByLabelText('Nytt passord')).toHaveProperty('value', '')
  expect(window.location.hash).toBe('')
})
