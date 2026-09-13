// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys } from '../../api/auth'
import { resultSharingApi, type ResultShareStatus } from '../../api/resultSharing'
import { ApiHttpError } from '../../api/http'
import { shareGrant, shareId, shareSecret } from '../../api/resultSharing/__tests__/fixtures'
import { session, tournament } from '../tournaments/lifecycle/__tests__/fixtures'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { ResultShareControl } from './ResultShareControl'
let client: QueryClient; let status: ResultShareStatus; let auth: AuthContextValue
function tree(refreshing = false) { return <QueryClientProvider client={client}><AuthContext value={auth}><ResultShareControl tournamentId={tournament.id} authorityRefreshing={refreshing} /></AuthContext></QueryClientProvider> }
async function create() { fireEvent.click(await screen.findByRole('button', { name: 'Opprett offentlig resultatlenke' })) }
beforeEach(() => {
  client = new QueryClient(); client.setQueryData(authKeys.session, session)
  auth = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
  status = { tournament_id: tournament.id, grant: null }
  vi.spyOn(resultSharingApi, 'status').mockImplementation(async () => status)
  vi.spyOn(resultSharingApi, 'issue').mockImplementation(async () => { status = { ...status, grant: shareGrant }; return { tournament_id: tournament.id, grant: shareGrant, token: shareSecret } })
  vi.spyOn(resultSharingApi, 'revoke').mockImplementation(async () => { status = { ...status, grant: { ...shareGrant, revoked_at: shareGrant.created_at } } })
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('explains disclosure before creation, handles copy fallback, hides receipt and revokes by metadata', async () => {
  vi.stubGlobal('navigator', { clipboard: { writeText: vi.fn().mockRejectedValue(new Error('denied')) } })
  render(tree()); await screen.findByText('Ingen resultatlenke er opprettet.')
  expect(screen.getByText(/Alle med lenken/)).toBeTruthy(); expect(resultSharingApi.issue).not.toHaveBeenCalled()
  await create(); await screen.findByLabelText('Offentlig resultatlenke')
  expect(resultSharingApi.issue).toHaveBeenCalledWith(tournament.id, null, session.csrf_token)
  fireEvent.click(screen.getByRole('button', { name: 'Kopier resultatlenke' })); await screen.findByText(/Kunne ikke kopiere/)
  fireEvent.click(screen.getByRole('button', { name: 'Skjul resultatlenken' }))
  expect(screen.queryByLabelText('Offentlig resultatlenke')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: 'Tilbakekall resultatlenken' }))
  await screen.findByText('Resultatlenken er tilbakekalt.')
  expect(resultSharingApi.revoke).toHaveBeenCalledWith(tournament.id, shareId, session.csrf_token)
  await waitFor(() => expect(client.getMutationCache().getAll()).toHaveLength(0))
  expect(JSON.stringify(client.getQueryCache().getAll().map(query => query.state.data))).not.toContain(shareSecret)
})
it('disables cached controls while authority refreshes and retains no existing secret', async () => {
  status.grant = shareGrant
  const view = render(tree()); const button = await screen.findByRole('button', { name: 'Erstatt resultatlenken' })
  view.rerender(tree(true)); expect(button.hasAttribute('disabled')).toBe(true)
  expect(screen.queryByLabelText('Offentlig resultatlenke')).toBeNull()
})
it('refreshes stale replacement failures and discards earlier receipt', async () => {
  render(tree()); await create(); await screen.findByLabelText('Offentlig resultatlenke')
  vi.mocked(resultSharingApi.issue).mockRejectedValue(new ApiHttpError(409, 'result_share_stale', shareSecret))
  fireEvent.click(screen.getByRole('button', { name: 'Erstatt resultatlenken' }))
  await screen.findByText(/Lenken ble endret et annet sted/)
  expect(screen.queryByLabelText('Offentlig resultatlenke')).toBeNull()
  expect(screen.queryByText(shareSecret)).toBeNull()
})
it.each(['unmount', 'session'])('ignores deferred issuance after %s and prevents duplicate requests', async transition => {
  let finish: (() => void) | undefined
  vi.mocked(resultSharingApi.issue).mockImplementation(() => new Promise(resolve => { finish = () => resolve({ tournament_id: tournament.id, grant: shareGrant, token: shareSecret }) }))
  const view = render(tree()); const button = await screen.findByRole('button', { name: 'Opprett offentlig resultatlenke' })
  fireEvent.click(button); fireEvent.click(button)
  await waitFor(() => expect(resultSharingApi.issue).toHaveBeenCalledTimes(1))
  if (transition === 'unmount') { view.unmount(); client.clear() }
  else { const replacement = { ...session, csrf_token: 'next' }; client.setQueryData(authKeys.session, replacement); auth = { ...auth, session: replacement }; view.rerender(tree()) }
  await act(async () => finish?.())
  expect(screen.queryByLabelText('Offentlig resultatlenke')).toBeNull()
  await waitFor(() => expect(client.getMutationCache().getAll()).toHaveLength(0))
})
it('shows metadata loading/error retry and expired state', async () => {
  vi.mocked(resultSharingApi.status).mockRejectedValue(new Error('private'))
  render(tree()); await screen.findByText('Kunne ikke hente status for resultatdeling.')
  status.grant = { ...shareGrant, expires_at: '2026-01-01T00:00:00Z' }
  vi.mocked(resultSharingApi.status).mockResolvedValue(status)
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater delingsstatus' }))
  await screen.findByText('Den siste resultatlenken er utløpt.')
  expect(screen.queryByRole('button', { name: 'Tilbakekall resultatlenken' })).toBeNull()
})
