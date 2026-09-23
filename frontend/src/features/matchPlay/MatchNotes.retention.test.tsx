// @vitest-environment jsdom
import { StrictMode } from 'react'
import { IDBFactory } from 'fake-indexeddb'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { AuthContext, type AuthContextValue } from '../auth/authContext'
import { session } from '../tournaments/lifecycle/__tests__/fixtures'
import { ScoringGuardProvider } from '../scoring/ScoringGuardProvider'
import { AppShell } from '../../ui/AppShell'
import { matchFixture } from '../../api/matchPlay/fixtures'
import { matchDatabase } from './offline/database'
import { STORAGE_ERROR } from './offline/model'
import { MatchPage } from '../../pages/MatchPage'
import { api } from '../../api/client'
import { matchApi } from '../../api/matchPlay'
import { round, tournament } from '../tournaments/lifecycle/__tests__/fixtures'
import { MatchScoring } from './MatchScoring'
vi.mock('../live/useTournamentLive', () => ({ useTournamentLive: () => false }))
const card = matchFixture()
const auth: AuthContextValue = { session, loading: false, error: null, signIn: vi.fn(), signOut: vi.fn(), establishSession: vi.fn(), retry: vi.fn() }
const clients: QueryClient[] = []
beforeEach(() => { vi.stubGlobal('indexedDB', new IDBFactory()); vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false) })
afterEach(() => { cleanup(); clients.splice(0).forEach(c => c.clear()); vi.restoreAllMocks(); vi.unstubAllGlobals() })
function mount(workspace = false) {
 const client = new QueryClient({ defaultOptions: { queries: { retry: false, networkMode: 'always' } } }); clients.push(client)
 const router = createMemoryRouter([{path:'/',element:<AppShell/>,children:[{path:'rounds/:roundId/matches/:matchId/score',element:workspace ? <MatchPage scoring/> : <MatchScoring card={card} admin recovering={false}/> }, {path:'profile', element:<p>Other route</p>}]}],{initialEntries:[`/rounds/${card.round_id}/matches/${card.match_id}/score`]})
 const tree=(value:AuthContextValue)=><StrictMode><QueryClientProvider client={client}><AuthContext value={value}><ScoringGuardProvider><RouterProvider router={router}/></ScoringGuardProvider></AuthContext></QueryClientProvider></StrictMode>
 const view=render(tree(auth))
 return { client, router, change:(value:AuthContextValue)=>view.rerender(tree(value)) }
}
for(const failed of [false,true]) it(`preserves ${failed?'failed-save':'unsaved'} input, hole and guard across same-account replacement`,async()=>{
 if(failed)vi.spyOn(matchDatabase,'enqueue').mockRejectedValue(new Error(STORAGE_ERROR))
 const view=mount(), input=()=>screen.getByLabelText(`Notat · ${card.opponents[0].display_name}`) as HTMLInputElement
 await waitFor(()=>expect(input().disabled).toBe(false))
 fireEvent.change(screen.getByLabelText('Hull'),{target:{value:'3'}})
 fireEvent.change(input(),{target:{value:'7'}})
 if(failed){const save=screen.getAllByRole('button',{name:'Lagre notat'})[0];if(!save)throw new Error('missing save');fireEvent.click(save);await waitFor(()=>expect(screen.getByRole('alert').textContent).toContain(STORAGE_ERROR))}
 view.change(auth)
 expect(input().value).toBe('7')
 view.change({...auth,session:{...session,csrf_token:'replacement'}})
 await waitFor(()=>expect(input().value).toBe('7'))
 expect((screen.getByLabelText('Hull') as HTMLSelectElement).value).toBe('3')
 expect(screen.getByRole('button',{name:'Logg ut'}).hasAttribute('disabled')).toBe(true)
 if(failed)expect(screen.getByRole('alert').textContent).toContain(STORAGE_ERROR)
 fireEvent.click(screen.getByRole('button',{name:'Forkast ulagrede notater'}))
 await waitFor(()=>expect(screen.getByRole('button',{name:'Logg ut'}).hasAttribute('disabled')).toBe(false))
})
it('clears transient input across account departure and never restores it on return',async()=>{
 const view=mount();const input=()=>screen.getByLabelText(`Notat · ${card.opponents[0].display_name}`) as HTMLInputElement
 await waitFor(()=>expect(input().disabled).toBe(false))
 fireEvent.change(input(),{target:{value:'7'}})
 await act(async()=>view.change({...auth,session:{...session,user_id:'00000000-0000-4000-8000-000000000099'}}))
 expect(input().value).toBe('')
 await act(async()=>view.change({...auth,session:null}))
 view.change(auth)
 await waitFor(()=>expect(input().value).toBe(''))
})

for (const outcome of ['success', 'failure'] as const) it(`settles a pending ${outcome} through actual provider replacement`, async () => {
 const enqueue = matchDatabase.enqueue.bind(matchDatabase)
 let release: (() => void) | undefined
 const gate = new Promise<void>(resolve => { release = resolve })
 vi.spyOn(matchDatabase, 'enqueue').mockImplementation(async (...args) => {
  const saved = outcome === 'success' ? await enqueue(...args) : null
  await gate
  if (!saved) throw new Error(STORAGE_ERROR)
  return saved
 })
 const view = mount(), input = () => screen.getByLabelText(`Notat · ${card.opponents[0].display_name}`) as HTMLInputElement
 await waitFor(() => expect(input().disabled).toBe(false))
 fireEvent.change(input(), { target: { value: '7' } })
 const save = screen.getAllByRole('button', { name: 'Lagre notat' })[0]; if (!save) throw new Error('missing save'); fireEvent.click(save)
 await waitFor(() => expect(input().disabled).toBe(true))
 view.change({ ...auth, session: { ...session, csrf_token: 'replacement' } })
 expect(input().value).toBe('7')
 await act(async () => release?.())
 if (outcome === 'success') await waitFor(() => expect(screen.queryByRole('button', { name: 'Forkast ulagrede notater' })).toBeNull())
 else {
  await waitFor(() => expect(screen.getByRole('alert').textContent).toContain(STORAGE_ERROR))
  expect(input().value).toBe('7'); expect(input().disabled).toBe(false)
 }
})
for (const state of ['round-loading', 'round-error', 'card-error', 'membership-denied', 'locked', 'terminal'] as const) it(`retains guarded local recovery after replacement with ${state}`, async () => {
 const roundRead = vi.spyOn(api, 'round').mockResolvedValue({ ...round, id: card.round_id, tournament_id: card.tournament_id, status: 'open', scoring_format: 'singles_match_play' })
 const cardRead = vi.spyOn(matchApi, 'scoring').mockResolvedValue(card)
 const members = vi.spyOn(api, 'myTournaments').mockResolvedValue([{ tournament: { ...tournament, id: card.tournament_id }, role: 'admin', player_id: session.player_id }])
 const view = mount(true), input = () => screen.getByLabelText(`Notat · ${card.opponents[0].display_name}`) as HTMLInputElement
 await waitFor(() => expect(input().disabled).toBe(false))
 fireEvent.change(input(), { target: { value: '7' } })
 if (state === 'round-loading') roundRead.mockImplementation(() => new Promise(() => {}))
 if (state === 'round-error') roundRead.mockRejectedValue(new Error('Synthetic round failure'))
 if (state === 'card-error') cardRead.mockRejectedValue(new Error('Synthetic card failure'))
 if (state === 'membership-denied') members.mockResolvedValue([])
 if (state === 'locked') cardRead.mockResolvedValue({ ...card, round_status: 'locked' })
 if (state === 'terminal') cardRead.mockResolvedValue({ ...card, finish: { type: 'conceded', winner: 'first' } })
 await act(async () => {
  view.client.removeQueries()
  view.change({ ...auth, session: { ...session, csrf_token: 'replacement' } })
 })
 await waitFor(() => expect(screen.getByText('Lokalt notat 1: 7')).toBeTruthy())
 expect(screen.queryByLabelText(`Notat · ${card.opponents[0].display_name}`)).toBeNull()
 expect(screen.getByRole('button', { name: 'Logg ut' }).hasAttribute('disabled')).toBe(true)
 const unload = new Event('beforeunload', { cancelable: true }); window.dispatchEvent(unload); expect(unload.defaultPrevented).toBe(true)
 await act(async () => { await view.router.navigate('/profile') })
 expect(view.router.state.location.pathname).toContain('/matches/')
 fireEvent.click(screen.getByRole('button', { name: 'Forkast ulagrede notater' }))
 await act(async () => { await view.router.navigate('/profile') })
 expect(view.router.state.location.pathname).toBe('/profile')
})
