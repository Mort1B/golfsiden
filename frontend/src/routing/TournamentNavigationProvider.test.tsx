// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { useContext, useEffect } from 'react'
import { createMemoryRouter, RouterProvider, useLocation } from 'react-router-dom'
import { afterEach, expect, it, vi } from 'vitest'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { session } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { TournamentNavigationProvider } from './TournamentNavigationProvider'
import { isScoreResumeSearch, TournamentNavigationContext, tournamentNavigationLinks, type TournamentNavigationSelection } from './tournamentNavigation'

const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
const callbacks = new Map<string, (selection: TournamentNavigationSelection | null, validRoundIds?: readonly string[]) => void>()
function Probe() {
  const value = useContext(TournamentNavigationContext)
  const { pathname } = useLocation()
  useEffect(() => { if (value) callbacks.set(pathname, value.publish) }, [value, pathname])
  const links = tournamentNavigationLinks(value?.selection ?? null)
  return <><a href={links.score}>Score</a><a href={links.results}>Results</a><output>{value?.pending ? 'pending' : 'ready'}</output>
    <button onClick={() => value?.publish({ tournamentId: pathname.slice(1), roundId: `${pathname.slice(1)}-round`, scope: 'tournament', metric: 'gross' })}>Loaded</button>
    <button onClick={() => value?.publish(null)}>Denied</button></>
}
function mount() {
  const router = createMemoryRouter([{ path: '*', element: <TournamentNavigationProvider><Probe /></TournamentNavigationProvider> }], { initialEntries: ['/A'] })
  const tree = (value: AuthContextValue) => <AuthContext value={value}><RouterProvider router={router} /></AuthContext>
  const view = render(tree(auth))
  return { router, ...view, tree }
}
afterEach(() => { cleanup(); callbacks.clear() })
const href = () => screen.getByRole('link', { name: 'Score' }).getAttribute('href')

it('retains validated context on neutral pages, blocks old context on route changes, and rejects late publishers', async () => {
  const { router } = mount()
  expect(screen.getByRole('status').textContent).toBe('pending')
  fireEvent.click(screen.getByText('Loaded'))
  const old = callbacks.get('/A')
  expect(href()).toBe('/score?tournament=A&round=A-round&resume=1')
  await act(() => router.navigate('/profile/'))
  expect(href()).toContain('tournament=A')
  await act(() => router.navigate('/tournaments/'))
  expect(href()).toContain('tournament=A')
  await act(() => router.navigate('/B'))
  expect(href()).toBe('/score')
  expect(screen.getByRole('status').textContent).toBe('pending')
  act(() => old?.({ tournamentId: 'A', roundId: 'old-round' }))
  expect(href()).toBe('/score')
  fireEvent.click(screen.getByText('Loaded'))
  expect(href()).toContain('tournament=B&round=B-round')
  act(() => old?.(null))
  expect(href()).toContain('tournament=B')
  fireEvent.click(screen.getByText('Denied'))
  await act(() => router.navigate('/profile'))
  expect(href()).toBe('/score')
})
it('clears account context synchronously and rejects old-account publishers', async () => {
  const { router, rerender, tree } = mount()
  fireEvent.click(screen.getByText('Loaded'))
  const old = callbacks.get('/A')
  await act(() => router.navigate('/profile'))
  rerender(tree({ ...auth, session: { ...session, user_id: 'other-user' } }))
  expect(href()).toBe('/score')
  act(() => old?.({ tournamentId: 'A' }))
  expect(href()).toBe('/score')
  rerender(tree({ ...auth, session: null }))
  rerender(tree(auth))
  expect(href()).toBe('/score')
})
it('carries metric and scope without carrying an old round into a different tournament', async () => {
  const { router } = mount()
  fireEvent.click(screen.getByText('Loaded'))
  await act(() => router.navigate('/B'))
  act(() => callbacks.get('/B')?.({ tournamentId: 'B' }))
  expect(href()).toBe('/score?tournament=B&resume=1')
  expect(screen.getByRole('link', { name: 'Results' }).getAttribute('href')).toBe('/leaderboard?tournament=B&scope=tournament&metric=gross')
})
it('only contextual resume entries use fresh gap selection; explicit card URLs remain explicit', () => {
  expect(isScoreResumeSearch('')).toBe(true)
  expect(isScoreResumeSearch('?tournament=A&round=B&resume=1')).toBe(true)
  for (const extra of ['owner=1', 'owner_type=team', 'hole=8', 'view=summary', 'unexpected=1']) {
    expect(isScoreResumeSearch(`?tournament=A&resume=1&${extra}`)).toBe(false)
  }
  expect(isScoreResumeSearch('?tournament=A&round=B')).toBe(false)
})

it('retains an earlier round on tournament-only pages until authoritative rounds or explicit selection clear it', async () => {
  const { router } = mount()
  fireEvent.click(screen.getByText('Loaded'))
  await act(() => router.navigate('/detail'))
  act(() => callbacks.get('/detail')?.({ tournamentId: 'A' }, ['A-round', 'later-round']))
  expect(href()).toContain('round=A-round')
  await act(() => router.navigate('/management'))
  act(() => callbacks.get('/management')?.({ tournamentId: 'A' }, ['later-round']))
  expect(href()).toBe('/score?tournament=A&resume=1')
  act(() => callbacks.get('/management')?.({ tournamentId: 'A', roundId: 'later-round' }))
  expect(href()).toContain('round=later-round')
  act(() => callbacks.get('/management')?.({ tournamentId: 'A', roundId: null }))
  expect(href()).toBe('/score?tournament=A&resume=1')
})
