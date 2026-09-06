// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { tournamentApi, tournamentKeys } from '../../../api/tournaments'
import { ApiHttpError } from '../../../api/http'
import { clearPrivateWorkspace } from '../../../api/privateWorkspace'
import type { Round, Tournament } from '../../../api/types'
import { AuthContext, type AuthContextValue } from '../../auth/authContext'
import { round, session, tournament as activeTournament } from '../lifecycle/__tests__/fixtures'
import { TournamentArchivePanel } from './TournamentArchivePanel'

const tournament: Tournament = { ...activeTournament, status: 'completed' }
let client: QueryClient
let server: Tournament
let serverRounds: Round[]
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
function Harness({ refreshing = false }: { refreshing?: boolean }) {
  const detail = useQuery({ queryKey: tournamentKeys.detail(session.user_id, tournament.id), queryFn: () => tournamentApi.detail(tournament.id) })
  const rounds = useQuery({ queryKey: tournamentKeys.rounds(session.user_id, tournament.id), queryFn: () => tournamentApi.rounds(tournament.id) })
  if (!detail.data || detail.error) return <p>Turneringslesing utilgjengelig</p>
  return <TournamentArchivePanel tournament={detail.data}
    authorityRefreshing={refreshing || detail.isFetching || rounds.isFetching}
    snapshotVersion={`${detail.dataUpdatedAt}:${rounds.dataUpdatedAt}`} />
}
function tree(refreshing = false) {
  return <QueryClientProvider client={client}><AuthContext value={auth}><MemoryRouter><Harness refreshing={refreshing} /></MemoryRouter></AuthContext></QueryClientProvider>
}
async function enabled() {
  const button = await screen.findByRole('button', { name: 'Arkiver turneringen' })
  await waitFor(() => expect(button.hasAttribute('disabled')).toBe(false))
  return button
}
async function confirm() {
  fireEvent.click(await enabled())
  fireEvent.click(screen.getByRole('button', { name: 'Bekreft og arkiver turneringen' }))
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  server = tournament
  serverRounds = [{ ...round, status: 'locked' }]
  vi.spyOn(tournamentApi, 'detail').mockImplementation(async () => server)
  vi.spyOn(tournamentApi, 'rounds').mockImplementation(async () => serverRounds)
  vi.spyOn(tournamentApi, 'archive').mockImplementation(async () => { server = { ...server, status: 'archived' }; return server })
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })

it('requires confirmation, supports Escape/focus and completes with the exact version', async () => {
  render(tree())
  const button = await enabled()
  fireEvent.click(button)
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Avbryt' }))
  expect(tournamentApi.archive).not.toHaveBeenCalled()
  fireEvent.keyDown(screen.getByRole('group'), { key: 'Escape' })
  expect(document.activeElement).toBe(button)
  await confirm()
  await screen.findByRole('heading', { name: 'Turneringen er arkivert' })
  expect(tournamentApi.archive).toHaveBeenCalledTimes(1)
  expect(tournamentApi.archive).toHaveBeenCalledWith(tournament.id, tournament.updated_at, session.csrf_token)
  expect(screen.queryByRole('button', { name: 'Arkiver turneringen' })).toBeNull()
})
it('discards confirmation permanently across authority refresh even when facts are unchanged', async () => {
  const view = render(tree())
  fireEvent.click(await enabled())
  view.rerender(tree(true))
  expect(screen.queryByRole('group')).toBeNull()
  view.rerender(tree(false))
  expect(screen.queryByRole('group')).toBeNull()
  expect(tournamentApi.archive).not.toHaveBeenCalled()
})
it('refreshes conflicts and requires a new confirmation', async () => {
  vi.mocked(tournamentApi.archive).mockRejectedValue(new ApiHttpError(409, 'tournament_archive_stale', 'stale'))
  render(tree())
  await confirm()
  await screen.findByText('Turneringen er endret. Se oppdatert status og kontroller på nytt.')
  expect(screen.queryByRole('group')).toBeNull()
  await enabled()
})
it('reconciles a lost successful response into the completed read-only state', async () => {
  vi.mocked(tournamentApi.archive).mockImplementation(async () => { server = { ...server, status: 'archived' }; throw new Error('network') })
  render(tree())
  await confirm()
  await screen.findByRole('heading', { name: 'Turneringen er arkivert' })
  expect(screen.queryByRole('button', { name: 'Arkiver turneringen' })).toBeNull()
})
it('blocks after failed reconciliation until a successful explicit refresh', async () => {
  vi.mocked(tournamentApi.archive).mockImplementation(async () => {
    vi.mocked(tournamentApi.rounds).mockRejectedValue(new Error('offline'))
    throw new Error('uncertain')
  })
  render(tree())
  await confirm()
  await screen.findByText(/Oppdateringen mislyktes/)
  expect(screen.getByRole('button', { name: 'Arkiver turneringen' }).hasAttribute('disabled')).toBe(true)
  vi.mocked(tournamentApi.rounds).mockResolvedValue(serverRounds)
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater arkiveringskontrollen' }))
  await enabled()
})
it('prevents duplicate submissions and never recreates private data after unmount', async () => {
  let finish: ((trip: Tournament) => void) | undefined
  vi.mocked(tournamentApi.archive).mockImplementation(() => new Promise((resolve) => { finish = resolve }))
  const view = render(tree())
  fireEvent.click(await enabled())
  const button = screen.getByRole('button', { name: 'Bekreft og arkiver turneringen' })
  fireEvent.click(button); fireEvent.click(button)
  await waitFor(() => expect(tournamentApi.archive).toHaveBeenCalledTimes(1))
  view.unmount()
  clearPrivateWorkspace(client)
  await act(async () => { finish?.({ ...server, status: 'archived' }) })
  expect(client.getQueryCache().getAll()).toHaveLength(0)
})
it('supersedes a pre-commit read so it cannot restore an actionable active state', async () => {
  let finish: ((trip: Tournament) => void) | undefined
  let stale: ((trip: Tournament) => void) | undefined
  vi.mocked(tournamentApi.archive).mockImplementation(() => new Promise((resolve) => { finish = resolve }))
  render(tree())
  await confirm()
  vi.mocked(tournamentApi.detail).mockImplementationOnce(() => new Promise((resolve) => { stale = resolve }))
  await act(async () => { void client.invalidateQueries({ queryKey: tournamentKeys.detail(session.user_id, tournament.id) }) })
  await act(async () => { server = { ...server, status: 'archived' }; finish?.(server) })
  await screen.findByRole('heading', { name: 'Turneringen er arkivert' })
  await act(async () => { stale?.(tournament) })
  expect(screen.queryByRole('button', { name: 'Arkiver turneringen' })).toBeNull()
})
it('allows a newer live refresh to supersede post-commit reconciliation', async () => {
  let stale: ((trip: Tournament) => void) | undefined
  render(tree())
  await enabled()
  vi.mocked(tournamentApi.detail).mockImplementationOnce(() => new Promise((resolve) => { stale = resolve }))
  await confirm()
  await waitFor(() => expect(stale).toBeDefined())
  await act(async () => { await client.invalidateQueries({ queryKey: tournamentKeys.detail(session.user_id, tournament.id) }) })
  await screen.findByRole('heading', { name: 'Turneringen er arkivert' })
  await act(async () => { stale?.(tournament) })
  expect(screen.queryByText(/Oppdateringen mislyktes/)).toBeNull()
  expect(screen.queryByRole('button', { name: 'Arkiver turneringen' })).toBeNull()
})
it.each(['draft', 'active'] as const)('blocks archive before completion: %s', async (status) => {
  server = { ...tournament, status }
  render(tree())
  await screen.findByText('Fullfør turneringen før den arkiveres.')
  expect(screen.getByRole('button', { name: 'Arkiver turneringen' }).hasAttribute('disabled')).toBe(true)
})
it('shows archive history link without reversal or mutation controls', async () => {
  server = { ...tournament, status: 'archived' }
  render(tree())
  await screen.findByRole('heading', { name: 'Turneringen er arkivert' })
  expect(screen.getByRole('link', { name: 'Se turneringsarkivet' }).getAttribute('href')).toBe('/tournaments?view=archived')
  expect(screen.queryByRole('button', { name: 'Arkiver turneringen' })).toBeNull()
})
