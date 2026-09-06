// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, useLocation, useNavigate } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../api/client'
import { tournamentKeys, type MyTournament } from '../api/tournaments'
import { clearPrivateWorkspace } from '../api/privateWorkspace'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { session, tournament } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { TournamentsPage } from './TournamentsPage'

let client: QueryClient
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
const items: MyTournament[] = [
  { tournament: { ...tournament, name: 'Fullført tur', status: 'completed' }, role: 'admin', player_id: null },
  { tournament: { ...tournament, id: '00000000-0000-0000-0000-000000000099', name: 'Arkivert tur', status: 'archived' }, role: 'viewer', player_id: null },
]
function Navigation() {
  const navigate = useNavigate()
  return <><p>{useLocation().search}</p><button onClick={() => void navigate(-1)}>Tilbake</button><TournamentsPage /></>
}
function tree(path = '/tournaments', value = auth) {
  return <QueryClientProvider client={client}><AuthContext value={value}><MemoryRouter initialEntries={[path]}><Navigation /></MemoryRouter></AuthContext></QueryClientProvider>
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  vi.spyOn(api, 'myTournaments').mockResolvedValue(items)
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })

it('keeps completed tournaments current, links archived history and preserves cache/URL back navigation', async () => {
  render(tree())
  await screen.findByRole('heading', { name: 'Fullført tur' })
  expect(screen.queryByRole('heading', { name: 'Arkivert tur' })).toBeNull()
  fireEvent.click(screen.getByRole('link', { name: 'Arkiv (1)' }))
  await screen.findByRole('heading', { name: 'Arkivert tur' })
  expect(screen.getByRole('link', { name: /Arkivert tur/ }).getAttribute('href')).toBe(`/tournaments/${items[1]?.tournament.id}`)
  expect(screen.getByRole('link', { name: 'Arkiv (1)' }).getAttribute('aria-current')).toBe('page')
  fireEvent.click(screen.getByRole('link', { name: 'Alle (2)' }))
  await screen.findByRole('heading', { name: 'Fullført tur' })
  expect(screen.getByRole('heading', { name: 'Arkivert tur' })).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Tilbake' }))
  expect(screen.queryByRole('heading', { name: 'Fullført tur' })).toBeNull()
  expect(client.getQueryData(tournamentKeys.mine(session.user_id))).toEqual(items)
})
it('separates no memberships from an empty selected view', async () => {
  vi.mocked(api.myTournaments).mockResolvedValue(items.slice(0, 1))
  render(tree('/tournaments?view=archived'))
  await screen.findByText('Ingen arkiverte turneringer ennå.')
  vi.mocked(api.myTournaments).mockResolvedValue([])
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater turneringer' }))
  await screen.findByText('Du er ikke med i noen turneringer ennå.')
})
it('hides stale cards after failed refresh and restores them after retry', async () => {
  render(tree('/tournaments?view=all'))
  await screen.findByRole('heading', { name: 'Arkivert tur' })
  vi.mocked(api.myTournaments).mockRejectedValue(new Error('offline'))
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater turneringer' }))
  await screen.findByRole('alert')
  expect(screen.queryByRole('heading', { name: 'Arkivert tur' })).toBeNull()
  vi.mocked(api.myTournaments).mockResolvedValue(items)
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
  await screen.findByRole('heading', { name: 'Arkivert tur' })
})
it('refreshes current to archive without removing the unfiltered membership', async () => {
  render(tree())
  await screen.findByRole('heading', { name: 'Fullført tur' })
  vi.mocked(api.myTournaments).mockResolvedValue(items.map((item) => ({ ...item, tournament: { ...item.tournament, status: 'archived' } })))
  await act(async () => { await client.invalidateQueries({ queryKey: tournamentKeys.mine(session.user_id) }) })
  await screen.findByText('Ingen nåværende turneringer. Tidligere turneringer finnes i Arkiv.')
  fireEvent.click(screen.getByRole('link', { name: 'Arkiv (2)' }))
  await screen.findByRole('heading', { name: 'Fullført tur' })
})
it('shows loading and clears the prior account list before a new identity read', async () => {
  const view = render(tree('/tournaments?view=all'))
  await screen.findByRole('heading', { name: 'Arkivert tur' })
  vi.mocked(api.myTournaments).mockImplementation(() => new Promise(() => {}))
  act(() => clearPrivateWorkspace(client))
  view.rerender(tree('/tournaments?view=all', { ...auth, session: { ...session, user_id: 'new-user' } }))
  await waitFor(() => expect(screen.queryByRole('heading', { name: 'Arkivert tur' })).toBeNull())
  expect(screen.getByRole('status')).toBeTruthy()
})
