import { useQuery, useQueryClient } from '@tanstack/react-query'
import { awardLabels } from '../../api/roundAwards/types'
import { useAuth } from '../auth/authContext'
import { LoadingState, ErrorState } from '../../ui/AsyncState'
import { awardQuery } from './query'
import { AwardEditor } from './AwardEditor'
import './awards.css'
const dateFormat=new Intl.DateTimeFormat('nb-NO',{dateStyle:'short',timeStyle:'short'})
export function RoundAwards({round,tournament,authorityRefreshing}:{round:string;tournament:string;authorityRefreshing:boolean}){
 const {session}=useAuth(),client=useQueryClient()
 const query=useQuery(awardQuery(client,session,tournament,round))
 const view=query.data
 return <section className="round-awards" aria-label="Valgfrie rundeutmerkelser">
 <h4>Valgfrie rundeutmerkelser</h4><p>Closest to pin og longest drive kan hver gi −1 på vinnerens nettoresultat. Begge gir −2. I Stableford gir hver +1 konkurransepoeng. Brutto, scorekort og Fantasy endres ikke. Ingen utmerkelser kreves.</p>
 {query.isPending?<LoadingState/>:query.error?<ErrorState error={query.error} onRetry={()=>void query.refetch()}/>:view&&<>
 {view.availability!=='available'?<p>Utmerkelser kan bare gis etter en fullført slagspill- eller Stableford-runde.</p>:view.eligible_players.length===0?<p>Ingen deltakere finnes i rundens bevarte spillerfelt.</p>:view.awards.map(slot=><AwardEditor key={slot.award_type} view={view} slot={slot} refreshing={authorityRefreshing||query.isFetching}/>)}
 <details><summary>Historikk for utmerkelsene ({view.history.length})</summary>{view.history.length===0?<p>Ingen utmerkelser er registrert.</p>:<ol>{view.history.map(receipt=><li key={receipt.request_id}><strong>{awardLabels[receipt.award_type]}</strong> · {receipt.player_id===null?'Fjernet':view.eligible_players.find(p=>p.player_id===receipt.player_id)?.display_name??'Tidligere deltaker'} · revisjon {receipt.revision}<p>{receipt.reason}</p><p>{receipt.correction?'Korrigert':'Registrert'} av administrator · {dateFormat.format(new Date(receipt.created_at))}</p></li>)}</ol>}</details>
 </>}
 </section>
}
