// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useState } from 'react'
import { api } from '../api/client'
import { scoringKeys, type ScoringScorecard } from '../api/scorecards'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { ScoreResumeProvider } from '../features/scoring/ScoreResumeProvider'
import { useScoreResume } from '../features/scoring/scoreResumeContext'
import { ScoringGuardProvider } from '../features/scoring/ScoringGuardProvider'
import { tournament, round as draft, session, completion } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { ScorePage } from './ScorePage'

vi.mock('../features/live/useTournamentLive', () => ({ useTournamentLive: () => undefined }))
const round = { ...draft, status: 'open' as const }
const owner = { type: 'player' as const, id: session.player_id ?? '' }
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
function card(scored: number[] = []): ScoringScorecard {
  return { projection: 'scoring', round_id: round.id, owner, number_of_holes: 18,
    gross_total: scored.length * 4, net_total: scored.length * 4, playing_handicap: 0,
    holes_scored: scored.length, complete: scored.length === 18, confirmed: false, confirmed_at: null, confirmed_by: null,
    holes: Array.from({ length: 18 }, (_, index) => ({ hole_id: `hole-${index + 1}`, hole_number: index + 1, par: 4, stroke_index: index + 1,
      net_strokes: scored.includes(index + 1) ? 4 : null, score: scored.includes(index + 1) ? {
        id: `score-${index}`, round_id: round.id, hole_id: `hole-${index + 1}`, owner, gross_strokes: 4,
        submitted_by: session.user_id, submitted_at: tournament.created_at, updated_at: tournament.updated_at,
      } : null })) }
}
const explicit = (hole: number, view = 'hole') => `/score?tournament=${tournament.id}&round=${round.id}&owner_type=player&owner=${owner.id}&hole=${hole}&view=${view}`
function mount(path = '/score') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: 20_000 } } })
  const router = createMemoryRouter([{ path: '/score', element: <ScorePage /> }, { path: '/elsewhere', element: <p>Elsewhere</p> }], { initialEntries: [path] })
  render(<QueryClientProvider client={client}><AuthContext value={auth}><ScoringGuardProvider><ScoreResumeProvider><RouterProvider router={router} /></ScoreResumeProvider></ScoringGuardProvider></AuthContext></QueryClientProvider>)
  return { client, router }
}
beforeEach(() => {
  vi.spyOn(api, 'tournaments').mockResolvedValue([tournament])
  vi.spyOn(api, 'rounds').mockResolvedValue([round])
  vi.spyOn(api, 'completionValidation').mockResolvedValue(completion())
  vi.spyOn(api, 'scoreAccess').mockResolvedValue({ round_id: round.id, writable_owners: [owner] })
  vi.spyOn(api, 'scorecardScoring').mockResolvedValue(card())
})
afterEach(() => { cleanup(); vi.restoreAllMocks() })
const expectHole = async (number: number) => { await waitFor(() => expect(screen.getByRole('heading', { name: String(number) })).toBeTruthy()) }

describe('scoring route resume', () => {
  it('starts at the first gap and does not jump on background refresh', async () => {
    vi.mocked(api.scorecardScoring).mockResolvedValue(card([1, 3]))
    const { client } = mount()
    await expectHole(2)
    vi.mocked(api.scorecardScoring).mockResolvedValue(card([1, 2, 3]))
    await act(() => client.invalidateQueries({ queryKey: scoringKeys.scoring(session.user_id, round.id, owner) }))
    await expectHole(2)
  })
  it('waits for authoritative data on return even with a fresh cached card; retry never resumes from stale data', async () => {
    const { router } = mount(explicit(8))
    await expectHole(8)
    await act(() => router.navigate('/elsewhere'))
    vi.mocked(api.scorecardScoring).mockRejectedValue(new Error('Read unavailable'))
    await act(() => router.navigate('/score'))
    await screen.findByText('Read unavailable')
    expect(router.state.location.search).toBe('')
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
  it('starts at one on an empty card and keeps a failed input guarded until discarded', async () => {
    vi.spyOn(api, 'saveScore').mockRejectedValue(new Error('Save unavailable'))
    const { router } = mount()
    await expectHole(1)
    fireEvent.click(screen.getByRole('button', { name: /Registrer par/ }))
    await screen.findByText('Save unavailable')
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
