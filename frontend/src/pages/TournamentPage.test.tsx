// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { beforeEach, afterEach, it, vi, expect } from 'vitest'
import { api } from '../api/client'
import { matchApi } from '../api/matchPlay'
import { authKeys } from '../api/auth'
import { ApiHttpError } from '../api/http'
import { AuthContext } from '../features/auth/authContext'
import { tournament, round, session } from '../features/tournaments/lifecycle/__tests__/fixtures'
import { TournamentPage } from './TournamentPage'
const live=vi.hoisted(()=>({disconnected:false}))
vi.mock('../features/live/useTournamentLive',()=>({useTournamentLive:()=>live.disconnected}))
vi.mock('../features/tournaments/TournamentPlayerSection',()=>({TournamentPlayerSection:()=>null}))
let client:QueryClient
beforeEach(()=>{
 live.disconnected=false
 client=new QueryClient({defaultOptions:{queries:{retry:false}}});client.setQueryData(authKeys.session,session)
 vi.spyOn(api,'tournament').mockResolvedValue(tournament);vi.spyOn(api,'rounds').mockResolvedValue([])
 vi.spyOn(api,'tournamentPlayers').mockResolvedValue({players:[],handicap_correction:{state:'editable'}})
 vi.spyOn(api,'myTournaments').mockResolvedValue([{tournament,role:'player',player_id:session.player_id}])
 vi.spyOn(api,'scoreAccess').mockResolvedValue({round_id:round.id,writable_owners:[{type:'player',id:session.player_id??''}]})
})
afterEach(()=>{cleanup();client.clear();vi.restoreAllMocks()})
function tree(activeSession=session){return (<QueryClientProvider client={client}><AuthContext value={{session:activeSession,loading:false,error:null,signIn:vi.fn(),signOut:vi.fn(),establishSession:vi.fn(),retry:vi.fn()}}><MemoryRouter initialEntries={[`/tournaments/${tournament.id}`]}><Routes><Route path="/tournaments/:tournamentId" element={<TournamentPage/>}/></Routes></MemoryRouter></AuthContext></QueryClientProvider>)}
function mount(){return render(tree())}
it('distinguishes pending rounds from a successfully empty inventory',async()=>{vi.mocked(api.rounds).mockImplementation(()=>new Promise(()=>{}));mount();await screen.findByRole('heading',{name:tournament.name});expect(screen.getByText('Laster …')).toBeTruthy();expect(screen.queryByText(/Ingen runder er opprettet ennå/)).toBeNull()})
it('renders an actionable empty state without querying scoring access',async()=>{mount();await screen.findByText(/Ingen runder er opprettet ennå/);expect(api.scoreAccess).not.toHaveBeenCalled();expect(screen.queryByRole('link',{name:/Fortsett scoreføring/})).toBeNull()})
it('retries a round error without exposing raw transport details',async()=>{vi.mocked(api.rounds).mockRejectedValueOnce(new Error('upstream internal detail'));mount();await screen.findByText('Kunne ikke hente rundene. Kontroller forbindelsen og prøv igjen.');expect(screen.queryByText('upstream internal detail')).toBeNull();fireEvent.click(screen.getByText('Prøv igjen'));await screen.findByText(/Ingen runder er opprettet ennå/)})
it('waits for fresh access then offers one contextual scoring resume link',async()=>{vi.mocked(api.rounds).mockResolvedValue([{...round,status:'open'}]);let finish:(value:Awaited<ReturnType<typeof api.scoreAccess>>)=>void=()=>{};vi.mocked(api.scoreAccess).mockImplementation(()=>new Promise(resolve=>{finish=resolve}));mount();await screen.findByText('Kontrollerer scoretilgang …');expect(screen.queryByRole('link',{name:/Fortsett scoreføring/})).toBeNull();finish({round_id:round.id,writable_owners:[{type:'player',id:session.player_id??''}]});const link=await screen.findByRole('link',{name:/Fortsett scoreføring/});expect(link.getAttribute('href')).toBe(`/score?tournament=${tournament.id}&round=${round.id}&resume=1`)})
it.each(['locked','draft'] as const)('explains %s instead of granting score access',async status=>{vi.mocked(api.rounds).mockResolvedValue([{...round,status}]);mount();await screen.findByRole('link',{name:new RegExp(round.name)});expect(api.scoreAccess).not.toHaveBeenCalled();expect(screen.queryByRole('link',{name:/Fortsett scoreføring/})).toBeNull()})
it('does not retain an entry link after fresh access is denied',async()=>{vi.mocked(api.rounds).mockResolvedValue([{...round,status:'open'}]);mount();await screen.findByRole('link',{name:/Fortsett scoreføring/});vi.mocked(api.scoreAccess).mockRejectedValue(new ApiHttpError(403,'forbidden','private'));await client.invalidateQueries();await waitFor(()=>expect(screen.queryByRole('link',{name:/Fortsett scoreføring/})).toBeNull());await screen.findByText(/Kunne ikke kontrollere scoretilgangen/)})
it('uses match authority rather than an ordinary scoring-owner list',async()=>{vi.mocked(api.rounds).mockResolvedValue([{...round,status:'open',scoring_format:'singles_match_play'}]);vi.spyOn(matchApi,'list').mockResolvedValue({round_id:round.id,matches:[],writable_match_ids:[]});mount();await screen.findByText(/Du har ikke scoretilgang/);expect(api.scoreAccess).not.toHaveBeenCalled();expect(matchApi.list).toHaveBeenCalled()})

it('hides continuation during disconnect and waits for fresh access after reconnect',async()=>{
 vi.mocked(api.rounds).mockResolvedValue([{...round,status:'open'}]);const view=mount();await screen.findByRole('link',{name:/Fortsett scoreføring/})
 live.disconnected=true;view.rerender(tree());expect(screen.queryByRole('link',{name:/Fortsett scoreføring/})).toBeNull()
 vi.mocked(api.scoreAccess).mockImplementation(()=>new Promise(()=>{}));live.disconnected=false;view.rerender(tree());await screen.findByText('Kontrollerer scoretilgang …');expect(screen.queryByRole('link',{name:/Fortsett scoreføring/})).toBeNull()
})
it('revalidates a replaced session even when the account has cached authority',async()=>{
 vi.mocked(api.rounds).mockResolvedValue([{...round,status:'open'}]);const view=mount();await screen.findByRole('link',{name:/Fortsett scoreføring/})
 vi.mocked(api.scoreAccess).mockImplementation(()=>new Promise(()=>{}));const replacement={...session,csrf_token:'replacement-session'};client.setQueryData(authKeys.session,replacement);view.rerender(tree(replacement));await screen.findByText('Kontrollerer scoretilgang …');expect(screen.queryByRole('link',{name:/Fortsett scoreføring/})).toBeNull()
})
