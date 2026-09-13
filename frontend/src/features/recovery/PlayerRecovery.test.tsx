// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys } from '../../api/auth'
import { ApiHttpError } from '../../api/http'
import { passwordRecoveryApi, type RecoveryReceipt } from '../../api/passwordRecovery'
import { session } from '../tournaments/lifecycle/__tests__/fixtures'
import { PlayerRecovery } from './PlayerRecovery'
let client: QueryClient
const receipt: RecoveryReceipt = { id: '00000000-0000-0000-0000-000000000011', expires_at: '2099-01-01T12:00:00Z', reset_url: 'https://golf.example/reset-password/00000000-0000-0000-0000-000000000011#token=' + 'a'.repeat(43) }
function tree() { return <QueryClientProvider client={client}><PlayerRecovery tournamentId="tour" playerId="player" playerName="Et langt spillernavn" session={session} /></QueryClientProvider> }
function open() { fireEvent.click(screen.getByRole('button', { name: /Hjelp med glemt passord/ })) }
function password() { fireEvent.change(screen.getByLabelText('Ditt nåværende passord'), { target: { value: 'current password' } }) }
beforeEach(() => { client = new QueryClient(); client.setQueryData(authKeys.session, session); vi.spyOn(passwordRecoveryApi, 'issue').mockResolvedValue(receipt); vi.spyOn(passwordRecoveryApi, 'revoke').mockResolvedValue() })
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('creates, handles clipboard failure, hides and revokes a lost receipt using fresh password confirmation', async () => {
  vi.stubGlobal('navigator', { clipboard: { writeText: vi.fn().mockRejectedValue(new Error('denied')) } })
  render(tree()); open(); password()
  fireEvent.click(screen.getByRole('button', { name: 'Lag lenke for nytt passord' }))
  await screen.findByLabelText('Lenke for nytt passord')
  expect(screen.getByLabelText('Ditt nåværende passord')).toHaveProperty('value', '')
  fireEvent.click(screen.getByRole('button', { name: 'Kopier lenke' }))
  await screen.findByText(/Kunne ikke kopiere/)
  fireEvent.click(screen.getByRole('button', { name: 'Skjul lenken' }))
  expect(screen.queryByLabelText('Lenke for nytt passord')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: /Lukk passordhjelp/ })); open(); password()
  fireEvent.click(screen.getByRole('button', { name: 'Tilbakekall lenker' }))
  await screen.findByText(/Utestående lenker/)
  expect(passwordRecoveryApi.revoke).toHaveBeenCalledWith('tour', 'player', 'current password', session.csrf_token)
  await waitFor(() => expect(client.getMutationCache().getAll()).toHaveLength(0))
})
it('shows wrong-password and generic denied states without server secrets or account metadata', async () => {
  vi.mocked(passwordRecoveryApi.issue).mockRejectedValueOnce(new ApiHttpError(409, 'current_password_incorrect', 'sensitive'))
    .mockRejectedValueOnce(new ApiHttpError(403, 'forbidden', 'other tournament'))
  render(tree()); open(); password(); fireEvent.click(screen.getByRole('button', { name: 'Lag lenke for nytt passord' }))
  await screen.findByText(/passord er ikke riktig/)
  password(); fireEvent.click(screen.getByRole('button', { name: 'Lag lenke for nytt passord' }))
  await screen.findByText(/nettstedsoperatøren/)
  expect(screen.queryByText('other tournament')).toBeNull()
})
it.each(['close', 'identity'])('discards an issuance finishing after %s and removes sensitive mutation cache', async (transition) => {
  let finish: (receipt: RecoveryReceipt) => void = () => { throw new Error('missing resolver') }
  vi.mocked(passwordRecoveryApi.issue).mockImplementation(() => new Promise(resolve => { finish = resolve }))
  render(tree()); open(); password(); fireEvent.click(screen.getByRole('button', { name: 'Lag lenke for nytt passord' }))
  await screen.findByText('Bekrefter handlingen …')
  if (transition === 'close') { fireEvent.click(screen.getByRole('button', { name: /Lukk passordhjelp/ })); open() }
  else client.setQueryData(authKeys.session, { ...session, csrf_token: 'new-session' })
  await act(async () => { finish(receipt) })
  expect(screen.queryByLabelText('Lenke for nytt passord')).toBeNull()
  await waitFor(() => expect(client.getMutationCache().getAll()).toHaveLength(0))
})
