// @vitest-environment jsdom
import type { ReactNode } from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys, type AuthSession } from '../../api/auth'
import { api } from '../../api/client'
import { fantasyApi } from '../../api/fantasy'
import { tournamentKeys } from '../../api/tournaments'
import { handleTournamentLiveSignal } from '../../api/liveInvalidation'
import { FantasyPage } from '../../pages/FantasyPage'
import { AuthContext } from '../auth/authContext'
import { ScoringGuardProvider } from '../scoring/ScoringGuardProvider'
import { tournament as trip } from '../tournaments/lifecycle/__tests__/fixtures'
import { id, picks, players, receipt, round, session, tournament, view } from './fixtures'
vi.mock('../live/useTournamentLive',()=>({useTournamentLive:()=>false}))
vi.mock('../../routing/tournamentNavigation',()=>({usePublishTournamentNavigation:()=>{}}))
vi.mock('./FantasyBoards',()=>({FantasyBoards:()=> <p>Private board</p>}))
let client:QueryClient
beforeEach(()=>{
 client=new QueryClient({defaultOptions:{queries:{retry:false},mutations:{retry:false}}});client.setQueryData(authKeys.session,session)
 vi.spyOn(api,'myTournaments').mockResolvedValue([{tournament:trip,role:'player',player_id:null}])
 vi.spyOn(api,'rounds').mockResolvedValue([round])
 vi.spyOn(api,'tournamentPlayers').mockResolvedValue({players,handicap_correction:{state:'editable'}})
 vi.spyOn(fantasyApi,'game').mockResolvedValue({tournament_id:tournament,enabled:true,rules_version:1})
 vi.spyOn(fantasyApi,'results').mockResolvedValue({tournament_id:tournament,rules_version:1,revision:'test',rounds:[],golfers:[],managers:[]})
 vi.spyOn(fantasyApi,'round').mockResolvedValue(view)
 vi.spyOn(fantasyApi,'save').mockResolvedValue(receipt)
})
afterEach(()=>{cleanup();client.clear();vi.restoreAllMocks()})
function SessionHarness({children}:{children:ReactNode}){const {data}=useQuery<AuthSession|null>({queryKey:authKeys.session,queryFn:async()=>session,enabled:false});return <AuthContext value={{session:data??null,loading:false,error:null,signIn:vi.fn(),establishSession:vi.fn(),signOut:vi.fn(),retry:vi.fn()}}>{children}</AuthContext>}
async function tree(){const router=createMemoryRouter([{path:'/tournaments/:tournamentId/fantasy',element:<FantasyPage/>},{path:'/away',element:<p>Away</p>}],{initialEntries:[`/tournaments/${tournament}/fantasy`]});render(<QueryClientProvider client={client}><SessionHarness><ScoringGuardProvider><RouterProvider router={router}/></ScoringGuardProvider></SessionHarness></QueryClientProvider>);await screen.findByRole('checkbox',{name:players[0]?.display_name});return router}
function choose(){for(const p of players.slice(0,4))fireEvent.click(screen.getByRole('checkbox',{name:p.display_name}));fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[0]}})}
it('copies a reviewed lineup to future rounds using each destination revision',async()=>{
 const second={...round,id:id(70),round_number:2},third={...round,id:id(71),round_number:3}
 vi.mocked(api.rounds).mockResolvedValue([round,second,third])
 vi.mocked(fantasyApi.round).mockImplementation(async(_t,r)=>({...view,window:{...view.window,round_id:r},selections:r===second.id?[{user_id:session.user_id,state:'draft',locked_at:null,receipt:{...receipt,round_id:r,revision:4}}]:[]}))
 await tree();choose()
 fireEvent.click(screen.getByRole('button',{name:'Kopier fireren til andre åpne runder'}))
 fireEvent.click(screen.getByRole('button',{name:'Forbered valgte runder'}))
 expect(fantasyApi.save).not.toHaveBeenCalled()
 fireEvent.change(screen.getByLabelText('Runde for Min firer'),{target:{value:second.id}})
 await waitFor(()=>expect(screen.getByLabelText('Kaptein · doble poeng')).toHaveProperty('value',picks[0]))
 await waitFor(()=>expect(screen.getByRole('button',{name:'Lagre firer og kaptein'})).toHaveProperty('disabled',false))
 fireEvent.click(screen.getByRole('button',{name:'Lagre firer og kaptein'}))
 await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalled())
 expect(vi.mocked(fantasyApi.save).mock.calls[0]?.[3].expected_revision).toBe(4)
})
const moreRounds=[round,{...round,id:id(70),round_number:2},{...round,id:id(71),round_number:3}]
function setupRounds(){
 const saved=new Map<string,typeof receipt>()
 vi.mocked(api.rounds).mockResolvedValue(moreRounds)
 vi.mocked(fantasyApi.round).mockImplementation(async(_t,r)=>({...view,window:{...view.window,round_id:r},selections:saved.has(r)?[{user_id:session.user_id,state:'draft',locked_at:null,receipt:saved.get(r)??null}]:[]}))
 vi.mocked(fantasyApi.save).mockImplementation(async(_t,r,_u,body)=>{const rec={...receipt,...body,round_id:r,revision:body.expected_revision+1};saved.set(r,rec);return rec})
 return saved
}
async function prepare(){await tree();choose();const button=screen.getByRole('button',{name:'Kopier fireren til andre åpne runder'});await waitFor(()=>expect(button).toHaveProperty('disabled',false));fireEvent.click(button);fireEvent.click(screen.getByRole('button',{name:'Forbered valgte runder'}))}
async function saveBatch(count=3){const button=screen.getByRole('button',{name:`Lagre gjennomgåtte utkast (${count})`});await waitFor(()=>expect(button).toHaveProperty('disabled',false));fireEvent.click(button)}
it('saves three independent drafts in tournament order and preserves a captain override',async()=>{
 setupRounds();await prepare()
 fireEvent.change(screen.getByLabelText('Runde for Min firer'),{target:{value:id(70)}})
 await waitFor(()=>expect(screen.getByLabelText('Kaptein · doble poeng').closest('fieldset')).toHaveProperty('disabled',false))
 fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[1]}})
 await saveBatch();await screen.findByText(/3 av 3 innsendinger bekreftet/)
 expect(vi.mocked(fantasyApi.save).mock.calls.map(call=>call[1])).toEqual(moreRounds.map(r=>r.id))
 expect(vi.mocked(fantasyApi.save).mock.calls.map(call=>call[3].captain)).toEqual([picks[0],picks[1],picks[0]])
})
it('pauses on a lost second response, replays exactly, then saves only the remaining draft',async()=>{
 const saved=setupRounds();let secondAttempts=0
 vi.mocked(fantasyApi.save).mockImplementation(async(_t,r,_u,body)=>{const rec={...receipt,...body,round_id:r,revision:body.expected_revision+1};saved.set(r,rec);if(r===id(70)&&secondAttempts++===0)throw new TypeError('lost response');return rec})
 await prepare();await saveBatch();await screen.findByText(/1 av 3 innsendinger bekreftet/)
 expect(fantasyApi.save).toHaveBeenCalledTimes(2)
 const uncertain=vi.mocked(fantasyApi.save).mock.calls[1]?.[3]
 fireEvent.change(screen.getByLabelText('Runde for Min firer'),{target:{value:id(70)}})
 const replay=await screen.findByRole('button',{name:'Avklar samme innsending'});await waitFor(()=>expect(replay).toHaveProperty('disabled',false));fireEvent.click(replay)
 await screen.findByText('Handlingen er bekreftet. Visningen er oppdatert.')
 expect(vi.mocked(fantasyApi.save).mock.calls[2]?.[3]).toEqual(uncertain)
 await saveBatch(1);await screen.findByText(/1 av 1 innsendinger bekreftet/)
 expect(vi.mocked(fantasyApi.save).mock.calls.map(call=>call[1])).toEqual([round.id,id(70),id(70),id(71)])
})
it('retains an acknowledged receipt and pauses all remaining rounds after refresh failure',async()=>{
 const saved=setupRounds();const normal=vi.mocked(fantasyApi.round).getMockImplementation();let fail=false
 vi.mocked(fantasyApi.save).mockImplementation(async(_t,r,_u,body)=>{const rec={...receipt,...body,round_id:r,revision:1};saved.set(r,rec);fail=true;return rec})
 vi.mocked(fantasyApi.round).mockImplementation(async(...args)=>{if(fail)throw new TypeError('refresh failed');if(!normal)throw new Error('missing fixture');return normal(...args)})
 await prepare();await saveBatch();await screen.findByText(/1 av 3 innsendinger bekreftet/)
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
 fail=false;fireEvent.click(screen.getByRole('button',{name:'Prøv oppdatering igjen'}));await screen.findByText('Handlingen er bekreftet. Visningen er oppdatert.')
 await waitFor(()=>expect(screen.getByRole('button',{name:'Lagre gjennomgåtte utkast (2)'})).toHaveProperty('disabled',false))
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
})
it.each(['closed','withdrawn','insufficient','revision'])('rechecks %s after the first save and never dispatches the next draft',async mode=>{
 const saved=setupRounds(),normal=vi.mocked(fantasyApi.round).getMockImplementation()
 vi.mocked(fantasyApi.round).mockImplementation(async(...args)=>{
  if(!normal)throw new Error('missing fixture');const data=await normal(...args)
  if(saved.size&&args[1]===id(70))return {...data,selection_availability:mode==='closed'?'closed':mode==='insufficient'?'insufficient_players':data.selection_availability,eligible_players:mode==='withdrawn'?data.eligible_players.slice(1):data.eligible_players,selections:mode==='revision'?[{user_id:session.user_id,state:'draft',locked_at:null,receipt:{...receipt,round_id:id(70),revision:8}}]:data.selections}
  return data
 })
 await prepare();await saveBatch();await screen.findByText(/En runde er stengt, har ugyldige valg eller et nyere lag/)
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
})
it('requires explicit consent to replace existing unsaved choices and invalidates stale consent',async()=>{
 setupRounds();await prepare()
 fireEvent.change(screen.getByLabelText('Runde for Min firer'),{target:{value:id(70)}})
 await waitFor(()=>expect(screen.getByLabelText('Kaptein · doble poeng').closest('fieldset')).toHaveProperty('disabled',false))
 fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[1]}})
 fireEvent.change(screen.getByLabelText('Runde for Min firer'),{target:{value:round.id}})
 const copy=screen.getByRole('button',{name:'Kopier fireren til andre åpne runder'});await waitFor(()=>expect(copy).toHaveProperty('disabled',false));fireEvent.click(copy)
 expect(screen.getByRole('region',{name:'Gjennomgå kopiering'})).toBe(document.activeElement)
 expect(screen.getByRole('checkbox',{name:'Erstatt også mine ulagrede valg i runde 2'})).toHaveProperty('checked',false)
 fireEvent.click(screen.getByRole('button',{name:'Forbered valgte runder'}))
 fireEvent.change(screen.getByLabelText('Runde for Min firer'),{target:{value:id(70)}})
 await waitFor(()=>expect(screen.getByLabelText('Kaptein · doble poeng')).toHaveProperty('value',picks[1]))
 fireEvent.click(screen.getByRole('button',{name:'Kopier fireren til andre åpne runder'}))
 fireEvent.click(screen.getByRole('checkbox',{name:'Erstatt også mine ulagrede valg i runde 1'}))
 fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[2]}})
 expect(screen.getByRole('button',{name:'Forbered valgte runder'})).toHaveProperty('disabled',true)
 expect(fantasyApi.save).not.toHaveBeenCalled()
})
it('pauses after privacy erasure during a held write while retaining its acknowledgement',async()=>{
 const saved=setupRounds();let finish:(value:typeof receipt)=>void=()=>{}
 vi.mocked(fantasyApi.save).mockImplementation(()=>new Promise(resolve=>{finish=resolve}))
 await prepare();await saveBatch();await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalledTimes(1))
 expect(screen.queryByRole('button',{name:'Forkast ulagrede Fantasy-valg'})).toBeNull()
 expect(screen.getByLabelText('Kaptein · doble poeng').closest('fieldset')).toHaveProperty('disabled',true)
 await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'error')})
 const body=vi.mocked(fantasyApi.save).mock.calls[0]?.[3];if(!body)throw new Error('missing write')
 const rec={...receipt,...body,round_id:round.id};saved.set(round.id,rec)
 await act(async()=>finish(rec))
 await screen.findByText(/1 av 3 innsendinger bekreftet/)
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
 await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'open')})
 await screen.findByRole('button',{name:'Lagre gjennomgåtte utkast (2)'})
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
})
it.each(['csrf','account'])('never continues the batch after canonical %s replacement',async mode=>{
 setupRounds();let finish:(value:typeof receipt)=>void=()=>{}
 vi.mocked(fantasyApi.save).mockImplementation(()=>new Promise(resolve=>{finish=resolve}))
 await prepare();await saveBatch();await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalledTimes(1))
 await act(async()=>{client.setQueryData(authKeys.session,{...session,user_id:mode==='account'?id(99):session.user_id,csrf_token:'replacement'})})
 await act(async()=>finish(receipt))
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
 expect(screen.queryByText(/av 3 innsendinger bekreftet/)).toBeNull()
})
it('uses server availability despite a fast device clock',async()=>{
 setupRounds();const normal=vi.mocked(fantasyApi.round).getMockImplementation()
 vi.mocked(fantasyApi.round).mockImplementation(async(...args)=>{if(!normal)throw new Error('fixture');const data=await normal(...args);return {...data,window:{...data.window,deadline:'2000-01-01T00:00:00Z'}}})
 await prepare();await saveBatch();await screen.findByText(/3 av 3 innsendinger bekreftet/)
 expect(fantasyApi.save).toHaveBeenCalledTimes(3)
})
it('excludes a closed destination and leaves a later-created round unprepared',async()=>{
 setupRounds();const normal=vi.mocked(fantasyApi.round).getMockImplementation()
 vi.mocked(fantasyApi.round).mockImplementation(async(...args)=>{if(!normal)throw new Error('fixture');const data=await normal(...args);return args[1]===id(70)?{...data,selection_availability:'closed'}:data})
 await tree();choose()
 const copy=screen.getByRole('button',{name:'Kopier fireren til andre åpne runder'});await waitFor(()=>expect(copy).toHaveProperty('disabled',false));fireEvent.click(copy)
 const preview=screen.getByRole('region',{name:'Gjennomgå kopiering'})
 expect(within(preview).queryByText(/Runde 2:/)).toBeNull()
 expect(within(preview).getByText(/Runde 3:/)).toBeTruthy()
 fireEvent.click(screen.getByRole('button',{name:'Forbered valgte runder'}))
 const added={...round,id:id(72),round_number:4}
 vi.mocked(api.rounds).mockResolvedValue([...moreRounds,added])
 await act(async()=>{await client.invalidateQueries({queryKey:tournamentKeys.rounds(session.user_id,tournament)})})
 const addedRow=(await screen.findByRole('heading',{name:`Runde 4: ${round.name}`})).closest('li')
 if(!addedRow)throw new Error('missing row')
 await waitFor(()=>expect(within(addedRow).getByText('Ikke forberedt')).toBeTruthy())
 await saveBatch(2);await screen.findByText(/2 av 2 innsendinger bekreftet/)
 expect(vi.mocked(fantasyApi.save).mock.calls.map(call=>call[1])).toEqual([round.id,id(71)])
})
it('discards only one round and requires deliberate rebase before saving a stale draft',async()=>{
 const saved=setupRounds();await prepare()
 fireEvent.click(screen.getByRole('button',{name:'Forkast utkast for runde 3'}))
 saved.set(id(70),{...receipt,round_id:id(70),revision:8})
 fireEvent.click(screen.getByRole('button',{name:'Oppdater Fantasy'}))
 fireEvent.change(screen.getByLabelText('Runde for Min firer'),{target:{value:id(70)}})
 const rebase=await screen.findByRole('button',{name:'Behold utkastet mot gjeldende revisjon'})
 await waitFor(()=>expect(rebase).toHaveProperty('disabled',false))
 expect(screen.getByRole('button',{name:'Lagre firer og kaptein'})).toHaveProperty('disabled',true)
 expect(fantasyApi.save).not.toHaveBeenCalled()
 fireEvent.click(rebase)
 await saveBatch(2);await screen.findByText(/2 av 2 innsendinger bekreftet/)
 expect(vi.mocked(fantasyApi.save).mock.calls.map(call=>[call[1],call[3].expected_revision])).toEqual([[round.id,0],[id(70),8]])
})
it('does not dispatch offline and serializes repeated clicks while a save is held',async()=>{
 setupRounds();await prepare()
 const online=vi.spyOn(navigator,'onLine','get').mockReturnValue(false)
 await saveBatch();await screen.findByText(/Lagringen er satt på pause. Oppdater tilgang/)
 expect(fantasyApi.save).not.toHaveBeenCalled()
 online.mockReturnValue(true)
 let finish:(value:typeof receipt)=>void=()=>{}
 vi.mocked(fantasyApi.save).mockImplementation(()=>new Promise(resolve=>{finish=resolve}))
 await saveBatch();await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalledTimes(1))
 const saving=screen.getByRole('button',{name:/Lagre gjennomgåtte utkast/})
 expect(saving).toHaveProperty('disabled',true);fireEvent.click(saving)
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
 await act(async()=>{client.setQueryData(authKeys.session,{...session,csrf_token:'stop-held-test'})})
 await act(async()=>finish(receipt))
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
})

it('reconciles each batch target once after its authoritative preflight',async()=>{
 setupRounds();await prepare()
 const button=screen.getByRole('button',{name:'Lagre gjennomgåtte utkast (3)'})
 await waitFor(()=>expect(button).toHaveProperty('disabled',false))
 const before=vi.mocked(fantasyApi.round).mock.calls.length
 fireEvent.click(button)
 await screen.findByText(/Alle gjennomgåtte utkast er lagret/)
 // Three preflights plus all three active rounds after each write. No extra target GET.
 expect(vi.mocked(fantasyApi.round).mock.calls.length-before).toBe(12)
 expect(fantasyApi.save).toHaveBeenCalledTimes(3)
})
