// @vitest-environment jsdom
import { StrictMode } from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { resultSharingApi, type PublicResults } from '../../api/resultSharing'
import { ApiHttpError } from '../../api/http'
import { publicFixture, shareId, shareSecret } from '../../api/resultSharing/__tests__/fixtures'
import { SharedResultsPage } from '../../pages/SharedResultsPage'
let privateClient: QueryClient
function tree(fragment = `#token=${shareSecret}`) {
  const path = `/results/shared/${shareId}${fragment}`
  window.history.replaceState(null, '', path)
  return render(<StrictMode><QueryClientProvider client={privateClient}><MemoryRouter initialEntries={[path]}><Routes><Route path="/results/shared/:grantId" element={<SharedResultsPage />} /></Routes></MemoryRouter></QueryClientProvider></StrictMode>)
}
function fragment(value: string) {
  window.history.replaceState(null, '', `/results/shared/${shareId}${value}`)
  window.dispatchEvent(new HashChangeEvent('hashchange'))
}
beforeEach(() => {
  privateClient = new QueryClient()
  Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' })
  vi.spyOn(resultSharingApi, 'read').mockImplementation(async (_id, _token, metric) => publicFixture(metric))
})
afterEach(() => { cleanup(); privateClient.clear(); vi.restoreAllMocks(); vi.useRealTimers() })
it('uses an isolated public cache, retains the reusable fragment and renders no private links', async () => {
  privateClient.setQueryData(['public-results', shareId, 'gross', 0], { ...publicFixture(), tournament_name: 'Private cached name' })
  tree(); await screen.findByRole('heading', { name: publicFixture().tournament_name })
  expect(screen.queryByText('Private cached name')).toBeNull()
  expect(screen.queryAllByRole('link')).toHaveLength(0)
  expect(window.location.hash).toBe(`#token=${shareSecret}`)
  expect(document.querySelector('meta[name=referrer]')?.getAttribute('content')).toBe('no-referrer')
  expect(document.querySelector('meta[name=robots]')?.getAttribute('content')).toBe('noindex, nofollow')
  expect(JSON.stringify(privateClient.getQueryCache().getAll().map(query => query.queryKey))).not.toContain(shareSecret)
})
it.each(['', '#token=short', '#token=' + shareSecret + '&other=1'])('rejects malformed capability %s without a request', hash => {
  tree(hash); expect(screen.getByRole('alert')).toBeTruthy(); expect(resultSharingApi.read).not.toHaveBeenCalled()
})
it('clears old metrics before a failed read and never revives them after hidden-final refresh', async () => {
  tree(); await screen.findByRole('heading', { name: publicFixture().tournament_name })
  fireEvent.click(screen.getByRole('button', { name: 'Netto' }))
  await screen.findByRole('heading', { name: 'Netto sammenlagt' })
  vi.mocked(resultSharingApi.read).mockRejectedValue(new Error('sensitive'))
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater resultater' }))
  expect(screen.queryByRole('heading', { name: 'Netto sammenlagt' })).toBeNull()
  await screen.findByText(/Tidligere resultater er skjult/)
  expect(screen.queryByText('sensitive')).toBeNull()
  const hidden = publicFixture('gross'); hidden.visibility = { mode: 'front_nine' }; hidden.entries = []
  vi.mocked(resultSharingApi.read).mockResolvedValue(hidden)
  fireEvent.click(screen.getByRole('button', { name: 'Brutto' }))
  await screen.findByText('Ingen resultater å vise ennå.')
  expect(screen.queryByText(publicFixture().entries[0]?.display_name ?? 'Anna')).toBeNull()
})
it('clears rows on native same-grant token replacement and ignores a late old response', async () => {
  tree(); await screen.findByRole('heading', { name: publicFixture().tournament_name })
  let old: ((board: PublicResults) => void) | undefined
  vi.mocked(resultSharingApi.read).mockImplementationOnce(() => new Promise(resolve => { old = resolve }))
  fireEvent.click(screen.getByRole('button', { name: 'Oppdater resultater' }))
  await waitFor(() => expect(old).toBeDefined())
  const next = 'b'.repeat(43)
  vi.mocked(resultSharingApi.read).mockRejectedValue(new ApiHttpError(404, 'result_share_unavailable', 'secret'))
  await act(async () => { fragment(`#token=${next}`) })
  await screen.findByText(/Den kan være utløpt/)
  await act(async () => { old?.(publicFixture()) })
  expect(screen.queryByRole('heading', { name: publicFixture().tournament_name })).toBeNull()
  expect(vi.mocked(resultSharingApi.read).mock.calls.some(call => call[1] === next)).toBe(true)
  await act(async () => { fragment('') })
  expect(screen.getByRole('alert').textContent).toContain('Åpne hele lenken')
})
it('clears the authorized snapshot before tab-return reauthorization', async () => {
  tree(); await screen.findByRole('heading', { name: publicFixture().tournament_name })
  vi.mocked(resultSharingApi.read).mockImplementation(() => new Promise(() => {}))
  await act(async () => { document.dispatchEvent(new Event('visibilitychange')); await Promise.resolve() })
  expect(screen.queryByRole('heading', { name: publicFixture().tournament_name })).toBeNull()
  expect(screen.getByText(/Kontrollerer lenken/)).toBeTruthy()
})
it('polls only when visible and stops after terminal revocation', async () => {
  vi.useFakeTimers()
  tree()
  await act(async () => { await vi.advanceTimersByTimeAsync(20) })
  expect(screen.getByRole('heading', { name: publicFixture().tournament_name })).toBeTruthy()
  const initial = vi.mocked(resultSharingApi.read).mock.calls.length
  Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' })
  await act(async () => { await vi.advanceTimersByTimeAsync(15_000) })
  expect(resultSharingApi.read).toHaveBeenCalledTimes(initial)
  Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' })
  vi.mocked(resultSharingApi.read).mockRejectedValue(new ApiHttpError(404, 'result_share_unavailable', 'unavailable'))
  await act(async () => { await vi.advanceTimersByTimeAsync(15_000) })
  await act(async () => { await vi.advanceTimersByTimeAsync(20) })
  expect(screen.getByRole('alert').textContent).toContain('Den kan være utløpt')
  const stopped = vi.mocked(resultSharingApi.read).mock.calls.length
  await act(async () => { await vi.advanceTimersByTimeAsync(60_000) })
  expect(resultSharingApi.read).toHaveBeenCalledTimes(stopped)
})
it('does not overflow a 30-day expiry timer and hides a snapshot exactly at local expiry', async () => {
  vi.useFakeTimers(); vi.setSystemTime(new Date('2026-09-13T10:00:00Z'))
  vi.mocked(resultSharingApi.read).mockResolvedValue({ ...publicFixture(), expires_at: '2026-10-13T10:00:00Z' })
  const view = tree()
  await act(async () => { await vi.advanceTimersByTimeAsync(20) })
  expect(screen.getByRole('heading', { name: publicFixture().tournament_name })).toBeTruthy()
  await act(async () => { await vi.advanceTimersByTimeAsync(100) })
  expect(screen.queryByRole('alert')).toBeNull()
  view.unmount()
  vi.mocked(resultSharingApi.read).mockResolvedValue({ ...publicFixture(), expires_at: new Date(Date.now() + 1000).toISOString() })
  tree(); await act(async () => { await vi.advanceTimersByTimeAsync(20) })
  await act(async () => { await vi.advanceTimersByTimeAsync(1000) })
  expect(screen.queryByRole('heading', { name: publicFixture().tournament_name })).toBeNull()
  expect(screen.getByRole('alert').textContent).toContain('Den kan være utløpt')
})
