// @vitest-environment jsdom
import { Component, StrictMode, useState, type ReactNode } from 'react'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, expect, it, vi } from 'vitest'
import { RequireSession } from '../features/auth/RequireSession'
import { AuthContext, type AuthContextValue } from '../features/auth/authContext'
import { ScoringGuardContext } from '../features/scoring/scoringGuardContext'
import { lazyPage } from './lazyPage'

afterEach(() => { cleanup(); vi.restoreAllMocks() })

function pendingPage() {
  let resolve: (page: () => ReactNode) => void = () => undefined
  const promise = new Promise<() => ReactNode>(done => { resolve = done })
  return { Page: lazyPage(() => promise), resolve }
}

it('announces loading without replacing state above the page and keeps page state on rerender', async () => {
  const pending = pendingPage()
  function Page() { const [value, setValue] = useState(''); return <input aria-label="Ulagret navn" value={value} onChange={e => setValue(e.target.value)} /> }
  function Shell() {
    const [count, setCount] = useState(0)
    return <><button onClick={() => setCount(n => n + 1)}>Skall {count}</button><pending.Page /></>
  }
  render(<Shell />)
  expect(screen.getByRole('status').textContent).toBe('Laster …')
  fireEvent.click(screen.getByRole('button', { name: 'Skall 0' }))
  await act(async () => pending.resolve(Page))
  expect(screen.getByRole('button', { name: 'Skall 1' })).toBeTruthy()
  fireEvent.change(screen.getByLabelText('Ulagret navn'), { target: { value: 'Behold dette' } })
  fireEvent.click(screen.getByRole('button', { name: 'Skall 1' }))
  expect((screen.getByLabelText('Ulagret navn') as HTMLInputElement).value).toBe('Behold dette')
})

it('offers only an explicit reload on import failure and honors the existing score guard', async () => {
  const load = vi.fn(async () => { throw new Error('Failed to fetch private module URL') })
  const Page = lazyPage(load)
  const view = (blocked: boolean) => <ScoringGuardContext value={{ blocked, register: vi.fn() }}><Page /></ScoringGuardContext>
  const mounted = render(view(true))
  await screen.findByRole('heading', { name: 'Siden kunne ikke lastes' })
  expect(screen.getByRole('button', { name: 'Last siden på nytt' }).hasAttribute('disabled')).toBe(true)
  expect(screen.getByRole('alert').textContent).not.toContain('private module URL')
  mounted.rerender(view(false))
  expect(screen.getByRole('button', { name: 'Last siden på nytt' }).hasAttribute('disabled')).toBe(false)
  expect(load).toHaveBeenCalledOnce()
})

it('does not classify rendering errors as chunk failures or offer a reload for them', async () => {
  vi.spyOn(console, 'error').mockImplementation(() => undefined)
  class ExistingErrorBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
    state = { failed: false }
    static getDerivedStateFromError() { return { failed: true } }
    render() { return this.state.failed ? <p>Eksisterende feilgrense</p> : this.props.children }
  }
  const Page = lazyPage(async () => () => { throw new Error('Scorer render failure') })
  render(<ExistingErrorBoundary><Page /></ExistingErrorBoundary>)
  await screen.findByText('Eksisterende feilgrense')
  expect(screen.queryByRole('button', { name: 'Last siden på nytt' })).toBeNull()
})

it('keeps a private import behind session verification', async () => {
  const load = vi.fn(async () => () => <p>Privat side</p>)
  const Page = lazyPage(load)
  const router = createMemoryRouter([
    { path: '/profile', element: <RequireSession><Page /></RequireSession> },
    { path: '/login', element: <p>Logg inn først</p> },
  ], { initialEntries: ['/profile?tab=account#details'] })
  const auth: AuthContextValue = { session: null, loading: true, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
  const mounted = render(<AuthContext value={auth}><RouterProvider router={router} /></AuthContext>)
  expect(load).not.toHaveBeenCalled()
  mounted.rerender(<AuthContext value={{ ...auth, loading: false }}><RouterProvider router={router} /></AuthContext>)
  await screen.findByText('Logg inn først')
  expect(load).not.toHaveBeenCalled()
  expect(new URLSearchParams(router.state.location.search).get('returnTo')).toBe('/profile?tab=account#details')
})

it('shares imports under StrictMode and remounts resolved code with fresh props after unmount', async () => {
  type PageComponent = (props: { name: string }) => ReactNode
  let resolve: (page: PageComponent) => void = () => undefined
  const pending = new Promise<PageComponent>(done => { resolve = done })
  const load = vi.fn(() => pending)
  const Page = lazyPage(load)
  const mounted = render(<StrictMode><Page name="Tidligere konto" /></StrictMode>)
  await act(async () => undefined)
  expect(load).toHaveBeenCalledOnce()
  mounted.unmount()
  await act(async () => resolve(({ name }) => <p>{name}</p>))
  expect(screen.queryByText('Tidligere konto')).toBeNull()
  const next = render(<StrictMode><Page name="Ny konto" /></StrictMode>)
  expect(screen.getByText('Ny konto')).toBeTruthy()
  next.rerender(<StrictMode><Page name="Oppdatert navn" /></StrictMode>)
  expect(screen.getByText('Oppdatert navn')).toBeTruthy()
  expect(load).toHaveBeenCalledOnce()
})
