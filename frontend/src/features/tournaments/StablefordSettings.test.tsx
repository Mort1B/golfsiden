// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { StablefordSettings } from './StablefordSettings'
import { round as draft, session } from './lifecycle/__tests__/fixtures'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { stablefordApi } from '../../api/stableford'
import { ApiHttpError } from '../../api/http'
const round = { ...draft, scoring_format: 'individual_stableford' as const }
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
function mount(status: 'draft' | 'open' = 'draft') {
  const client = new QueryClient()
  render(<QueryClientProvider client={client}><AuthContext value={auth}><StablefordSettings round={{ ...round, status }} /></AuthContext></QueryClientProvider>)
  return client
}
afterEach(() => { cleanup(); vi.restoreAllMocks() })
it('validates 0–100 and persists enabled/allowance with the exact draft timestamp', async () => {
  const save = vi.spyOn(stablefordApi, 'settings').mockResolvedValue({ ...round, handicap_allowance_percent: 0, handicap_enabled: false })
  mount()
  const field = screen.getByLabelText('Handicapandel (0–100 %)')
  fireEvent.change(field, { target: { value: '101' } })
  expect(screen.getByRole('button').hasAttribute('disabled')).toBe(true)
  fireEvent.change(field, { target: { value: '0' } })
  fireEvent.click(screen.getByLabelText('Bruk handicap'))
  fireEvent.click(screen.getByRole('button'))
  await waitFor(() => expect(save).toHaveBeenCalledWith(round, false, 0, session.csrf_token))
  await waitFor(() => expect(screen.getByRole('status').textContent).toContain('lagret'))
})
it('drops a stale local draft and clearly requires review of current settings before retry', async () => {
  vi.spyOn(stablefordApi, 'settings').mockRejectedValue(new ApiHttpError(409, 'round_configuration_stale', 'stale'))
  mount()
  fireEvent.change(screen.getByLabelText('Handicapandel (0–100 %)'), { target: { value: '50' } })
  fireEvent.click(screen.getByRole('button'))
  await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('Utkastet er erstattet'))
  expect((screen.getByLabelText('Handicapandel (0–100 %)') as HTMLInputElement).value).toBe(String(round.handicap_allowance_percent))
})
it('shows preserved settings after opening without mutation controls', () => {
  mount('open')
  expect(screen.getByText(/Innstillinger låst ved åpning/)).toBeTruthy()
  expect(screen.queryByRole('button')).toBeNull()
})
