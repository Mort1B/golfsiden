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
import { round, session, tournament } from '../lifecycle/__tests__/fixtures'
import { TournamentCompletionPanel } from './TournamentCompletionPanel'

let client: QueryClient
let server: Tournament
let serverRounds: Round[]
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
function Harness({ refreshing = false }: { refreshing?: boolean }) {
  const detail = useQuery({ queryKey: tournamentKeys.detail(session.user_id, tournament.id), queryFn: () => tournamentApi.detail(tournament.id) })
  const rounds = useQuery({ queryKey: tournamentKeys.rounds(session.user_id, tournament.id), queryFn: () => tournamentApi.rounds(tournament.id) })
  if (!detail.data || detail.error) return <p>Turneringslesing utilgjengelig</p>
  return <TournamentCompletionPanel tournament={detail.data} rounds={{ data: rounds.data, pending: rounds.isPending, error: rounds.error }}
    authorityRefreshing={refreshing || detail.isFetching || rounds.isFetching}
    snapshotVersion={`${detail.dataUpdatedAt}:${rounds.dataUpdatedAt}`} />
}
function tree(refreshing = false) {
  return <QueryClientProvider client={client}><AuthContext value={auth}><MemoryRouter><Harness refreshing={refreshing} /></MemoryRouter></AuthContext></QueryClientProvider>
}
async function enabled() {
  const button = await screen.findByRole('button', { name: 'Fullfør turneringen' })
  await waitFor(() => expect(button.hasAttribute('disabled')).toBe(false))
  return button
}
async function confirm() {
  fireEvent.click(await enabled())
  fireEvent.click(screen.getByRole('button', { name: 'Bekreft og fullfør turneringen' }))
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  server = tournament
  serverRounds = [{ ...round, status: 'locked' }]
  vi.spyOn(tournamentApi, 'detail').mockImplementation(async () => server)
  vi.spyOn(tournamentApi, 'rounds').mockImplementation(async () => serverRounds)
  vi.spyOn(tournamentApi, 'complete').mockImplementation(async () => { server = { ...server, status: 'completed' }; return server })
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })

it('requires confirmation, supports Escape/focus and completes with the exact version', async () => {
  render(tree())
  const button = await enabled()
  fireEvent.click(button)
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Avbryt' }))
  expect(tournamentApi.complete).not.toHaveBeenCalled()
  fireEvent.keyDown(screen.getByRole('group'), { key: 'Escape' })
  expect(document.activeElement).toBe(button)
  await confirm()
  await screen.findByRole('heading', { name: 'Turneringen er avsluttet' })
  expect(tournamentApi.complete).toHaveBeenCalledTimes(1)
  expect(tournamentApi.complete).toHaveBeenCalledWith(tournament.id, tournament.updated_at, session.csrf_token)
  expect(screen.queryByRole('button', { name: 'Fullfør turneringen' })).toBeNull()
})
it('names unlocked rounds and preserves exact management links', async () => {
  serverRounds = [{ ...round, status: 'completed' }]
  render(tree())
  const link = await screen.findByRole('link', { name: `Runde 1: ${round.name}` })
  expect(link.getAttribute('href')).toBe(`/manage/tournaments/${tournament.id}?round=${round.id}#lifecycle`)
  expect(screen.getByRole('button', { name: 'Fullfør turneringen' }).hasAttribute('disabled')).toBe(true)
})
it('discards confirmation permanently across authority refresh even when facts are unchanged', async () => {
  const view = render(tree())
  fireEvent.click(await enabled())
  view.rerender(tree(true))
  expect(screen.queryByRole('group')).toBeNull()
  view.rerender(tree(false))
  expect(screen.queryByRole('group')).toBeNull()
  expect(tournamentApi.complete).not.toHaveBeenCalled()
})
it('refreshes conflicts and requires a new confirmation', async () => {
  vi.mocked(tournamentApi.complete).mockRejectedValue(new ApiHttpError(409, 'tournament_completion_stale', 'stale'))
  render(tree())
  await confirm()
  await screen.findByText('Turneringen er endret. Se oppdatert status og kontroller på nytt.')
  expect(screen.queryByRole('group')).toBeNull()
  await enabled()
})
it('reconciles a lost successful response into the completed read-only state', async () => {
  vi.mocked(tournamentApi.complete).mockImplementation(async () => { server = { ...server, status: 'completed' }; throw new Error('network') })
  render(tree())
  await confirm()
  await screen.findByRole('heading', { name: 'Turneringen er avsluttet' })
  expect(screen.queryByRole('button', { name: 'Fullfør turneringen' })).toBeNull()
})
it('blocks after failed reconciliation until a successful explicit refresh', async () => {
  vi.mocked(tournamentApi.complete).mockImplementation(async () => {
    vi.mocked(tournamentApi.rounds).mockRejectedValue(new Error('offline'))
    throw new Error('uncertain')
  })
  render(tree())
  await confirm()
  await screen.findByText(/Oppdateringen mislyktes/)
  expect(screen.getByRole('button', { name: 'Fullfør turneringen' }).hasAttribute('disabled')).toBe(true)
  vi.mocked(tournamentApi.rounds).mockResolvedValue(serverRounds)
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater fullføringskontrollen' }))
  await enabled()
})
it('prevents duplicate submissions and never recreates private data after unmount', async () => {
  let finish: ((trip: Tournament) => void) | undefined
  vi.mocked(tournamentApi.complete).mockImplementation(() => new Promise((resolve) => { finish = resolve }))
  const view = render(tree())
  fireEvent.click(await enabled())
  const button = screen.getByRole('button', { name: 'Bekreft og fullfør turneringen' })
  fireEvent.click(button); fireEvent.click(button)
  await waitFor(() => expect(tournamentApi.complete).toHaveBeenCalledTimes(1))
  view.unmount()
  clearPrivateWorkspace(client)
  await act(async () => { finish?.({ ...server, status: 'completed' }) })
  expect(client.getQueryCache().getAll()).toHaveLength(0)
})
it('supersedes a pre-commit read so it cannot restore an actionable active state', async () => {
  let finish: ((trip: Tournament) => void) | undefined
  let stale: ((trip: Tournament) => void) | undefined
  vi.mocked(tournamentApi.complete).mockImplementation(() => new Promise((resolve) => { finish = resolve }))
  render(tree())
  await confirm()
  vi.mocked(tournamentApi.detail).mockImplementationOnce(() => new Promise((resolve) => { stale = resolve }))
  await act(async () => { void client.invalidateQueries({ queryKey: tournamentKeys.detail(session.user_id, tournament.id) }) })
  await act(async () => { server = { ...server, status: 'completed' }; finish?.(server) })
  await screen.findByRole('heading', { name: 'Turneringen er avsluttet' })
  await act(async () => { stale?.(tournament) })
  expect(screen.queryByRole('button', { name: 'Fullfør turneringen' })).toBeNull()
})
it('allows a newer live refresh to supersede post-commit reconciliation', async () => {
  let stale: ((trip: Tournament) => void) | undefined
  render(tree())
  await enabled()
  vi.mocked(tournamentApi.detail).mockImplementationOnce(() => new Promise((resolve) => { stale = resolve }))
  await confirm()
  await waitFor(() => expect(stale).toBeDefined())
  await act(async () => { await client.invalidateQueries({ queryKey: tournamentKeys.detail(session.user_id, tournament.id) }) })
  await screen.findByRole('heading', { name: 'Turneringen er avsluttet' })
  await act(async () => { stale?.(tournament) })
  expect(screen.queryByText(/Oppdateringen mislyktes/)).toBeNull()
  expect(screen.queryByRole('button', { name: 'Fullfør turneringen' })).toBeNull()
})
it('shows deliberate empty and pending round states', async () => {
  vi.mocked(tournamentApi.rounds).mockImplementation(() => new Promise(() => {}))
  render(tree())
  await screen.findByRole('status')
  expect(screen.getByRole('button', { name: 'Fullfør turneringen' }).hasAttribute('disabled')).toBe(true)
  await act(async () => { client.setQueryData(tournamentKeys.rounds(session.user_id, tournament.id), []) })
  await screen.findByText(/Rundeplanen er ufullstendig/)
})
