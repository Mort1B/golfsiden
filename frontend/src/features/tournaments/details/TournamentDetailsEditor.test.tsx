// @vitest-environment jsdom
import { cleanup, fireEvent, screen, waitFor, act } from '@testing-library/react'
import { QueryClient, useQuery } from '@tanstack/react-query'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { tournamentApi, tournamentKeys } from '../../../api/tournaments'
import { ApiHttpError } from '../../../api/http'
import { TournamentDetailsEditor } from './TournamentDetailsEditor'
import { detailsErrors, detailsFields } from './detailsValidation'
import { deferred, mountEditor, round, session, tournament, transition } from '../__tests__/managementLifetimeFixtures'
vi.mock('../../../api/tournaments', async original => ({ ...await original<typeof import('../../../api/tournaments')>(),
  tournamentApi: { detail: vi.fn(), updateDetails: vi.fn() } }))
const saved = { ...tournament, name: 'Ny turnering', updated_at: '2026-09-09T12:00:00Z' }
let client: QueryClient
beforeEach(() => { client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } }); vi.mocked(tournamentApi.detail).mockResolvedValue(saved); vi.mocked(tournamentApi.updateDetails).mockResolvedValue(saved) })
afterEach(() => { cleanup(); client.clear(); vi.clearAllMocks() })
function mount(options: { loading?: boolean; readError?: Error; status?: import('../../../api/types').Tournament['status']; empty?: boolean } = {}) {
  function Editor() {
    const { data = tournament } = useQuery({ queryKey: tournamentKeys.detail(session.user_id, tournament.id), queryFn: () => tournamentApi.detail(tournament.id), initialData: { ...tournament, status: options.status ?? 'draft' }, staleTime: Infinity })
    return <TournamentDetailsEditor tournament={data} rounds={options.empty ? [] : [round]} loading={options.loading ?? false} readError={options.readError ?? null} retry={vi.fn()} />
  }
  return mountEditor(client, () => <Editor />)
}
function change() { fireEvent.change(screen.getByLabelText('Turneringsnavn'), { target: { value: 'Ny turnering' } }) }
function save() { fireEvent.click(screen.getByRole('button', { name: 'Lagre turneringsopplysninger' })) }
describe('tournament details editor', () => {
  it('saves with the captured version and confirms only after authoritative refresh', async () => {
    const pending = deferred<typeof saved>(); vi.mocked(tournamentApi.detail).mockReturnValue(pending.promise)
    mount(); change(); save()
    await waitFor(() => expect(tournamentApi.updateDetails).toHaveBeenCalledWith(tournament.id, { ...detailsFields(saved), expected_tournament_updated_at: tournament.updated_at }, session.csrf_token))
    expect(screen.queryByText('Turneringsopplysningene er lagret og kontrollert.')).toBeNull()
    await act(async () => pending.resolve(saved))
    await screen.findByText('Turneringsopplysningene er lagret og kontrollert.')
    expect((screen.getByLabelText('Turneringsnavn') as HTMLInputElement).value).toBe(saved.name)
    expect(screen.getByRole('button', { name: 'Lagre turneringsopplysninger' }).hasAttribute('disabled')).toBe(true)
  })
  it('retains a failed draft and supports an explicit retry', async () => {
    vi.mocked(tournamentApi.updateDetails).mockRejectedValueOnce(new Error('network'))
    mount(); change(); save(); await screen.findByText(/Kunne ikke lagre eller kontrollere/)
    expect((screen.getByLabelText('Turneringsnavn') as HTMLInputElement).value).toBe(saved.name)
    fireEvent.click(screen.getByRole('button', { name: 'Prøv lagring igjen' }))
    await screen.findByText('Turneringsopplysningene er lagret og kontrollert.')
    expect(tournamentApi.updateDetails).toHaveBeenCalledTimes(2)
  })
  it('keeps stale edits until explicit discard and reload', async () => {
    vi.mocked(tournamentApi.updateDetails).mockRejectedValue(new ApiHttpError(409, 'tournament_details_stale', 'stale'))
    vi.mocked(tournamentApi.detail).mockResolvedValue({ ...saved, name: 'Annen administrator' })
    mount(); change(); save(); await screen.findByText(/Turneringen ble endret et annet sted/)
    expect((screen.getByLabelText('Turneringsnavn') as HTMLInputElement).value).toBe(saved.name)
    fireEvent.click(screen.getByRole('button', { name: 'Forkast utkast og hent siste' }))
    await waitFor(() => expect((screen.getByLabelText('Turneringsnavn') as HTMLInputElement).value).toBe('Annen administrator'))
    expect(tournamentApi.updateDetails).toHaveBeenCalledTimes(1)
  })
  it('requires reload when an accepted edit cannot be refreshed', async () => {
    vi.mocked(tournamentApi.detail).mockRejectedValue(new Error('offline'))
    mount(); change(); save(); await screen.findByText(/Kunne ikke lagre eller kontrollere/)
    expect(screen.getByRole('button', { name: 'Prøv lagring igjen' }).hasAttribute('disabled')).toBe(true)
    vi.mocked(tournamentApi.detail).mockResolvedValue(saved)
    fireEvent.click(screen.getByRole('button', { name: 'Forkast utkast og hent siste' }))
    await waitFor(() => expect(screen.queryByText(/Kunne ikke lagre eller kontrollere/)).toBeNull())
    expect(tournamentApi.updateDetails).toHaveBeenCalledTimes(1)
  })
  it.each(['logout', 'account', 'csrf', 'unmount'])('ignores late successful writes after %s', async kind => {
    const pending = deferred<typeof saved>(); vi.mocked(tournamentApi.updateDetails).mockReturnValue(pending.promise)
    const view = mount(); change(); save()
    await waitFor(() => expect(tournamentApi.updateDetails).toHaveBeenCalledTimes(1))
    await transition(view.change, kind)
    const invalidate = vi.spyOn(client, 'invalidateQueries')
    await act(async () => pending.resolve(saved))
    expect(invalidate).not.toHaveBeenCalled(); expect(tournamentApi.detail).not.toHaveBeenCalled()
    expect(screen.queryByText('Turneringsopplysningene er lagret og kontrollert.')).toBeNull()
  })
  it.each(['active', 'completed', 'archived'] as const)('shows read-only explanation for %s', status => {
    mount({ status }); expect(screen.queryByLabelText('Turneringsnavn')).toBeNull()
    expect(screen.getByText(/bare endres mens turneringen er et utkast/)).toBeTruthy()
  })
  it.each([{ loading: true }, { readError: new Error('read') }])('disables editing while reads are unavailable: %o', options => {
    mount(options); expect(screen.getByLabelText('Turneringsnavn').closest('fieldset')?.disabled).toBe(true)
  })
  it('allows draft details without rounds and rejects invalid fields', () => {
    mount({ empty: true }); change()
    expect(screen.getByRole('button', { name: 'Lagre turneringsopplysninger' }).hasAttribute('disabled')).toBe(false)
    fireEvent.change(screen.getByLabelText('Turneringsnavn'), { target: { value: 'æ'.repeat(61) } })
    expect(screen.getByRole('button', { name: 'Lagre turneringsopplysninger' }).hasAttribute('disabled')).toBe(true)
  })
  it('checks real dates, ordering, and existing round containment', () => {
    const fields = detailsFields(tournament)
    expect(detailsErrors({ ...fields, start_date: '2026-02-30' }, [])).toHaveProperty('start_date')
    expect(detailsErrors({ ...fields, start_date: '2026-10-01' }, [])).toHaveProperty('end_date')
    expect(detailsErrors({ ...fields, start_date: '2026-09-07' }, [round])).toHaveProperty('end_date')
    expect(detailsErrors({ ...fields, description: 'æ'.repeat(1001) }, [])).toHaveProperty('description')
  })
})
