import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ChevronLeft, RefreshCw } from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import { api } from '../api/client'
import { fantasyApi } from '../api/fantasy'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { FantasySetup } from '../features/fantasy/FantasyAdmin'
import { FantasyBoards } from '../features/fantasy/FantasyBoards'
import { FantasyRound } from '../features/fantasy/FantasyRound'
import { FantasyRules } from '../features/fantasy/FantasyRules'
import { useFantasyAction, useFantasyQuery } from '../features/fantasy/useFantasy'
import { usePublishTournamentNavigation } from '../routing/tournamentNavigation'
import { ErrorState, LoadingState } from '../ui/AsyncState'
import '../features/fantasy/fantasy.css'
export function FantasyPage(){const {tournamentId=''}=useParams(),{session}=useAuth();return <FantasyWorkspace key={`${tournamentId}:${session?.user_id}:${session?.csrf_token}`} tournament={tournamentId}/>}
function FantasyWorkspace({tournament}:{tournament:string}){
  const {session}=useAuth(),user=session?.user_id??'',disconnected=useTournamentLive(tournament)
  const memberships=useQuery({queryKey:tournamentKeys.mine(user),queryFn:api.myTournaments,retry:false})
  const member=memberships.data?.find(m=>m.tournament.id===tournament),admin=member?.role==='admin',authorized=!!member&&!memberships.error
  const rounds=useQuery({queryKey:tournamentKeys.rounds(user,tournament),queryFn:()=>api.rounds(tournament),enabled:authorized,retry:false})
  const roster=useQuery({queryKey:tournamentKeys.players(user,tournament),queryFn:()=>api.tournamentPlayers(tournament),enabled:authorized,retry:false})
  const game=useFantasyQuery(tournament,['game'],signal=>fantasyApi.game(tournament,signal),authorized)
  const results=useFantasyQuery(tournament,['results'],signal=>fantasyApi.results(tournament,signal),authorized&&game.data?.enabled===true&&!game.error)
  const action=useFantasyAction(tournament),[selected,setSelected]=useState('')
  const selectedRound=rounds.data?.find(r=>r.id===selected)??rounds.data?.find(r=>r.status==='open')??rounds.data?.find(r=>r.status==='draft')??rounds.data?.at(-1)
  usePublishTournamentNavigation(authorized?{tournamentId:tournament}:null,!memberships.isFetching&&!rounds.isFetching,rounds.data)
  const error=memberships.error??rounds.error??roster.error??game.error
  const refresh=()=>{void memberships.refetch();void rounds.refetch();void roster.refetch();void action.refresh()}
  return <section className="page fantasy-page"><header className="detail-header"><Link className="back-button" to={`/tournaments/${tournament}`} aria-label="Tilbake til turneringen"><ChevronLeft/></Link><div><p className="brand">{member?.tournament.name??'Turnering'}</p><h1>Fantasy</h1></div><button type="button" className="fantasy-refresh" onClick={refresh} aria-label="Oppdater Fantasy"><RefreshCw aria-hidden="true"/></button></header>
    <p>Din firer, én kaptein. Alle runder teller.</p>{disconnected&&<p role="alert">Direkteforbindelsen er brutt. Private resultater skjules til forbindelsen er gjenopprettet. Oppdater ved behov.</p>}
    {error?<ErrorState error={error} onRetry={refresh}/>:memberships.isPending?<LoadingState/>:!member?<p role="alert">Du må være medlem av turneringen for å åpne Fantasy.</p>:game.isPending||rounds.isPending||roster.isPending?<LoadingState/>:<>
      {admin&&<FantasySetup tournament={tournament} game={game.data??null} rounds={rounds.data??[]} refreshing={game.isFetching||memberships.isFetching}/>}
      {!game.data?.enabled?<p>Fantasy er ikke aktivert for denne turneringen.</p>:<>
        {rounds.data?.length?<><label htmlFor="fantasy-round">Runde for Min firer</label><select id="fantasy-round" value={selectedRound?.id??''} onChange={e=>setSelected(e.target.value)}>{rounds.data.map(r=><option value={r.id} key={r.id}>Runde {r.round_number}: {r.name}</option>)}</select>{selectedRound&&<FantasyRound key={selectedRound.id} tournament={tournament} round={selectedRound} players={roster.data?.players??[]} rounds={rounds.data??[]} admin={admin} authorityRefreshing={memberships.isFetching||rounds.isFetching||roster.isFetching}/>}</>:<p>Ingen runder er opprettet ennå.</p>}
        {results.error?<ErrorState error={results.error} onRetry={()=>void results.refetch()}/>:!results.data?<LoadingState/>:<>{results.isFetching&&<p role="status">Oppdaterer poengtavlene …</p>}<FantasyBoards tournament={tournament} results={results.data}/></>}
      </>}
    </>}<FantasyRules/>
  </section>
}
