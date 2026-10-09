// @vitest-environment jsdom
import type { ReactNode } from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { authKeys, type AuthSession } from '../../api/auth'
import { api } from '../../api/client'
import { fantasyApi, type Results, type GolferResult } from '../../api/fantasy'
import { handleTournamentLiveSignal } from '../../api/liveInvalidation'
import { FantasyPage } from '../../pages/FantasyPage'
import { AuthContext } from '../auth/authContext'
import { ScoringGuardProvider } from '../scoring/ScoringGuardProvider'
import { tournament as trip } from '../tournaments/lifecycle/__tests__/fixtures'
import { id, picks, players, receipt, round, roundId, session, tournament, view } from './fixtures'
vi.mock('../live/useTournamentLive',()=>({useTournamentLive:()=>false}))
vi.mock('../../routing/tournamentNavigation',()=>({usePublishTournamentNavigation:()=>{}}))
let client:QueryClient
const summary={round_id:roundId,round_number:1,name:'Første runde',format:round.scoring_format,sporting_status:round.status,visibility:{mode:'full' as const},points:{state:'not_started' as const}}
const second={...summary,round_id:id(90),round_number:2,name:'Andre runde'}
const standings=players.map(p=>({id:p.player_id,display_name:p.display_name,rank:null,points:{state:'not_started' as const},rounds:[]}))
const results:Results={tournament_id:tournament,rules_version:1,revision:'test',rounds:[summary,second],golfers:standings,managers:[]}
const golfer:GolferResult={player_id:id(10),display_name:players[0]?.display_name??'',points:{state:'not_started'},rank:null,team_id:null,holes:[],recorded_hole_points:null,placement_points:null,settlement:null,match_outcome:null}
beforeEach(()=>{
 client=new QueryClient({defaultOptions:{queries:{retry:false}}});client.setQueryData(authKeys.session,session)
 vi.spyOn(api,'myTournaments').mockResolvedValue([{tournament:trip,role:'player',player_id:null}]);vi.spyOn(api,'rounds').mockResolvedValue([round]);vi.spyOn(api,'tournamentPlayers').mockResolvedValue({players,handicap_correction:{state:'editable'}})
 vi.spyOn(fantasyApi,'game').mockResolvedValue({tournament_id:tournament,enabled:true,rules_version:1});vi.spyOn(fantasyApi,'round').mockResolvedValue(view);vi.spyOn(fantasyApi,'results').mockResolvedValue(results)
 vi.spyOn(fantasyApi,'roundResults').mockResolvedValue({tournament_id:tournament,revision:'test',round:summary,golfers:players.map(p=>({...golfer,player_id:p.player_id,display_name:p.display_name})),managers:[]})
 const standing=standings[0];if(!standing)throw new Error('fixture')
 vi.spyOn(fantasyApi,'golfer').mockResolvedValue({tournament_id:tournament,revision:'test',standing,rounds:[{round:summary,result:golfer},{round:second,result:golfer}]})
})
afterEach(()=>{cleanup();client.clear();vi.restoreAllMocks()})
function SessionHarness({children}:{children:ReactNode}){const {data}=useQuery<AuthSession|null>({queryKey:authKeys.session,queryFn:async()=>session,enabled:false});return <AuthContext value={{session:data??null,loading:false,error:null,signIn:vi.fn(),establishSession:vi.fn(),signOut:vi.fn(),retry:vi.fn()}}>{children}</AuthContext>}
async function tree(){const router=createMemoryRouter([{path:'/tournaments/:tournamentId/fantasy',element:<FantasyPage/>}],{initialEntries:[`/tournaments/${tournament}/fantasy`]});render(<QueryClientProvider client={client}><SessionHarness><ScoringGuardProvider><RouterProvider router={router}/></ScoringGuardProvider></SessionHarness></QueryClientProvider>);fireEvent.click(await screen.findByText('Spillerpoeng'));return router}
async function open(){const button=await screen.findByRole('button',{name:new RegExp(golfer.display_name)});fireEvent.click(button);await screen.findByRole('heading',{name:`${golfer.display_name} · spillerpoeng`});return button}
it('opens breakdown inside its row and restores focus when closing',async()=>{await tree();const button=await open();const row=button.closest('li');if(!row)throw new Error('row');expect(within(row).getByRole('heading',{name:/spillerpoeng/})).toBeTruthy();fireEvent.click(within(row).getByText('Lukk poengdetaljer'));expect(button).toBe(document.activeElement)})
it('limits detail to the selected round',async()=>{await tree();fireEvent.change(screen.getByLabelText('Vis poeng for'),{target:{value:roundId}});await open();expect(screen.queryByText(/Runde 2: Andre runde ·/)).toBeNull();expect(screen.getByText(/Runde 1: Første runde ·/)).toBeTruthy()})
it('restores only viewing preferences through parent privacy clearing and reconnect',async()=>{await tree();fireEvent.change(screen.getByLabelText('Vis poeng for'),{target:{value:roundId}});await open();await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'error')});await waitFor(()=>expect(screen.queryByRole('heading',{name:/· spillerpoeng/})).toBeNull());await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'open')});await waitFor(()=>expect(screen.getByText('Spillerpoeng').getAttribute('aria-pressed')).toBe('true'));expect(screen.getByLabelText('Vis poeng for')).toHaveProperty('value',roundId);await screen.findByRole('heading',{name:/· spillerpoeng/})})
it('drops removed row/round identifiers only after fresh inventory and never shows old detail',async()=>{
 await tree();fireEvent.change(screen.getByLabelText('Vis poeng for'),{target:{value:roundId}});await open()
 await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'error')})
 vi.mocked(fantasyApi.results).mockResolvedValue({...results,rounds:[second],golfers:standings.slice(1)})
 await act(async()=>{await handleTournamentLiveSignal(client,session.user_id,'open')})
 await waitFor(()=>expect(screen.getByLabelText('Vis poeng for')).toHaveProperty('value',''))
 expect(screen.queryByRole('heading',{name:/· spillerpoeng/})).toBeNull()
})
it.each(['csrf','account'])('clears viewing context after %s replacement',async mode=>{
 await tree();fireEvent.change(screen.getByLabelText('Vis poeng for'),{target:{value:roundId}});await open()
 await act(async()=>client.setQueryData(authKeys.session,{...session,user_id:mode==='account'?id(99):session.user_id,csrf_token:'new-session'}))
 await waitFor(()=>expect(screen.getByText('Fantasy-lag').getAttribute('aria-pressed')).toBe('true'))
 expect(screen.getByLabelText('Vis poeng for')).toHaveProperty('value','');expect(screen.queryByRole('heading',{name:/· spillerpoeng/})).toBeNull()
})

it('clears acknowledged failure after a successful inline detail retry without another write',async()=>{
 await tree();await open()
 const detail=await vi.mocked(fantasyApi.golfer).mock.results[0]?.value
 if(!detail)throw new Error('detail fixture')
 vi.spyOn(fantasyApi,'save').mockResolvedValue(receipt)
 for(const p of players.slice(0,4))fireEvent.click(screen.getByRole('checkbox',{name:p.display_name}))
 fireEvent.change(screen.getByLabelText('Kaptein · doble poeng'),{target:{value:picks[0]}})
 vi.mocked(fantasyApi.golfer).mockRejectedValue(new TypeError('detail read failed'))
 fireEvent.click(screen.getByText('Lagre firer og kaptein'))
 await screen.findByText(/Handlingen er bekreftet, men visningen/)
 vi.mocked(fantasyApi.golfer).mockResolvedValue(detail)
 fireEvent.click(within(screen.getByRole('region',{name:/poengdetaljer/})).getByText('Prøv igjen'))
 await screen.findByText('Handlingen er bekreftet. Visningen er oppdatert.')
 expect(fantasyApi.save).toHaveBeenCalledTimes(1)
 expect(screen.queryByText('Prøv oppdatering igjen')).toBeNull()
})
