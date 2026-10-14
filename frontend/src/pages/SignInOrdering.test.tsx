// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom'
import { afterEach, expect, it, vi } from 'vitest'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { session } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { SignInPage } from './SignInPage'

afterEach(cleanup)
function Location() { return <output aria-label="Location">{useLocation().pathname}</output> }
it('keeps a failed deliberate login reviewable after an earlier creator cookie is reconciled', async () => {
  const signIn = vi.fn().mockRejectedValueOnce(new Error('Feil brukernavn eller passord')).mockResolvedValue(session)
  let auth: AuthContextValue = { session: null, loading: false, error: null, signIn,
    signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
  const tree = () => <AuthContext value={auth}><MemoryRouter initialEntries={['/login?returnTo=%2Ftournaments']}>
    <Routes><Route path="/login" element={<SignInPage />} /><Route path="/tournaments" element={<p>Turneringer</p>} /></Routes><Location />
  </MemoryRouter></AuthContext>
  const view = render(tree())
  fireEvent.change(screen.getByLabelText('Brukernavn'), { target: { value: 'newer' } })
  fireEvent.change(screen.getByLabelText('Passord'), { target: { value: 'wrong password' } })
  fireEvent.click(screen.getByRole('button', { name: 'Logg inn' }))
  await screen.findByRole('alert')
  auth = { ...auth, session }
  view.rerender(tree())
  expect(screen.getByRole('alert').textContent).toContain('Feil brukernavn eller passord')
  expect(screen.getByLabelText('Location').textContent).toBe('/login')
  fireEvent.change(screen.getByLabelText('Passord'), { target: { value: 'correct password' } })
  fireEvent.click(screen.getByRole('button', { name: 'Logg inn' }))
  await screen.findByText('Turneringer')
  expect(signIn).toHaveBeenCalledTimes(2)
})
