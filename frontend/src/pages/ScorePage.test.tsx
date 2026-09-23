// @vitest-environment jsdom
import { IDBFactory } from 'fake-indexeddb'
import { ScoreQueueProvider } from '../features/scoring/offline/ScoreQueueProvider'
import { queueDatabase } from '../features/scoring/offline/database'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useState } from 'react'
import { api } from '../api/client'
import { handleTournamentLiveSignal } from '../api/liveInvalidation'
import { ApiHttpError } from '../api/http'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { scoringKeys, type ScoringScorecard } from '../api/scorecards'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { ScoreResumeProvider } from '../features/scoring/ScoreResumeProvider'
import { useScoreResume } from '../features/scoring/scoreResumeContext'
import { ScoringGuardProvider } from '../features/scoring/ScoringGuardProvider'
import { tournament, round as draft, session, completion } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { ScorePage } from './ScorePage'

vi.mock('../features/live/useTournamentLive', () => ({ useTournamentLive: vi.fn(() => false) }))
const round = { ...draft, status: 'open' as const }
const owner = { type: 'player' as const, id: session.player_id ?? '' }
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
function card(scored: number[] = []): ScoringScorecard {
  return { projection: 'scoring', round_id: round.id, owner, number_of_holes: 18,
    gross_total: scored.length * 4, net_total: scored.length * 4, playing_handicap: 0,
    holes_scored: scored.length, complete: scored.length === 18, confirmed: false, confirmed_at: null, confirmed_by: null,
    holes: Array.from({ length: 18 }, (_, index) => ({ hole_id: `00000000-0000-0000-0001-${String(index + 1).padStart(12, '0')}`, hole_number: index + 1, par: 4, stroke_index: index + 1, handicap_strokes: 0,
      net_strokes: scored.includes(index + 1) ? 4 : null, score: scored.includes(index + 1) ? {
        id: `00000000-0000-0000-0002-${String(index + 1).padStart(12, '0')}`, round_id: round.id, hole_id: `00000000-0000-0000-0001-${String(index + 1).padStart(12, '0')}`, owner, gross_strokes: 4,
        revision: '1', submitted_by: session.user_id, submitted_at: tournament.created_at, updated_at: tournament.updated_at,
      } : null })) }
}
const explicit = (hole: number, view = 'hole') => `/score?tournament=${tournament.id}&round=${round.id}&owner_type=player&owner=${owner.id}&hole=${hole}&view=${view}`
function mount(path = '/score') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: 20_000 } } })
  const router = createMemoryRouter([{ path: '/score', element: <ScorePage /> }, { path: '/elsewhere', element: <p>Elsewhere</p> }], { initialEntries: [path] })
  render(<QueryClientProvider client={client}><AuthContext value={auth}><ScoreQueueProvider><ScoringGuardProvider><ScoreResumeProvider><RouterProvider router={router} /></ScoreResumeProvider></ScoringGuardProvider></ScoreQueueProvider></AuthContext></QueryClientProvider>)
  return { client, router }
}
beforeEach(() => {
  vi.stubGlobal('indexedDB', new IDBFactory())
  vi.mocked(useTournamentLive).mockReturnValue(false)
  vi.spyOn(api, 'tournaments').mockResolvedValue([tournament])
  vi.spyOn(api, 'rounds').mockResolvedValue([round])
  vi.spyOn(api, 'completionValidation').mockResolvedValue(completion())
  vi.spyOn(api, 'scoreAccess').mockResolvedValue({ round_id: round.id, writable_owners: [owner] })
  vi.spyOn(api, 'scorecardScoring').mockResolvedValue(card())
})
afterEach(() => { cleanup(); vi.restoreAllMocks() })
const expectHole = async (number: number) => { await waitFor(() => expect(screen.getByRole('heading', { name: String(number) })).toBeTruthy()) }

describe('scoring route resume', () => {
  it('retains durable input and allows navigation during disconnected and failed recovery', async () => {
    vi.spyOn(api, 'saveConditionalScore').mockRejectedValue(new Error('Unavailable'))
    const { client, router } = mount(explicit(8))
    await expectHole(8)
    await waitFor(() => expect(screen.getByRole('button', { name: /Registrer par/ }).hasAttribute('disabled')).toBe(false))
    fireEvent.click(screen.getByRole('button', { name: /Registrer par/ }))
    await waitFor(async () => expect(await queueDatabase.list(session.user_id)).toHaveLength(1))
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'error'))
    await expectHole(8)
    expect(screen.queryByText('Spiller med et langt navn')).toBeNull()
    expect(screen.getByRole('button', { name: 'Legg til ett slag' }).hasAttribute('disabled')).toBe(false)
    fireEvent.click(screen.getByRole('button', { name: 'Neste' }))
    await expectHole(9)
    vi.mocked(api.completionValidation).mockRejectedValue(new Error('Progress unavailable'))
    vi.mocked(api.scoreAccess).mockRejectedValue(new ApiHttpError(503, 'unavailable', 'Access unavailable'))
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'open'))
    await screen.findByText('Noe kunne ikke oppdateres. Viste data beholdes.')
    await act(() => router.navigate('/elsewhere'))
    expect(router.state.location.pathname).toBe('/elsewhere')
    expect(await queueDatabase.list(session.user_id)).toHaveLength(1)
  })
  it('keeps immutable submitted and successor intent through reconnect', async () => {
    let release: () => void = () => undefined
    const pending = new Promise<void>(resolve => { release = resolve })
    const saved = card([8]).holes[7]?.score
    if (!saved) throw new Error('Missing saved score')
    let stored = 0
    const save = vi.spyOn(api, 'saveConditionalScore').mockImplementation(async (_round, input) => {
      if (input.gross_strokes === 4) await pending
      stored = input.gross_strokes
      return { request_id: input.request_id, applied_score: { score_id: saved.id, revision: String(stored) } }
    })
    vi.mocked(api.scorecardScoring).mockImplementation(async () => {
      const latest = card(stored ? [8] : []); const hole = latest.holes[7]
      if (hole?.score) { hole.score.gross_strokes = stored; hole.net_strokes = stored }
      latest.gross_total = stored; latest.net_total = stored
      return latest
    })
    const { client } = mount(explicit(8))
    await expectHole(8)
    await waitFor(() => expect(screen.getByRole('button', { name: /Registrer par/ }).hasAttribute('disabled')).toBe(false))
    fireEvent.click(screen.getByRole('button', { name: /Registrer par/ }))
    await waitFor(() => expect(save).toHaveBeenCalledOnce())
    fireEvent.click(screen.getByRole('button', { name: 'Legg til ett slag' }))
    await waitFor(async () => expect((await queueDatabase.list(session.user_id))[0]?.desired).toBe(5))
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'error'))
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'open'))
    await act(async () => { release() })
    await waitFor(() => expect(save).toHaveBeenCalledTimes(2), { timeout: 5000 })
    await screen.findByText('Lagret på serveren')
    expect(save.mock.calls.map(call => call[1].gross_strokes)).toEqual([4, 5])
    expect(save.mock.calls[1]?.[1].expected_score).toEqual({ type: 'present', score_id: saved.id, revision: '4' })
  })
  it('keeps pending confirmation guarded through completion clearing', async () => {
    const complete = card(Array.from({ length: 18 }, (_, i) => i + 1))
    vi.mocked(api.scorecardScoring).mockResolvedValue(complete)
    let resolve: (value: ScoringScorecard) => void = () => undefined
    const confirm = vi.spyOn(api, 'confirmScorecard').mockImplementation(() => new Promise(done => { resolve = done }))
    const { client, router } = mount(explicit(8, 'summary'))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Bekreft fullført scorekort' }).hasAttribute('disabled')).toBe(false))
    fireEvent.click(screen.getByRole('button', { name: 'Bekreft fullført scorekort' }))
    await screen.findByRole('button', { name: 'Bekrefter …' })
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'error'))
    expect(screen.getByRole('button', { name: 'Bekrefter …' }).hasAttribute('disabled')).toBe(true)
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'open'))
    expect(screen.getByRole('button', { name: 'Bekrefter …' }).hasAttribute('disabled')).toBe(true)
    await act(() => router.navigate('/elsewhere'))
    expect(router.state.location.pathname).toBe('/score')
    await act(async () => { resolve(complete) })
    expect(confirm).toHaveBeenCalledOnce()
  })
  it('disables a failed confirmation retry while recovering', async () => {
    vi.mocked(api.scorecardScoring).mockResolvedValue(card(Array.from({ length: 18 }, (_, i) => i + 1)))
    const confirm = vi.spyOn(api, 'confirmScorecard').mockRejectedValue(new Error('Confirmation unavailable'))
    const { client } = mount(explicit(8, 'summary'))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Bekreft fullført scorekort' }).hasAttribute('disabled')).toBe(false))
    fireEvent.click(screen.getByRole('button', { name: 'Bekreft fullført scorekort' }))
    await screen.findByText('Confirmation unavailable')
    vi.mocked(useTournamentLive).mockReturnValue(true)
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'error'))
    const retry = screen.getByRole('button', { name: 'Prøv bekreftelse igjen' })
    await waitFor(() => expect(retry.hasAttribute('disabled')).toBe(true))
    fireEvent.click(retry)
    expect(confirm).toHaveBeenCalledOnce()
    vi.mocked(useTournamentLive).mockReturnValue(false)
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'open'))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Prøv bekreftelse igjen' }).hasAttribute('disabled')).toBe(false))
  })
  it('does not keep a writable recovery shell after completion authorization is denied', async () => {
    const { client } = mount(explicit(8))
    await expectHole(8)
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'error'))
    vi.mocked(api.completionValidation).mockRejectedValue(new ApiHttpError(403, 'forbidden', 'No access'))
    await act(() => handleTournamentLiveSignal(client, session.user_id, 'open'))
    await screen.findByText('No access')
    expect(screen.queryByRole('heading', { name: '8' })).toBeNull()
  })
  it('starts at the first gap and does not jump on background refresh', async () => {
    vi.mocked(api.scorecardScoring).mockResolvedValue(card([1, 3]))
    const { client } = mount()
    await expectHole(2)
    vi.mocked(api.scorecardScoring).mockResolvedValue(card([1, 2, 3]))
    await act(() => client.invalidateQueries({ queryKey: scoringKeys.scoring(session.user_id, round.id, owner) }))
    await expectHole(2)
  })
  it.each(['/score', `/score?tournament=${tournament.id}&round=${round.id}&resume=1`])('waits for authoritative data on %s even with a fresh cached card; retry never resumes from stale data', async (target) => {
    const { router } = mount(explicit(8))
    await expectHole(8)
    await act(() => router.navigate('/elsewhere'))
    vi.mocked(api.scorecardScoring).mockRejectedValue(new Error('Read unavailable'))
    await act(() => router.navigate(target))
    await screen.findByText('Read unavailable')
    expect(router.state.location.search).toBe(target.slice('/score'.length))
    vi.mocked(api.scorecardScoring).mockResolvedValue(card([1, 3]))
    fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
    await expectHole(2)
    expect(router.state.location.search).toContain(`owner=${owner.id}`)
  })
  it('preserves explicit holes, summary links, and Back/Forward', async () => {
    const { router } = mount(explicit(8))
    await expectHole(8)
    await act(() => router.navigate(explicit(4, 'summary')))
    await screen.findByRole('button', { name: 'Ett hull' })
    expect(router.state.location.search).toContain('view=summary')
    await act(() => router.navigate(-1))
    await expectHole(8)
    await act(() => router.navigate(1))
    expect(router.state.location.search).toContain('view=summary')
  })
  it('retains a selected team and earlier round instead of the latest open round', async () => {
    const team = { type: 'team' as const, id: owner.id }
    vi.mocked(api.rounds).mockResolvedValue([{ ...round, scoring_format: 'team_scramble' }, { ...round, id: 'later', round_number: 2 }])
    vi.mocked(api.completionValidation).mockResolvedValue({ ...completion(), owners: [{ ...completion().owners[0],
      owner: team, owner_name: 'Team', holes_scored: 0, required_holes: 18, complete: false, confirmed: false }] })
    vi.mocked(api.scoreAccess).mockResolvedValue({ round_id: round.id, writable_owners: [team] })
    vi.mocked(api.scorecardScoring).mockResolvedValue({ ...card(), owner: team })
    const { router } = mount(explicit(8).replace('owner_type=player', 'owner_type=team'))
    await expectHole(8)
    await act(() => router.navigate('/elsewhere'))
    await act(() => router.navigate('/score'))
    await expectHole(1)
    expect(router.state.location.search).toContain(`round=${round.id}`)
    expect(router.state.location.search).toContain('owner_type=team')
  })
  it('keeps restricted locked cards within returned holes without inferring completion', async () => {
    vi.mocked(api.rounds).mockResolvedValue([{ ...round, status: 'locked' }])
    vi.mocked(api.completionValidation).mockResolvedValue(completion('locked'))
    vi.mocked(api.scoreAccess).mockResolvedValue({ round_id: round.id, writable_owners: [] })
    vi.spyOn(api, 'scorecardRead').mockResolvedValue({ ...card(), projection: 'read', holes: card().holes.slice(0, 9),
      visible_hole_count: 9, complete: null, confirmed: null, visibility: { mode: 'front_nine' } })
    const { router } = mount(explicit(18))
    await expectHole(9)
    await screen.findByText('Hull 10–18 er skjult til administratoren frigir finalens bakni.')
    await act(() => router.navigate('/elsewhere'))
    await act(() => router.navigate('/score'))
    await expectHole(1)
    expect(router.state.location.search).toContain('view=hole')
    expect(api.scorecardScoring).not.toHaveBeenCalled()
  })
  it('opens summary when all holes are registered', async () => {
    vi.mocked(api.scorecardScoring).mockResolvedValue(card(Array.from({ length: 18 }, (_, i) => i + 1)))
    const { router } = mount()
    await waitFor(() => expect(router.state.location.search).toContain('view=summary'))
    expect(await screen.findByRole('button', { name: 'Bekreft fullført scorekort' })).toBeTruthy()
  })
  it('starts at one and guards input that fails device persistence until discarded', async () => {
    vi.spyOn(queueDatabase, 'enqueue').mockRejectedValue(new Error('Device storage unavailable'))
    const { router } = mount()
    await expectHole(1)
    fireEvent.click(screen.getByRole('button', { name: /Registrer par/ }))
    await screen.findByText('Device storage unavailable')
    await act(() => router.navigate('/elsewhere'))
    expect(router.state.location.pathname).toBe('/score')
    fireEvent.click(screen.getByRole('button', { name: /Forkast/ }))
    await act(() => router.navigate('/elsewhere'))
    await act(() => router.navigate('/score'))
    await expectHole(1)
  })
})

function ReceiptProbe() {
  const { selection, remember } = useScoreResume()
  const [receipt, setReceipt] = useState('')
  return <><button onClick={() => { setReceipt('one-time receipt'); remember({ tournamentId: tournament.id, roundId: round.id, owner }) }}>Remember</button>
    <p>{receipt}</p><output>{selection?.roundId ?? 'empty'}</output></>
}
it('clears identity selections synchronously without remounting onboarding receipts; same-user refresh retains selection', () => {
  const tree = (value: AuthContextValue) => <AuthContext value={value}><ScoreResumeProvider><ReceiptProbe /></ScoreResumeProvider></AuthContext>
  const view = render(tree(auth))
  fireEvent.click(screen.getByText('Remember'))
  view.rerender(tree({ ...auth, session: { ...session, display_name: 'Refreshed' } }))
  expect(screen.getByRole('status').textContent).toBe(round.id)
  view.rerender(tree({ ...auth, session: null }))
  expect(screen.getByRole('status').textContent).toBe('empty')
  expect(screen.getByText('one-time receipt')).toBeTruthy()
  view.rerender(tree({ ...auth, session: { ...session, user_id: 'another-user' } }))
  fireEvent.click(screen.getByText('Remember'))
  view.rerender(tree(auth))
  expect(screen.getByRole('status').textContent).toBe('empty')
})


describe('H1 nondurable recovery', () => {
  it.each(['lock', '403', 'metadata', 'access-revoked'] as const)('retains local 5 after %s replaces the scorer', async transition => {
    vi.mocked(api.scorecardScoring).mockResolvedValue(card([1]))
    vi.spyOn(queueDatabase, 'enqueue').mockRejectedValue(new Error('Device storage unavailable'))
    const { client, router } = mount(explicit(1))
    await expectHole(1)
    fireEvent.click(screen.getByRole('button', { name: 'Legg til ett slag' }))
    await screen.findByText('Device storage unavailable')
    if (transition === 'lock') {
      vi.mocked(api.rounds).mockResolvedValue([{ ...round, status: 'locked' }])
      vi.mocked(api.completionValidation).mockResolvedValue(completion('locked'))
      vi.spyOn(api, 'scorecardRead').mockRejectedValue(new ApiHttpError(403, 'forbidden', 'Denied'))
    } else if (transition === 'access-revoked') {
      vi.mocked(api.scoreAccess).mockResolvedValue({ round_id: round.id, writable_owners: [] })
      vi.spyOn(api, 'scorecardRead').mockResolvedValue({ ...card([1]), projection: 'read', visible_hole_count: 18, visibility: { mode: 'full' } })
    } else if (transition === '403') {
      vi.mocked(api.scorecardScoring).mockRejectedValue(new ApiHttpError(403, 'forbidden', 'Denied'))
      vi.mocked(api.scoreAccess).mockRejectedValue(new ApiHttpError(403, 'forbidden', 'Denied'))
    } else {
      vi.mocked(api.completionValidation).mockRejectedValue(new Error('Metadata unavailable'))
      await act(() => handleTournamentLiveSignal(client, session.user_id, 'error'))
    }
    await act(async () => { await client.invalidateQueries() })
    const recovery = await screen.findByRole('region', { name: 'Ulagrede scoreendringer' })
    expect(recovery.textContent).toContain('5 slag')
    expect(screen.queryByText('Spiller med et langt navn')).toBeNull()
    await act(() => router.navigate('/elsewhere'))
    expect(router.state.location.pathname).toBe('/score')
    fireEvent.click(screen.getByRole('button', { name: 'Forkast ulagret endring' }))
    await act(() => router.navigate('/elsewhere'))
    expect(router.state.location.pathname).toBe('/elsewhere')
  })
})
