// @vitest-environment jsdom
import type { ReactNode } from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys, type AuthSession } from '../../api/auth'
import { api } from '../../api/client'
import { fantasyApi, fantasyKeys } from '../../api/fantasy'
import { ApiHttpError } from '../../api/http'
import { handleTournamentLiveSignal } from '../../api/liveInvalidation'
import { FantasyPage } from '../../pages/FantasyPage'
import { AuthContext } from '../auth/authContext'
import { ScoringGuardProvider } from '../scoring/ScoringGuardProvider'
import { tournament as trip } from '../tournaments/lifecycle/__tests__/fixtures'
import { picks, players, receipt, round, roundId, session, tournament, view } from './fixtures'
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
async function reconnect(){await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'error')});await waitFor(()=>expect(screen.queryByLabelText('Kaptein · doble poeng')).toBeNull());await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'open')});await screen.findByLabelText('Kaptein · doble poeng')}
it('retains local selections through actual parent privacy clearing and reconnect',async()=>{await tree();choose();await reconnect();expect(screen.getByLabelText('Kaptein · doble poeng')).toHaveProperty('value',picks[0]);expect(screen.getAllByRole('checkbox').filter(e=>(e as HTMLInputElement).checked)).toHaveLength(4)})
it('retains the exact uncertain request across parent unmount and reconnect',async()=>{vi.mocked(fantasyApi.save).mockRejectedValueOnce(new TypeError('lost'));await tree();choose();fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Lagringen er ikke bekreftet/);const first=vi.mocked(fantasyApi.save).mock.calls[0]?.[3];await reconnect();fireEvent.click(screen.getByText('Avklar samme innsending'));await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalledTimes(2));expect(vi.mocked(fantasyApi.save).mock.calls[1]?.[3]).toEqual(first)})
it('retains acknowledged receipt after failed refetch and retries only reads',async()=>{await tree();choose();vi.mocked(fantasyApi.round).mockRejectedValue(new TypeError('refresh failed'));fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Handlingen er bekreftet, men visningen kunne ikke oppdateres/);expect(screen.queryByText(/Lagringen er ikke bekreftet/)).toBeNull();vi.mocked(fantasyApi.round).mockResolvedValue({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt}]});fireEvent.click(screen.getByText('Prøv oppdatering igjen'));await screen.findByText(/Visningen er oppdatert/);expect(fantasyApi.save).toHaveBeenCalledTimes(1);expect(client.getQueryData(fantasyKeys.query(session.user_id,tournament,'round',roundId))).toBeTruthy()})
it('blocks navigation with local input and permits it after explicit discard',async()=>{const router=await tree();choose();await act(async()=>{await router.navigate('/away')});expect(router.state.location.pathname).toContain('/fantasy');fireEvent.click(screen.getByText('Forkast ulagrede Fantasy-valg'));await act(async()=>{await router.navigate('/away')});await screen.findByText('Away')})
it('retains draft when the round query fails and restores it on a read retry',async()=>{await tree();choose();vi.mocked(fantasyApi.round).mockRejectedValue(new TypeError('read failed'));await act(async()=>{await client.invalidateQueries({queryKey:fantasyKeys.query(session.user_id,tournament,'round',roundId)})});await waitFor(()=>expect(screen.queryByLabelText('Kaptein · doble poeng')).toBeNull());vi.mocked(fantasyApi.round).mockResolvedValue(view);fireEvent.click(screen.getByRole('button',{name:'Prøv igjen'}));await screen.findByLabelText('Kaptein · doble poeng');expect(screen.getByLabelText('Kaptein · doble poeng')).toHaveProperty('value',picks[0])})
it('preserves a held write receipt even when privacy clearing unmounts the editor',async()=>{let finish:(value:typeof receipt)=>void=()=>{};vi.mocked(fantasyApi.save).mockImplementation(()=>new Promise(resolve=>{finish=resolve}));await tree();choose();fireEvent.click(screen.getByText('Lagre firer og kaptein'));await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalled());await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'error')});await waitFor(()=>expect(screen.queryByLabelText('Kaptein · doble poeng')).toBeNull());await act(async()=>finish(receipt));await screen.findByText(/Innsending revisjon 1 er bekreftet|Kvittering beholdt/);expect(fantasyApi.save).toHaveBeenCalledTimes(1)})
it.each(['csrf','account','logout'])('isolates local recovery after canonical %s changes',async mode=>{await tree();choose();await act(async()=>{client.setQueryData(authKeys.session,mode==='logout'?null:{...session,user_id:mode==='account'?'00000000-0000-0000-0000-000000000099':session.user_id,csrf_token:'renewed'})});await waitFor(()=>expect(screen.queryByText('Ulagrede endringer')).toBeNull());expect(screen.queryByText('Forkast ulagrede Fantasy-valg')).toBeNull()})
it('resolves definitively closed replay and releases navigation after draft discard',async()=>{vi.mocked(fantasyApi.save).mockRejectedValueOnce(new TypeError('lost')).mockRejectedValueOnce(new ApiHttpError(409,'fantasy_closed','closed'));const router=await tree();choose();fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Lagringen er ikke bekreftet/);fireEvent.click(screen.getByText('Avklar samme innsending'));await screen.findByText(/Fristen er passert/);await waitFor(()=>expect(screen.queryByText('Avklar samme innsending')).toBeNull());fireEvent.click(screen.getByText('Forkast ulagrede Fantasy-valg'));await act(async()=>{await router.navigate('/away')});await screen.findByText('Away')})
it('reconciles the original acknowledged round after switching during a held write',async()=>{const second={...round,id:'00000000-0000-0000-0000-000000000088',round_number:2};vi.mocked(api.rounds).mockResolvedValue([round,second]);vi.mocked(fantasyApi.round).mockImplementation(async(_t,id)=>({...view,window:{...view.window,round_id:id}}));let finish:(value:typeof receipt)=>void=()=>{};vi.mocked(fantasyApi.save).mockImplementation(()=>new Promise(resolve=>{finish=resolve}));await tree();choose();fireEvent.click(screen.getByText('Lagre firer og kaptein'));await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalled());fireEvent.change(screen.getByLabelText('Runde for Min firer'),{target:{value:second.id}});await waitFor(()=>expect(fantasyApi.round).toHaveBeenCalledWith(tournament,second.id,session.user_id,expect.any(AbortSignal)));await act(async()=>finish(receipt));await screen.findByText('Handlingen er bekreftet. Visningen er oppdatert.');expect(fantasyApi.save).toHaveBeenCalledTimes(1);expect(vi.mocked(fantasyApi.round).mock.calls.some(call=>call[1]===roundId)).toBe(true)})
it('preserves admin acknowledgement through parent game failure and retries only reads',async()=>{vi.mocked(api.myTournaments).mockResolvedValue([{tournament:trip,role:'admin',player_id:null}]);vi.spyOn(fantasyApi,'roundResults').mockImplementation(()=>new Promise(()=>{}));vi.spyOn(fantasyApi,'configure').mockResolvedValue({tournament_id:tournament,enabled:false,rules_version:1});await tree();vi.mocked(fantasyApi.game).mockRejectedValue(new TypeError('read failed'));fireEvent.click(screen.getByRole('button',{name:'Deaktiver Fantasy'}));await screen.findByText(/Handlingen er bekreftet, men visningen kunne ikke oppdateres/);vi.mocked(fantasyApi.game).mockResolvedValue({tournament_id:tournament,enabled:false,rules_version:1});fireEvent.click(screen.getByText('Prøv oppdatering igjen'));await screen.findByText('Handlingen er bekreftet. Visningen er oppdatert.');expect(fantasyApi.configure).toHaveBeenCalledTimes(1)})
it('does not report refreshed success when enabling Fantasy mounts a failing query',async()=>{
 vi.mocked(api.myTournaments).mockResolvedValue([{tournament:trip,role:'admin',player_id:null}])
 vi.mocked(fantasyApi.game).mockResolvedValue({tournament_id:tournament,enabled:false,rules_version:1})
 vi.mocked(fantasyApi.round).mockRejectedValue(new TypeError('newly enabled read failed'))
 vi.spyOn(fantasyApi,'roundResults').mockRejectedValue(new TypeError('newly enabled read failed'))
 vi.spyOn(fantasyApi,'configure').mockImplementation(async()=>{vi.mocked(fantasyApi.game).mockResolvedValue({tournament_id:tournament,enabled:true,rules_version:1});return {tournament_id:tournament,enabled:true,rules_version:1}})
 const router=createMemoryRouter([{path:'/tournaments/:tournamentId/fantasy',element:<FantasyPage/>}],{initialEntries:[`/tournaments/${tournament}/fantasy`]})
 render(<QueryClientProvider client={client}><SessionHarness><ScoringGuardProvider><RouterProvider router={router}/></ScoringGuardProvider></SessionHarness></QueryClientProvider>)
 fireEvent.click(await screen.findByRole('button',{name:'Aktiver Fantasy'}))
 await screen.findByText(/Handlingen er bekreftet, men visningen kunne ikke oppdateres/)
 expect(screen.queryByText('Handlingen er bekreftet. Visningen er oppdatert.')).toBeNull()
 vi.mocked(fantasyApi.round).mockResolvedValue(view)
 vi.mocked(fantasyApi.roundResults).mockResolvedValue({tournament_id:tournament,revision:'test',round:{round_id:roundId,round_number:1,name:round.name,format:round.scoring_format,sporting_status:'draft',visibility:{mode:'full'},points:{state:'not_started'}},golfers:[],managers:[]})
 fireEvent.click(screen.getByText('Prøv oppdatering igjen'))
 await screen.findByText('Handlingen er bekreftet. Visningen er oppdatert.')
 expect(fantasyApi.configure).toHaveBeenCalledTimes(1)
})
it('allows leaving after acknowledged write even when refresh fails',async()=>{const router=await tree();choose();vi.mocked(fantasyApi.round).mockRejectedValue(new TypeError('read failed'));fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Handlingen er bekreftet, men/);await act(async()=>{await router.navigate('/away')});await screen.findByText('Away')})
it('clears failed refresh feedback after successful toolbar refresh without another write',async()=>{await tree();choose();vi.mocked(fantasyApi.round).mockRejectedValue(new TypeError('read failed'));fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Handlingen er bekreftet, men/);vi.mocked(fantasyApi.round).mockResolvedValue(view);fireEvent.click(screen.getByLabelText('Oppdater Fantasy'));await waitFor(()=>expect(screen.queryByText('Prøv oppdatering igjen')).toBeNull());expect(fantasyApi.save).toHaveBeenCalledTimes(1)})
it('resolves a superseded unaccepted request with explicit draft rebase',async()=>{vi.mocked(fantasyApi.save).mockRejectedValueOnce(new TypeError('lost')).mockRejectedValueOnce(new ApiHttpError(409,'fantasy_revision_conflict','stale'));await tree();choose();fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Lagringen er ikke bekreftet/);const original=vi.mocked(fantasyApi.save).mock.calls[0]?.[3];const competing={...receipt,request_id:'00000000-0000-0000-0000-000000000055',captain:picks[1]??''};vi.mocked(fantasyApi.round).mockResolvedValue({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt:competing}]});await reconnect();fireEvent.click(screen.getByText('Avklar samme innsending'));const rebase=await screen.findByText('Behold valgene som nytt utkast');await waitFor(()=>expect(rebase).toHaveProperty('disabled',false));expect(vi.mocked(fantasyApi.save).mock.calls[1]?.[3]).toEqual(original);fireEvent.click(rebase);expect(fantasyApi.save).toHaveBeenCalledTimes(2);fireEvent.click(screen.getByText('Lagre firer og kaptein'));await waitFor(()=>expect(fantasyApi.save).toHaveBeenCalledTimes(3));const next=vi.mocked(fantasyApi.save).mock.calls[2]?.[3];expect(next?.expected_revision).toBe(1);expect(next?.request_id).not.toBe(original?.request_id);expect(next?.picks).toEqual(original?.picks);expect(next?.captain).toBe(original?.captain)})
it('keeps transient replay conflicts uncertain and reconciles an accepted original superseded by a newer lineup',async()=>{
 vi.mocked(fantasyApi.save).mockRejectedValueOnce(new TypeError('lost')).mockRejectedValueOnce(new ApiHttpError(409,'fantasy_conflict','retry')).mockResolvedValueOnce(receipt)
 await tree();choose();fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Lagringen er ikke bekreftet/)
 const original=vi.mocked(fantasyApi.save).mock.calls[0]?.[3]
 vi.mocked(fantasyApi.round).mockResolvedValue({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt:{...receipt,revision:2,request_id:'00000000-0000-0000-0000-000000000066'}}]})
 await reconnect();fireEvent.click(screen.getByText('Avklar samme innsending'));await screen.findByText(/Grunnlaget ble endret/)
 expect(screen.queryByText('Behold valgene som nytt utkast')).toBeNull()
 await waitFor(()=>expect(screen.getByText('Avklar samme innsending')).toHaveProperty('disabled',false))
 fireEvent.click(screen.getByText('Avklar samme innsending'));await screen.findByText(/Et nyere lag er allerede lagret/)
 expect(vi.mocked(fantasyApi.save).mock.calls.slice(1).every(call=>JSON.stringify(call[3])===JSON.stringify(original))).toBe(true)
})
it('can use current lineup after definitive nonacceptance without writing again',async()=>{
 vi.mocked(fantasyApi.save).mockRejectedValueOnce(new TypeError('lost')).mockRejectedValueOnce(new ApiHttpError(409,'fantasy_revision_conflict','stale'))
 const router=await tree();choose();fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Lagringen er ikke bekreftet/)
 vi.mocked(fantasyApi.round).mockResolvedValue({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt}]})
 fireEvent.click(screen.getByText('Avklar samme innsending'));const useCurrent=await screen.findByText('Bruk gjeldende lag');await waitFor(()=>expect(useCurrent).toHaveProperty('disabled',false));fireEvent.click(useCurrent)
 expect(screen.queryByText('Ulagrede endringer')).toBeNull();expect(fantasyApi.save).toHaveBeenCalledTimes(2)
 await act(async()=>{await router.navigate('/away')});await screen.findByText('Away')
})
it('does not let an older manual refresh replace a later uncertain write outcome',async()=>{
 await tree();choose()
 vi.mocked(fantasyApi.round).mockResolvedValue({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt}]})
 fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText('Handlingen er bekreftet. Visningen er oppdatert.')
 fireEvent.click(await screen.findByRole('button',{name:'Endre valg'}))
 await waitFor(()=>expect(screen.getByLabelText('Kaptein · doble poeng').closest('fieldset')).toHaveProperty('disabled',false))
 let finish:()=>void=()=>{}
 const held=new Promise<void>(resolve=>{finish=resolve})
 vi.spyOn(client,'invalidateQueries').mockImplementationOnce(()=>held)
 fireEvent.click(screen.getByLabelText('Oppdater Fantasy'))
 await waitFor(()=>expect(screen.getByLabelText('Kaptein · doble poeng').closest('fieldset')).toHaveProperty('disabled',false))
 fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[1]}})
 vi.mocked(fantasyApi.save).mockRejectedValueOnce(new TypeError('second response lost'))
 fireEvent.click(screen.getByText('Lagre firer og kaptein'));await screen.findByText(/Lagringen er ikke bekreftet/)
 await waitFor(()=>expect(screen.getByText('Avklar samme innsending')).toHaveProperty('disabled',false))
 const before=vi.mocked(fantasyApi.round).mock.calls.length
 await act(async()=>finish())
 await waitFor(()=>expect(vi.mocked(fantasyApi.round).mock.calls.length).toBeGreaterThan(before))
 expect(screen.queryByText('Handlingen er bekreftet. Visningen er oppdatert.')).toBeNull()
 expect(screen.getByText(/Kunne ikke hente en bekreftelse fra serveren/)).toBeTruthy()
 expect(fantasyApi.save).toHaveBeenCalledTimes(2)
})

it('shows a valid saved lineup compactly and retains edits across section switches',async()=>{
 vi.mocked(fantasyApi.round).mockResolvedValue({...view,selections:[{user_id:session.user_id,state:'draft',locked_at:null,receipt}]})
 const router=createMemoryRouter([{path:'/tournaments/:tournamentId/fantasy',element:<FantasyPage/>}],{initialEntries:[`/tournaments/${tournament}/fantasy`]})
 render(<QueryClientProvider client={client}><SessionHarness><ScoringGuardProvider><RouterProvider router={router}/></ScoringGuardProvider></SessionHarness></QueryClientProvider>)
 fireEvent.click(await screen.findByRole('button',{name:'Endre valg'}))
 expect(screen.getByRole('group',{name:/Velg fire spillere/})).toBe(document.activeElement)
 fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[1]}})
 fireEvent.click(screen.getByRole('button',{name:'Poengtavler'}))
 expect(screen.queryByRole('checkbox')).toBeNull()
 expect(screen.getByText(/Fantasy-valgene beholdes/)).toBeTruthy()
 fireEvent.click(screen.getByRole('button',{name:'Min firer'}))
 expect(screen.getByLabelText('Kaptein · doble poeng')).toHaveProperty('value',picks[1])
})
