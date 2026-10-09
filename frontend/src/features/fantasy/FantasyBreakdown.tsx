import { fantasyApi, type GolferResult, type ManagerResult } from '../../api/fantasy'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import { categories, pointText, selectionLabels } from './format'
import { useFantasyDrafts } from './fantasyDrafts'
import { useFantasyQuery } from './useFantasy'
export function FantasyBreakdown({tournament,kind,id,names,roundId=''}:{roundId?:string;tournament:string;kind:'golfers'|'managers';id:string;names:Map<string,string>}){
  return kind==='golfers'?<GolferBreakdown tournament={tournament} id={id} roundId={roundId}/>:<ManagerBreakdown tournament={tournament} id={id} names={names} roundId={roundId}/>
}
function GolferBreakdown({tournament,id,roundId}:{tournament:string;id:string;roundId:string}){
  const {action}=useFantasyDrafts()
  const query=useFantasyQuery(tournament,['golfer',id],signal=>fantasyApi.golfer(tournament,id,signal))
  if(query.error)return <ErrorState error={query.error} onRetry={()=>void action.refresh()}/>
  if(!query.data)return <LoadingState/>
  return <section className="fantasy-section"><h3>{query.data.standing.display_name} · spillerpoeng</h3>{!roundId&&<p>{pointText(query.data.standing.points)} · uten kapteinmultiplikator</p>}{query.isFetching&&<p role="status">Oppdaterer poeng …</p>}{query.data.rounds.filter(item=>!roundId||item.round.round_id===roundId).map(({round,result})=><details key={round.round_id} open><summary>Runde {round.round_number}: {round.name} · {pointText(result.points)}</summary><GolferDetail result={result}/></details>)}</section>
}
function ManagerBreakdown({tournament,id,names,roundId}:{tournament:string;id:string;names:Map<string,string>;roundId:string}){
  const {action}=useFantasyDrafts()
  const query=useFantasyQuery(tournament,['manager',id],signal=>fantasyApi.manager(tournament,id,signal))
  if(query.error)return <ErrorState error={query.error} onRetry={()=>void action.refresh()}/>
  if(!query.data)return <LoadingState/>
  return <section className="fantasy-section"><h3>{query.data.standing.display_name} · Fantasy-lag</h3>{!roundId&&<p>{pointText(query.data.standing.points)} · alle runder</p>}{query.isFetching&&<p role="status">Oppdaterer poeng …</p>}{query.data.rounds.filter(item=>!roundId||item.round.round_id===roundId).map(({round,result})=><details key={round.round_id} open><summary>Runde {round.round_number}: {round.name} · {pointText(result.points)}</summary><ManagerDetail result={result} names={names}/></details>)}</section>
}
export function GolferDetail({result}:{result:GolferResult}){
  return <div className="fantasy-breakdown"><p>{result.settlement==='non_finish'?'Ikke fullført: registrerte hull beholdes, uspilt hull gir ingen straff eller plassering.':result.settlement==='stale_non_finish'?'Ikke-fullført må vurderes på nytt etter en scoreendring.':result.settlement==='confirmed'?'Scorekortet er bekreftet.':result.settlement==='unconfirmed'?'Resultatet venter på bekreftelse.':result.settlement==='withheld'?'Resultatet er skjult.':''}</p>
    {result.match_outcome&&<p>Matchresultat: {result.match_outcome==='win'?'Seier +3':result.match_outcome==='draw'?'Uavgjort +1':'Tap −1'}. Ingen hull- eller plasseringspoeng.</p>}
    {result.recorded_hole_points!==null&&<p>Registrerte hullpoeng: {result.recorded_hole_points}</p>}{result.placement_points!==null&&<p>Plasseringspoeng: {result.placement_points}</p>}
    {result.holes.length>0&&<ol className="fantasy-holes">{result.holes.map(h=><li key={h.hole_id}><strong>Hull {h.hole_number} · par {h.par}</strong><span>{h.category?categories[h.category]:'Ingen kategori'}{h.net_strokes!==null?` · netto ${h.net_strokes}`:''}</span><span>{pointText(h.points)}</span></li>)}</ol>}
  </div>
}
export function ManagerDetail({result,names}:{result:ManagerResult;names:Map<string,string>}){
  return <div className="fantasy-breakdown"><p>{selectionLabels[result.selection_state]}</p>{result.lineup?<><p>{result.lineup.origin==='carried_forward'?'Gjenbrukt fra tidligere låst runde':'Levert for denne runden'}</p><ul className="fantasy-contributions">{result.lineup.picks.map(p=>{const c=result.contributions.find(c=>c.player_id===p);return <li key={p}><strong>{names.get(p)??'Spiller'}{p===result.lineup?.captain?' · kaptein ×2':''}</strong>{c&&<><span>Grunnpoeng: {pointText(c.base_points)}</span><span>Bidrag ×{c.multiplier}: {pointText(c.points)}</span></>}</li>})}</ul></>:<p>{result.selection_state==='unlocked'?'Andre deltakeres valg er private fram til fristen.':'Ingen komplett firer er tilgjengelig for denne runden.'}</p>}</div>
}
