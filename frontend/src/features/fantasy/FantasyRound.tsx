import { fantasyApi } from '../../api/fantasy'
import type { Round, TournamentPlayer } from '../../api/types'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import { useAuth } from '../auth/authContext'
import { FantasyDeadline } from './FantasyAdmin'
import { FantasySettlement } from './FantasySettlement'
import { MyFour } from './MyFour'
import { useFantasyQuery } from './useFantasy'
export function FantasyRound({tournament,round,players,rounds,admin,authorityRefreshing}:{tournament:string;round:Round;players:TournamentPlayer[];rounds:Round[];admin:boolean;authorityRefreshing:boolean}){
  const {session}=useAuth()
  const view=useFantasyQuery(tournament,['round',round.id],signal=>fantasyApi.round(tournament,round.id,session?.user_id??'',signal))
  const results=useFantasyQuery(tournament,['round-results',round.id],signal=>fantasyApi.roundResults(tournament,round.id,signal),admin)
  if(view.error)return <ErrorState error={view.error} onRetry={()=>void view.refetch()}/>
  if(!view.data)return <LoadingState/>
  return <><MyFour tournament={tournament} round={round.id} view={view.data} players={players} rounds={rounds} refreshing={view.isFetching||authorityRefreshing}/>{view.isFetching&&<p role="status">Vurderer frist og gjeldende lag på nytt …</p>}{admin&&<details className="fantasy-admin"><summary>Administrer denne Fantasy-runden</summary><FantasyDeadline tournament={tournament} round={round} view={view.data} refreshing={view.isFetching||authorityRefreshing}/>{results.error?<ErrorState error={results.error} onRetry={()=>void results.refetch()}/>:!results.data?<LoadingState/>:<FantasySettlement tournament={tournament} results={results.data} refreshing={results.isFetching||authorityRefreshing}/>}</details>}</>
}
