// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { decodeCoursePresets, presetApi, presetKey } from '../../api/coursePresets'
import { presetResponse } from '../../api/coursePresets.fixture'
import { courseApi } from '../../api/courses'
import { privateWorkspaceKeys, clearPrivateWorkspace } from '../../api/privateWorkspace'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { round, session, tournament } from './lifecycle/__tests__/fixtures'
import { SavedCoursePicker } from './SavedCoursePicker'
import { useCourseConfiguration } from './useCourseConfiguration'

let client: QueryClient
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), establishSession: vi.fn(), signOut: vi.fn(), retry: vi.fn() }
const presets = decodeCoursePresets(presetResponse)
function presetAt(index: number) {
  const preset = presets[index]
  if (!preset) throw new Error('Missing expected preset')
  return preset
}
const first = presetAt(0)
const third = presetAt(2)
function Wrapper({ children }: { children: ReactNode }) { return <QueryClientProvider client={client}><AuthContext value={auth}>{children}</AuthContext></QueryClientProvider> }
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } })
  vi.spyOn(presetApi, 'list').mockResolvedValue(presets)
})
afterEach(() => { cleanup(); client.clear(); vi.restoreAllMocks() })
it('inspects and explicitly applies exact male red-tee facts; blocks while saving or read-only', async () => {
  const save = vi.fn().mockResolvedValue(true)
  const tree = (enabled = true, saving = false) => <Wrapper><SavedCoursePicker tournamentId={tournament.id} enabled={enabled} saving={saving} error={null} onSave={save} /></Wrapper>
  const view = render(tree())
  const select = await screen.findByLabelText('Lagret bane')
  const button = screen.getByRole('button', { name: 'Bruk lagret bane på runden' })
  expect(button.hasAttribute('disabled')).toBe(true)
  fireEvent.change(select, { target: { value: third.id } })
  expect(screen.getByText('Red Tees · Herre · 18 hull · Par 72')).toBeTruthy()
  expect(screen.getByText('Baneverdi 66,9 · Slope 118')).toBeTruthy()
  expect(save).not.toHaveBeenCalled()
  fireEvent.click(button)
  expect(save).toHaveBeenCalledExactlyOnceWith(third.selection)
  view.rerender(tree(true, true))
  expect(screen.getByRole('button', { name: 'Lagrer …' }).hasAttribute('disabled')).toBe(true)
  view.rerender(tree(false))
  expect(button.hasAttribute('disabled')).toBe(true)
})
it('shows loading, then empty and recoverable errors, without a cached selectable layout', async () => {
  let resolve!: (v: typeof presets) => void
  vi.mocked(presetApi.list).mockReturnValue(new Promise((r) => { resolve = r }))
  render(<Wrapper><SavedCoursePicker tournamentId={tournament.id} enabled saving={false} error={null} onSave={vi.fn()} /></Wrapper>)
  expect(screen.getByRole('status').textContent).toBe('Laster …')
  await act(async () => resolve([]))
  await screen.findByText('Ingen lagrede baner er tilgjengelige.')
  vi.mocked(presetApi.list).mockRejectedValue(new Error('offline'))
  await act(async () => { await client.refetchQueries({ queryKey: presetKey(session.user_id, tournament.id) }) })
  await screen.findByText('offline')
  expect(screen.queryByLabelText('Lagret bane')).toBeNull()
  vi.mocked(presetApi.list).mockResolvedValue(presets)
  fireEvent.click(screen.getByRole('button', { name: 'Prøv igjen' }))
  await screen.findByLabelText('Lagret bane')
})
it('does not reinsert cleared private data when a successful save resolves after unmount', async () => {
  let resolve!: (v: typeof round) => void
  vi.spyOn(courseApi, 'configure').mockReturnValue(new Promise((r) => { resolve = r }))
  const hook = renderHook(() => useCourseConfiguration({ tournamentId: tournament.id, round, providerCourseId: '', catalogQuery: '', expanded: false }), { wrapper: Wrapper })
  let pending!: ReturnType<typeof hook.result.current.save>
  act(() => { pending = hook.result.current.save(first.selection) })
  await waitFor(() => expect(courseApi.configure).toHaveBeenCalledTimes(1))
  hook.unmount()
  clearPrivateWorkspace(client)
  await act(async () => { resolve({ ...round, course_name: first.selection.course_name }); await pending })
  expect(await pending).toEqual({ configured: null, failure: null })
  expect(client.getQueriesData({ queryKey: privateWorkspaceKeys.root })).toEqual([])
})
