// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys } from '../../api/auth'
import { fantasyApi, type RoundView, type Save } from '../../api/fantasy'
import { ApiHttpError } from '../../api/http'
import { AuthContext } from '../auth/authContext'
import { FantasyDraftProvider, FantasyRecoveryStatus } from './FantasyDraftProvider'
import { ScoringGuardProvider } from '../scoring/ScoringGuardProvider'
import { MyFour } from './MyFour'
import { id, picks, players, receipt, round, roundId, session, tournament, view } from './fixtures'
vi.mock('../../api/fantasy',async original=>({...await original<typeof import('../../api/fantasy')>(),fantasyApi:{save:vi.fn(),enter:vi.fn()}}))
vi.mock('react-router-dom',()=>({useBlocker:()=>({state:'unblocked'})}))
let client:QueryClient
beforeEach(()=>{vi.resetAllMocks();client=new QueryClient({defaultOptions:{queries:{retry:false},mutations:{retry:false}}});client.setQueryData(authKeys.session,session);vi.mocked(fantasyApi.save).mockImplementation(async(_t,_r,_u,body)=>({...receipt,...body,revision:body.expected_revision+1}));vi.stubGlobal('crypto',{randomUUID:()=>id(50)})})
afterEach(()=>{cleanup();client.clear();vi.unstubAllGlobals()})
function element(data:RoundView=view){return (<QueryClientProvider client={client}><AuthContext.Provider value={{session,loading:false,error:null,signIn:vi.fn(),establishSession:vi.fn(),signOut:vi.fn(),retry:vi.fn()}}><ScoringGuardProvider><FantasyDraftProvider tournament={tournament}><MyFour tournament={tournament} round={roundId} view={data} players={players} rounds={[round]} refreshing={false}/><FantasyRecoveryStatus/></FantasyDraftProvider></ScoringGuardProvider></AuthContext.Provider></QueryClientProvider>)}
function tree(data:RoundView=view){return render(element(data))}
function choose(){for(const p of players.slice(0,4))fireEvent.click(screen.getByRole('checkbox',{name:p.display_name}));fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[0]}})}
it('requires four unique players and a selected captain before saving the whole lineup',async()=>{tree();expect(screen.getByRole('button',{name:'Lagre firer og kaptein'})).toHaveProperty('disabled',true);choose();fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await screen.findByText(/Innsending revisjon 1 er bekreftet/);expect(fantasyApi.save).toHaveBeenCalledWith(tournament,roundId,session.user_id,{request_id:id(50),expected_revision:0,picks,captain:picks[0]},session.csrf_token)})
it('keeps the previous-round carry preview separate from the current expected revision',async()=>{tree({...view,carry_forward_preview:{...receipt,round_id:id(99),revision:8},carry_forward_eligible:true});choose();fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalled());expect(vi.mocked(fantasyApi.save).mock.calls[0]?.[3].expected_revision).toBe(0)})
it('allows removing a selected golfer who became ineligible and replacing them',async()=>{tree({...view,eligible_players:players.slice(1).map(p=>p.player_id),selections:[{user_id:session.user_id,state:'invalid_draft',locked_at:null,receipt}]});const checkbox=screen.getByRole('checkbox',{name:/Spiller 1 .*ikke valgbar/});expect(checkbox).toHaveProperty('checked',true);fireEvent.click(checkbox);fireEvent.click(screen.getByRole('checkbox',{name:players[4]?.display_name}));fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[1]}});fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalled());expect(vi.mocked(fantasyApi.save).mock.calls[0]?.[3].picks).toEqual([id(11),id(12),id(13),id(14)])})
it('retries uncertain delivery with exactly the same request, revision and pick order',async()=>{vi.mocked(fantasyApi.save).mockRejectedValueOnce(new TypeError('network'));tree();choose();fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await screen.findByText(/Lagringen er ikke bekreftet/);expect(screen.getByRole('button',{name:'Lagre firer og kaptein'})).toHaveProperty('disabled',true);const first=vi.mocked(fantasyApi.save).mock.calls[0]?.[3];fireEvent.click(screen.getByRole('button',{name:'Avklar samme innsending'}));await screen.findByText(/Innsending revisjon 1 er bekreftet/);expect(vi.mocked(fantasyApi.save).mock.calls[1]?.[3]).toEqual(first)})
it.each(['csrf','logout','unmount'])('ignores held mutation completion after %s',async mode=>{let finish:(value:typeof receipt)=>void=()=>{};vi.mocked(fantasyApi.save).mockImplementation(()=>new Promise(resolve=>{finish=resolve}));const mounted=tree();choose();fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalled());const invalidate=vi.spyOn(client,'invalidateQueries');if(mode==='unmount')mounted.unmount();else client.setQueryData(authKeys.session,mode==='logout'?null:{...session,csrf_token:'renewed'});await act(async()=>finish(receipt));expect(invalidate).not.toHaveBeenCalled();expect(screen.queryByText(/Innsending revisjon 1 er bekreftet/)).toBeNull()})
it('keeps a rejected save unsaved and explains deadline closure',async()=>{vi.mocked(fantasyApi.save).mockRejectedValue(new ApiHttpError(409,'fantasy_closed','closed'));tree();choose();fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await screen.findByText(/Fristen er passert/);expect(screen.queryByText(/Innsending revisjon/)).toBeNull();expect(screen.getByText('Ulagrede endringer')).toBeTruthy()})
it('renders locked origin, empty availability and explicit invalid selection',()=>{tree({...view,window:{...view.window,locked_at:'2026-01-01T12:00:00Z'},selection_availability:'closed',selections:[{user_id:session.user_id,state:'invalid',locked_at:'2026-01-01T12:00:00Z',receipt:null}]});expect(screen.getByText('Ugyldig lag ved fristen – 0 poeng')).toBeTruthy();expect(screen.queryByLabelText('Kaptein · doble poeng')).toBeNull()})
it('does not automatically submit or retry offline',async()=>{vi.spyOn(navigator,'onLine','get').mockReturnValue(false);tree();choose();fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await screen.findByText(/Du er frakoblet/);expect(fantasyApi.save).not.toHaveBeenCalled()})
it('shows stale-edit conflict instead of replacing newer server selection',async()=>{let input:Save|undefined;vi.mocked(fantasyApi.save).mockImplementation(async(_t,_r,_u,body)=>{input=body;throw new ApiHttpError(409,'fantasy_conflict','changed')});tree({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt}]});fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[1]}});fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await screen.findByText(/Grunnlaget ble endret/);expect(input?.expected_revision).toBe(1);expect(screen.getByText('Ulagrede endringer')).toBeTruthy()})

it('reconciles an old accepted retry with a newer current lineup without claiming the old one is current',async()=>{
  vi.mocked(fantasyApi.save).mockRejectedValueOnce(new TypeError('network'))
  const mounted=tree();choose();fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}));await screen.findByText(/Lagringen er ikke bekreftet/)
  const newer={...receipt,revision:2,request_id:id(80),captain:picks[1]??''}
  mounted.rerender(element({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt:newer}]}))
  fireEvent.click(screen.getByRole('button',{name:'Avklar samme innsending'}))
  await screen.findByText(/Et nyere lag er allerede lagret/)
  expect(screen.getByLabelText('Kaptein · doble poeng')).toHaveProperty('value',picks[1])
  expect(screen.getByText('Lagret lag · revisjon 2')).toBeTruthy()
})

it('does not suggest carry-forward will replace a valid saved lineup for this round',()=>{
  tree({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt}],carry_forward_preview:{...receipt,round_id:id(99),captain:picks[1]??''},carry_forward_eligible:true})
  expect(screen.getByText('Lagret lag · revisjon 1')).toBeTruthy()
  expect(screen.queryByText('Hvis du ikke leverer et nytt gyldig lag')).toBeNull()
})
