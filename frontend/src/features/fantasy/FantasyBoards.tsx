import { useState } from 'react'
import { fantasyApi, type Results } from '../../api/fantasy'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import { FantasyBreakdown } from './FantasyBreakdown'
import { pointText } from './format'
import { useFantasyQuery } from './useFantasy'
export function FantasyBoards({tournament,results}:{tournament:string;results:Results}){
  const [kind,setKind]=useState<'managers'|'golfers'>('managers'),[round,setRound]=useState(''),[detail,setDetail]=useState<string|null>(null)
  const query=useFantasyQuery(tournament,['round-results',round],signal=>fantasyApi.roundResults(tournament,round,signal),!!round)
  const names=new Map(results.golfers.map(p=>[p.id,p.display_name]))
  const rows=round ? query.data ? kind==='managers'?query.data.managers.map(m=>({...m,id:m.user_id})):query.data.golfers.map(g=>({...g,id:g.player_id})) : [] : results[kind]
  return <section className="fantasy-section" aria-labelledby="fantasy-board-heading"><h2 id="fantasy-board-heading">Poengtavler</h2><div className="fantasy-tabs" role="group" aria-label="Velg poengtavle"><button aria-pressed={kind==='managers'} onClick={()=>{setKind('managers');setDetail(null)}}>Fantasy-lag</button><button aria-pressed={kind==='golfers'} onClick={()=>{setKind('golfers');setDetail(null)}}>Spillerpoeng</button></div><p>{kind==='managers'?'Fire spillere per lag. Kapteinen dobler hele sitt bidrag.':'Alle golfspillernes grunnpoeng, uavhengig av valg og kapteiner.'}</p>
    <label htmlFor="fantasy-board-round">Vis poeng for</label><select id="fantasy-board-round" value={round} onChange={e=>{setRound(e.target.value);setDetail(null)}}><option value="">Sammenlagt · alle runder</option>{results.rounds.map(r=><option value={r.round_id} key={r.round_id}>Runde {r.round_number}: {r.name}</option>)}</select>
    {round&&query.error?<ErrorState error={query.error} onRetry={()=>void query.refetch()}/>:round&&!query.data?<LoadingState/>:<><p>Delte poeng gir delt plassering. Avventende og skjulte poeng får ingen plassering.</p>{query.isFetching&&round&&<p role="status">Oppdaterer runden …</p>}{rows.length===0?<p>Ingen {kind==='managers'?'Fantasy-lag er påmeldt':'spillere er registrert'}.</p>:<ol className="fantasy-standings">{rows.map(row=><li key={row.id}><span className="fantasy-rank" aria-label={row.rank?`Plass ${row.rank}`:'Ingen plassering'}>{row.rank??'–'}</span><button onClick={()=>setDetail(detail===row.id?null:row.id)} aria-expanded={detail===row.id}><strong>{row.display_name}</strong><span>{pointText(row.points)}</span><small>Vis {kind==='managers'?'lag og bidrag':'hull og poeng'}</small></button>{!round&&'rounds' in row&&<ul className="fantasy-round-totals">{row.rounds.map(r=><li key={r.round_id}>R{results.rounds.find(item=>item.round_id===r.round_id)?.round_number??'?'}: {pointText(r.points)}</li>)}</ul>}</li>)}</ol>}
    {detail&&<div className="fantasy-detail"><button onClick={()=>setDetail(null)}>Lukk poengdetaljer</button><FantasyBreakdown key={`${kind}:${detail}`} tournament={tournament} kind={kind} id={detail} names={names}/></div>}</>}
  </section>
}
