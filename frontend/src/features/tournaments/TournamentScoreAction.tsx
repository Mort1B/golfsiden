import { useContext } from 'react'
import { ScoreResumeContext } from '../scoring/scoreResumeContext'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { api } from '../../api/client'
import { privateWorkspaceKeys } from '../../api/privateWorkspace'
import { matchApi, matchKeys } from '../../api/matchPlay'
import type { Round } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { loadPrivateResult } from '../../api/privateResults'
import { preferredScoreRound } from '../scoring/selection'
import { tournamentNavigationLinks } from '../../routing/tournamentNavigation'

export function TournamentScoreAction({tournament,rounds,ready}:{tournament:string;rounds:Round[];ready:boolean}) {
  const client=useQueryClient()
  const user=useAuth().session?.user_id??''
  const remembered=useContext(ScoreResumeContext)?.selection
  const candidates=rounds.filter(r=>r.status==='open'||r.status==='completed')
  const round=candidates.find(r=>remembered?.tournamentId===tournament&&r.id===remembered.roundId)??preferredScoreRound(candidates)
  const match=round?.scoring_format==='singles_match_play'
  const access=useQuery({queryKey:privateWorkspaceKeys.scoreAccess(user,round?.id??''),queryFn:()=>api.scoreAccess(round?.id??''),enabled:ready&&!!round&&!match,staleTime:0,refetchOnMount:'always',retry:false})
  const matchKey=matchKeys.list(user,round?.id??'')
  const matches=useQuery({queryKey:matchKey,queryFn:({signal})=>loadPrivateResult(client,{userId:user,roundId:round?.id??''},matchKey,signal,()=>matchApi.list(round?.id??'',signal)),enabled:ready&&!!round&&match,staleTime:0,refetchOnMount:'always',retry:false})
  const query=match?matches:access
  if(!ready)return <p role="status">Oppdaterer runder og scoretilgang …</p>
  if(!round)return <p>{rounds.length===0?'Opprett en runde før scoreføring kan starte.':rounds.some(r=>r.status==='draft')?'Scoreføring blir tilgjengelig når arrangøren åpner en runde.':'Rundene er låst. Åpne en runde nedenfor for å se scorekort og resultater.'}</p>
  if(query.error)return <div role="alert"><p>Kunne ikke kontrollere scoretilgangen. Prøv igjen før du fortsetter scoreføringen.</p><button className="retry-button" onClick={()=>void query.refetch()}>Kontroller scoretilgang igjen</button></div>
  if(!query.isFetchedAfterMount||query.isFetching||!query.isSuccess)return <p role="status">Kontrollerer scoretilgang …</p>
  const writable=match?!!matches.data?.writable_match_ids.length:!!access.data?.writable_owners.length
  if(!writable)return <p>Du har ikke scoretilgang i {round.name}. Åpne en runde nedenfor for å se scorekort, eller kontakt arrangøren hvis tilgangen mangler.</p>
  return <Link className="continue-scoring" to={tournamentNavigationLinks({tournamentId:tournament,roundId:round.id}).score}>Fortsett scoreføring<span>Runde {round.round_number}: {round.name}</span></Link>
}
