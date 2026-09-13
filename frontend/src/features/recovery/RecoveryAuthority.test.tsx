// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { authKeys } from '../../api/auth'
import type { TournamentPlayerRoster } from '../../api/types'
import { session } from '../tournaments/lifecycle/__tests__/fixtures'
import { TournamentPlayerSection } from '../tournaments/TournamentPlayerSection'
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
const roster: TournamentPlayerRoster = { handicap_correction: { state: 'locked', reason: 'round_opened' }, players: [session.player_id ?? '', 'other-player'].map((player_id, index) => ({ tournament_id: 'tour', player_id, display_name: index ? 'Other' : 'Self', player_active: true, tournament_handicap: 10, seed: null, status: 'active', created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' })) }
afterEach(cleanup)
it('hides self recovery, clears disclosure secrets on authority changes and keeps handicap locking independent', () => {
  const client = new QueryClient(); client.setQueryData(authKeys.session, session)
  const tree = (adminAccessPending = false, adminAccessError: Error | null = null, isAdmin = true, csrf = session.csrf_token, recoveryAccessPending = false) => <QueryClientProvider client={client}><AuthContext value={{ ...auth, session: { ...session, csrf_token: csrf } }}><TournamentPlayerSection tournamentId="tour" isAdmin={isAdmin} roster={roster} pending={false} error={null} recoveryAccessPending={recoveryAccessPending} onRetry={vi.fn()} adminAccessPending={adminAccessPending} adminAccessError={adminAccessError} /></AuthContext></QueryClientProvider>
  const view = render(tree())
  expect(screen.queryByRole('button', { name: /glemt passord for Self/ })).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: /glemt passord for Other/ }))
  fireEvent.change(screen.getByLabelText('Ditt nåværende passord'), { target: { value: 'private password' } })
  view.rerender(tree(false, null, true, 'changed-session'))
  expect(screen.queryByLabelText('Ditt nåværende passord')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: /glemt passord for Other/ }))
  view.rerender(tree(false, null, true, 'changed-session', true))
  expect(screen.queryByRole('button', { name: /passordhjelp for Other/ })).toBeNull()
  expect(screen.queryByLabelText('Ditt nåværende passord')).toBeNull()
  view.rerender(tree(true)); expect(screen.queryByRole('button', { name: /glemt passord/ })).toBeNull()
  view.rerender(tree(false, new Error('denied'))); expect(screen.queryByRole('button', { name: /glemt passord/ })).toBeNull()
  view.rerender(tree(false, null, false)); expect(screen.queryByRole('button', { name: /glemt passord/ })).toBeNull()
  client.clear()
})
