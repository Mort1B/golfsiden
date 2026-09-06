// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '../../../../api/client'
import { ApiHttpError } from '../../../../api/http'
import { roundLifecycleApi } from '../../../../api/roundLifecycle'
import { tournamentKeys } from '../../../../api/tournaments'
import type { Round } from '../../../../api/types'
import { AuthContext, type AuthContextValue } from '../../../auth/authContext'
import { RoundLifecyclePanel } from '../RoundLifecyclePanel'
import { completion, opening, round, session, tournament } from './fixtures'

let client: QueryClient
let serverRound: Round
let progress = completion()
const auth: AuthContextValue = {
  session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn(),
}

function Harness({ selectedId = round.id, refreshing = false }: { selectedId?: string; refreshing?: boolean }) {
  const rounds = useQuery({ queryKey: tournamentKeys.rounds(session.user_id, tournament.id), queryFn: async () => [serverRound] })
  return <RoundLifecyclePanel tournament={tournament} rounds={rounds.data ?? []} selectedRoundId={selectedId} onSelectRound={vi.fn()} authorityRefreshing={refreshing} />
}

function mount(props: { selectedId?: string; refreshing?: boolean } = {}) {
  return render(<QueryClientProvider client={client}><AuthContext value={auth}><MemoryRouter><Harness {...props} /></MemoryRouter></AuthContext></QueryClientProvider>)
}

beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity }, mutations: { retry: false } } })
  serverRound = round
  progress = completion()
  client.setQueryData(tournamentKeys.rounds(session.user_id, tournament.id), [serverRound])
  vi.spyOn(api, 'round').mockImplementation(async () => serverRound)
  vi.spyOn(api, 'completionValidation').mockImplementation(async () => progress)
  vi.spyOn(roundLifecycleApi, 'validation').mockResolvedValue(opening)
  vi.spyOn(roundLifecycleApi, 'transition').mockImplementation(async (_id, _trip, action) => {
    serverRound = { ...serverRound, status: action === 'open' ? 'open' : action === 'complete' ? 'completed' : 'locked' }
    progress = completion(serverRound.status)
    return serverRound
  })
})

afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })

async function enabledButton(name: string) {
  const button = await screen.findByRole('button', { name })
  await waitFor(() => expect(button.hasAttribute('disabled')).toBe(false))
  return button
}

describe('administrator round lifecycle', () => {
  it('explains provisional standings before completion without implying scoring starts then', async () => {
    serverRound = { ...round, status: 'open' }
    client.setQueryData(tournamentKeys.rounds(session.user_id, tournament.id), [serverRound])
    mount()
    fireEvent.click(await enabledButton('Fullfør runden'))
    expect(screen.getByText(/Synlig score fra den åpne runden kan allerede inngå foreløpig/)).toBeTruthy()
    expect(screen.queryByText(/Fullføring gjør at rundens resultater teller/)).toBeNull()
  })
  it('requires explicit confirmation, supports Escape/focus, and opens once', async () => {
    mount()
    const open = await enabledButton('Åpne runden')
    fireEvent.click(open)
    expect(roundLifecycleApi.transition).not.toHaveBeenCalled()
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Avbryt' }))
    fireEvent.keyDown(screen.getByRole('group'), { key: 'Escape' })
    expect(document.activeElement).toBe(open)
    expect(screen.queryByRole('button', { name: 'Avbryt' })).toBeNull()
    fireEvent.click(open)
    const confirm = screen.getByRole('button', { name: 'Bekreft og åpne runden' })
    fireEvent.click(confirm)
    fireEvent.click(confirm)
    await screen.findByText('Runden er åpnet.')
    expect(roundLifecycleApi.transition).toHaveBeenCalledTimes(1)
    expect(roundLifecycleApi.transition).toHaveBeenCalledWith(round.id, tournament.id, 'open', session.csrf_token)
    await enabledButton('Fullfør runden')
    expect(api.completionValidation).toHaveBeenCalledWith(round.id, round.scoring_format)
  })

  it('shows affected entrants and round-preserving setup links', async () => {
    vi.mocked(roundLifecycleApi.validation).mockResolvedValue({ ...opening, ready: false,
      issues: [{ code: 'missing_flight_assignment', message: 'missing' }, { code: 'missing_course', message: 'course' }],
      missing_flight_players: [{ player_id: session.player_id ?? '', display_name: 'Mangler Flight' }],
    })
    mount()
    await screen.findByText('Mangler Flight')
    expect(screen.getByRole('button', { name: 'Åpne runden' }).hasAttribute('disabled')).toBe(true)
    expect(screen.getByRole('link', { name: 'Til spillegrupper' }).getAttribute('href')).toContain(`round=${round.id}#pairings`)
    expect(screen.getByRole('link', { name: 'Til baneoppsett' }).getAttribute('href')).toContain(`round=${round.id}#courses`)
  })

  it.each(['individual_stroke_play', 'team_scramble', 'two_player_foursomes'] as const)('links an unconfirmed %s owner to its exact card', async (format) => {
    serverRound = { ...round, status: 'open', scoring_format: format }
    const owner = { type: format === 'individual_stroke_play' ? 'player' as const : 'team' as const, id: session.player_id ?? '' }
    progress = { ...completion(), ready_to_complete: false,
      owners: [{ owner, owner_name: 'Korrekt eier', holes_scored: 18, required_holes: 18, complete: true, confirmed: false }] }
    client.setQueryData(tournamentKeys.rounds(session.user_id, tournament.id), [serverRound])
    mount()
    const link = await screen.findByRole('link', { name: 'Bekreft scorekort for Korrekt eier' })
    const url = new URL(link.getAttribute('href') ?? '', 'http://localhost')
    expect(url.pathname).toBe('/score')
    expect(url.searchParams.get('owner_type')).toBe(owner.type)
    expect(url.searchParams.get('owner')).toBe(owner.id)
    expect(url.searchParams.get('round')).toBe(round.id)
    expect(screen.getByRole('link', { name: 'Les scorekort for Korrekt eier' }).getAttribute('href')).toContain(`/scorecards/${owner.type}/${owner.id}`)
    expect(screen.getByRole('button', { name: 'Fullfør runden' }).hasAttribute('disabled')).toBe(true)
  })

  it('rechecks correction/confirmation before locking and keeps final visibility independent', async () => {
    serverRound = { ...round, status: 'completed' }
    progress = completion('completed')
    client.setQueryData(tournamentKeys.rounds(session.user_id, tournament.id), [serverRound])
    mount()
    await enabledButton('Lås runden')
    progress = { ...progress, ready_to_lock: false, owners: progress.owners.map((owner) => ({ ...owner, confirmed: false })) }
    fireEvent.click(await enabledButton('Oppdater kontrollen'))
    await screen.findByText(/Mangler bekreftelse/)
    expect(screen.getByRole('button', { name: 'Lås runden' }).hasAttribute('disabled')).toBe(true)
    progress = completion('completed')
    fireEvent.click(await enabledButton('Oppdater kontrollen'))
    fireEvent.click(await enabledButton('Lås runden'))
    fireEvent.click(screen.getByRole('button', { name: 'Bekreft og lås runden' }))
    await screen.findByText('Runden er låst.')
    expect(screen.queryByRole('button', { name: 'Lås runden' })).toBeNull()
    expect(screen.getByText(/Fullføring og låsing endrer ikke synligheten/)).toBeTruthy()
  })

  it('reconciles a lost successful response without offering the old transition again', async () => {
    vi.mocked(roundLifecycleApi.transition).mockImplementation(async () => {
      serverRound = { ...round, status: 'open' }
      throw new TypeError('network lost')
    })
    mount()
    fireEvent.click(await enabledButton('Åpne runden'))
    fireEvent.click(screen.getByRole('button', { name: 'Bekreft og åpne runden' }))
    await screen.findByText(/Svaret kunne ikke bekreftes/)
    await enabledButton('Fullfør runden')
    expect(screen.queryByRole('button', { name: 'Åpne runden' })).toBeNull()
  })

  it('refetches detailed opening blockers after a conflict', async () => {
    vi.mocked(roundLifecycleApi.transition).mockImplementation(async () => {
      vi.mocked(roundLifecycleApi.validation).mockResolvedValue({ ...opening, ready: false,
        issues: [{ code: 'missing_course', message: 'missing' }] })
      throw new ApiHttpError(409, 'conflict', 'not ready')
    })
    mount()
    fireEvent.click(await enabledButton('Åpne runden'))
    fireEvent.click(screen.getByRole('button', { name: 'Bekreft og åpne runden' }))
    await screen.findByText('Velg bane for runden.')
    expect(screen.getByRole('button', { name: 'Åpne runden' }).hasAttribute('disabled')).toBe(true)
  })

  it('disables stale readiness after a read failure and requires a successful retry', async () => {
    mount()
    await enabledButton('Åpne runden')
    vi.mocked(roundLifecycleApi.validation).mockRejectedValue(new Error('read failed'))
    fireEvent.click(await enabledButton('Oppdater kontrollen'))
    await screen.findByText(/Oppdateringen mislyktes/)
    expect(screen.getByRole('button', { name: 'Åpne runden' }).hasAttribute('disabled')).toBe(true)
    vi.mocked(roundLifecycleApi.validation).mockResolvedValue(opening)
    fireEvent.click(await enabledButton('Oppdater kontrollen'))
    await enabledButton('Åpne runden')
  })

  it('does not enable actions while administrator authority refreshes', async () => {
    mount({ refreshing: true })
    await screen.findByText('Runden er klar til å åpnes.')
    expect(screen.getByRole('button', { name: 'Åpne runden' }).hasAttribute('disabled')).toBe(true)
  })

  it('fails closed for a redacted final and for an unknown selected round', async () => {
    serverRound = { ...round, status: 'open' }
    progress = { ...completion(), visibility: { mode: 'front_nine' }, ready_to_complete: null, ready_to_lock: null }
    client.setQueryData(tournamentKeys.rounds(session.user_id, tournament.id), [serverRound])
    const view = mount()
    await screen.findByText(/Rundestatus eller tilgang er endret/)
    expect(screen.getByRole('button', { name: 'Fullfør runden' }).hasAttribute('disabled')).toBe(true)
    expect(screen.queryByText('Spiller med et langt navn')).toBeNull()
    view.unmount()
    mount({ selectedId: 'another-trip-round' })
    await screen.findByText('Den valgte runden finnes ikke i denne turneringen.')
    expect(screen.queryByRole('button', { name: 'Fullfør runden' })).toBeNull()
  })

  it('does not repopulate private caches when an old mutation resolves after unmount', async () => {
    let resolve: ((value: Round) => void) | undefined
    vi.mocked(roundLifecycleApi.transition).mockReturnValue(new Promise<Round>((done) => { resolve = done }))
    const view = mount()
    fireEvent.click(await enabledButton('Åpne runden'))
    fireEvent.click(screen.getByRole('button', { name: 'Bekreft og åpne runden' }))
    await waitFor(() => expect(roundLifecycleApi.transition).toHaveBeenCalledTimes(1))
    view.unmount()
    client.clear()
    await act(async () => { resolve?.({ ...round, status: 'open' }) })
    expect(client.getQueryCache().getAll()).toHaveLength(0)
  })
})
