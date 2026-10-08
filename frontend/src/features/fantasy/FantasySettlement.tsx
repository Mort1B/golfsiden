import { useState } from 'react'
import { fantasyApi, type OwnerKind, type RoundResult, type Source } from '../../api/fantasy'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import { fantasyDate } from './format'
import { useFantasyAction, useFantasyQuery } from './useFantasy'
export function FantasySettlement({tournament,results,refreshing}:{tournament:string;results:RoundResult;refreshing:boolean}){
  const [selected,setSelected]=useState('')
  const owners=new Map<string,{id:string;kind:OwnerKind;names:string[]}>()
  for(const g of results.golfers){if(g.points.state==='not_participating'||g.points.state==='not_started')continue;const kind:OwnerKind=g.team_id?'team':'player',id=g.team_id??g.player_id,key=`${kind}:${id}`;const old=owners.get(key);if(old)old.names.push(g.display_name);else owners.set(key,{id,kind,names:[g.display_name]})}
  const owner=owners.get(selected)
  if(results.round.sporting_status==='draft')return <p>Ikke-fullført kan registreres når golfrunden er åpnet.</p>
  return <section className="fantasy-section"><h3>Ikke fullført · Fantasy</h3><p>Beholder registrerte poeng uten plassering eller straff for uspilt hull. Dette endrer ikke golfscore eller golfrundens status. I lagrunder gjelder avgjørelsen begge partnere.</p><label htmlFor="fantasy-owner">Spiller eller lag</label><select id="fantasy-owner" value={selected} onChange={e=>setSelected(e.target.value)}><option value="">Velg scorekort</option>{[...owners.entries()].map(([key,o])=><option key={key} value={key}>{o.names.join(' / ')}</option>)}</select>{owner&&<SourceEditor key={`${results.round.round_id}:${selected}`} tournament={tournament} round={results.round.round_id} kind={owner.kind} owner={owner.id} locked={results.round.sporting_status==='locked'} refreshing={refreshing}/>}</section>
}
function SourceEditor({tournament,round,kind,owner,locked,refreshing}:{tournament:string;round:string;kind:OwnerKind;owner:string;locked:boolean;refreshing:boolean}){
  const action=useFantasyAction(tournament)
  const query=useFantasyQuery(tournament,['source',round,kind,owner],signal=>fantasyApi.source(tournament,round,kind,owner,signal))
  if(query.error)return <ErrorState error={query.error} onRetry={()=>void query.refetch()}/>
  if(!query.data)return <LoadingState/>
  return <>{(query.isFetching||refreshing)&&<p role="status">Oppdaterer scoregrunnlaget …</p>}<DispositionForm key={`${query.data.source_token}:${query.data.disposition?.id??''}`} tournament={tournament} source={query.data} locked={locked} action={action} refreshing={query.isFetching||refreshing}/>{action.feedback&&<p role={action.feedback.error?'alert':'status'}>{action.feedback.text}</p>}</>
}
function DispositionForm({tournament,source,locked,action,refreshing}:{tournament:string;source:Source;locked:boolean;action:ReturnType<typeof useFantasyAction>;refreshing:boolean}){
  const [reason,setReason]=useState(''),[reviewed,setReviewed]=useState(false)
  const correction=locked||source.disposition!==null,valid=reason.trim().length>0&&new TextEncoder().encode(reason).length<=500&&(!correction||reviewed)
  const save=(disposed:boolean)=>action.execute(s=>fantasyApi.dispose(tournament,source.round_id,source.owner_kind,source.owner_id,{expected_source_token:source.source_token,disposed,correction,reason},s.csrf_token),()=>{setReason('');setReviewed(false)})
  return <div className="fantasy-settlement-form">
    {source.disposition&&<div><p>{source.disposition.disposed?'Registrert som ikke fullført':'Ikke-fullført er trukket tilbake'} · {fantasyDate(source.disposition.created_at)}</p><p>Begrunnelse: {source.disposition.reason}</p><p>{source.disposition_current?'Avgjørelsen gjelder gjeldende scoregrunnlag.':'Scoregrunnlaget er endret. Avgjørelsen er foreldet; kontroller scorekortet og registrer en ny begrunnet avgjørelse.'}</p></div>}
    <p>Kontroller scorekortet før du bekrefter. Bekreftede kort og avgjorte matcher kan ikke overstyres her.</p><label htmlFor="fantasy-reason">Begrunnelse (maks. 500 byte)</label><textarea id="fantasy-reason" value={reason} disabled={action.pending||refreshing} onChange={e=>{setReason(e.target.value);action.clear()}}/>
    {new TextEncoder().encode(reason).length>500&&<p role="alert">Begrunnelsen er for lang.</p>}
    {correction&&<label className="fantasy-check"><input type="checkbox" checked={reviewed} disabled={action.pending||refreshing} onChange={e=>setReviewed(e.target.checked)}/>Jeg har kontrollert grunnlaget og vil registrere en sporbar korreksjon.</label>}
    <div className="fantasy-actions"><button disabled={action.pending||refreshing||!valid} onClick={()=>void save(true)}>{action.pending?'Lagrer …':correction?'Bekreft ikke fullført på nytt':'Registrer ikke fullført'}</button>{source.disposition?.disposed&&<button disabled={action.pending||refreshing||!valid} onClick={()=>void save(false)}>Trekk tilbake ikke-fullført</button>}</div>
  </div>
}
