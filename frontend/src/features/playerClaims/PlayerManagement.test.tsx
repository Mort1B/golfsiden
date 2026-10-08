// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { beforeEach, afterEach, expect, it, vi } from 'vitest'
import { authKeys } from '../../api/auth'
import { ApiHttpError } from '../../api/http'
import { playerClaimsApi, type ClaimReceipt } from '../../api/playerClaims'
import type { TournamentPlayerRoster } from '../../api/types'
import { AuthContext } from '../auth/authContext'
import { tournament, session } from '../tournaments/lifecycle/__tests__/fixtures'
import { PlayerManagement } from './PlayerManagement'
const playerId = '00000000-0000-0000-0000-000000000012'
const playerName = 'Spilleren med et meget langt navn og langt etternavn'
const receipt: ClaimReceipt = { player_id: playerId, claim_id: '00000000-0000-0000-0000-000000000013', expires_at: '2099-01-01T12:00:00Z', token: 'a'.repeat(43) }
const account = { player_id: playerId, has_account: false, claim_id: receipt.claim_id, claim_expires_at: receipt.expires_at, claim_revoked_at: null, claimed_at: null }
const roster: TournamentPlayerRoster = { players: [{ player_id: playerId, tournament_id: tournament.id, display_name: playerName, status: 'active', tournament_handicap: 12, player_active: true, seed: null, created_at: tournament.created_at, updated_at: tournament.updated_at }], handicap_correction: { state: 'editable' } }
let client: QueryClient
function tree(data = roster, closed = false) {
  return render(<QueryClientProvider client={client}><AuthContext value={{ session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), retry: vi.fn(), establishSession: vi.fn() }}><MemoryRouter><PlayerManagement tournament={{ ...tournament, status: closed ? 'completed' : 'active' }} roster={{ data, pending: false, error: null, retry: vi.fn() }} authorityRefreshing={false} /></MemoryRouter></AuthContext></QueryClientProvider>)
}
async function create() {
  await waitFor(() => expect(screen.getByRole('button', { name: 'Opprett spiller og kontolenke' })).toHaveProperty('disabled', false))
  fireEvent.change(screen.getByLabelText('Spillerens navn'), { target: { value: playerName } })
  fireEvent.change(screen.getByLabelText('Spillerens handicap'), { target: { value: '14,4' } })
  fireEvent.click(screen.getByRole('button', { name: 'Opprett spiller og kontolenke' }))
}
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } }); client.setQueryData(authKeys.session, session)
  vi.spyOn(playerClaimsApi, 'accounts').mockResolvedValue([account])
  vi.spyOn(playerClaimsApi, 'create').mockResolvedValue(receipt)
  vi.spyOn(playerClaimsApi, 'reissue').mockResolvedValue(receipt)
  vi.spyOn(playerClaimsApi, 'withdraw').mockResolvedValue()
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })
it('creates with Norwegian handicap, shows ephemeral receipt and manual copy fallback', async () => {
  tree(); await create()
  await screen.findByLabelText('Personlig kontolenke')
  expect(playerClaimsApi.create).toHaveBeenCalledWith(tournament.id, playerName, 14.4, session.csrf_token)
  fireEvent.click(screen.getByRole('button', { name: 'Kopier lenke' }))
  await screen.findByText(/Kunne ikke kopiere/)
  fireEvent.click(screen.getByRole('button', { name: 'Skjul lenken' }))
  expect(screen.queryByLabelText('Personlig kontolenke')).toBeNull()
  await waitFor(() => expect(client.getMutationCache().getAll()).toHaveLength(0))
  expect(JSON.stringify(client.getQueryCache().getAll().map(q => q.state.data))).not.toContain(receipt.token)
})
it('prevents blind creation retry when delivery is unknown', async () => {
  vi.mocked(playerClaimsApi.create).mockRejectedValue(new TypeError('Failed to fetch'))
  tree(); await create(); await screen.findByText(/Spilleren kan være opprettet/)
  expect(screen.getByRole('button', { name: 'Opprett spiller og kontolenke' })).toHaveProperty('disabled', true)
  fireEvent.click(screen.getByRole('button', { name: 'Jeg har kontrollert listen' }))
  expect(screen.getByRole('button', { name: 'Opprett spiller og kontolenke' })).toHaveProperty('disabled', false)
})
it('confirms withdrawal and links to actionable draft and live-round remedies', async () => {
  vi.mocked(playerClaimsApi.withdraw).mockRejectedValue(new ApiHttpError(409, 'player_assigned_draft', 'internal'))
  tree(); await waitFor(() => expect(screen.getByRole('button', { name: `Fjern ${playerName} fra turneringen` })).toHaveProperty('disabled', false))
  fireEvent.click(screen.getByRole('button', { name: `Fjern ${playerName} fra turneringen` }))
  expect(screen.getByText(/Eksisterende tilgang og roller beholdes/)).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Bekreft fjerning' }))
  await screen.findByRole('link', { name: 'Åpne spillegrupper, lag og matcher' })
  vi.mocked(playerClaimsApi.withdraw).mockRejectedValue(new ApiHttpError(409, 'player_round_in_progress', 'internal'))
  await waitFor(() => expect(screen.getByRole('button', { name: 'Bekreft fjerning' })).toHaveProperty('disabled', false))
  fireEvent.click(screen.getByRole('button', { name: 'Bekreft fjerning' }))
  await screen.findByRole('link', { name: 'Åpne rundestyring' })
})
it.each(['csrf', 'logout', 'unmount'])('ignores held create completion after %s', async mode => {
  let finish: (value: ClaimReceipt) => void = () => undefined
  vi.mocked(playerClaimsApi.create).mockImplementation(() => new Promise(resolve => { finish = resolve }))
  const view = tree(); await create(); await screen.findByText('Lagrer og oppdaterer spillerlisten …')
  const invalidate = vi.spyOn(client, 'invalidateQueries')
  if (mode === 'unmount') view.unmount()
  else client.setQueryData(authKeys.session, mode === 'logout' ? null : { ...session, csrf_token: 'new-session' })
  await act(async () => { finish(receipt) })
  expect(screen.queryByLabelText('Personlig kontolenke')).toBeNull()
  expect(invalidate).not.toHaveBeenCalled()
})
it('keeps claim actions available in closed tournaments and hides them for withdrawn/claimed players', async () => {
  const view = tree(roster, true)
  await screen.findByText(playerName)
  await waitFor(() => expect(screen.getByRole('button', { name: `Lag ny kontolenke for ${playerName}` })).toHaveProperty('disabled', false))
  expect(screen.getByRole('button', { name: 'Opprett spiller og kontolenke' })).toHaveProperty('disabled', true)
  expect(screen.getByRole('button', { name: `Fjern ${playerName} fra turneringen` })).toHaveProperty('disabled', true)
  view.unmount(); tree({ ...roster, players: roster.players.map(p => ({ ...p, status: 'withdrawn' })) })
  expect(screen.queryByRole('button', { name: /Lag ny kontolenke/ })).toBeNull()
})
it('shows empty and metadata failure states with retry', async () => {
  vi.mocked(playerClaimsApi.accounts).mockRejectedValue(new Error('private'))
  tree({ ...roster, players: [] })
  await screen.findByText('Kunne ikke laste kontostatus.')
  expect(screen.getByText('Ingen deltakere er registrert.')).toBeTruthy()
  expect(screen.getByRole('button', { name: 'Opprett spiller og kontolenke' })).toHaveProperty('disabled', true)
})
