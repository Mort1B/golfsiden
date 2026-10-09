import { useEffect } from 'react'
import { fantasyApi, type Results } from '../../api/fantasy'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import { FantasyBreakdown } from './FantasyBreakdown'
import { FantasyStandingRow } from './FantasyStandingRow'
import { useFantasyDrafts } from './fantasyDrafts'
import { pointText } from './format'
import { useFantasyQuery } from './useFantasy'

export function FantasyBoards({tournament,results,refreshing=false}:{tournament:string;results:Results;refreshing?:boolean}){
  const {viewing,setViewing,action}=useFantasyDrafts(),{kind,detail}=viewing
  const round=results.rounds.some(item=>item.round_id===viewing.round)?viewing.round:''
  const query=useFantasyQuery(tournament,['round-results',round],signal=>fantasyApi.roundResults(tournament,round,signal),!!round)
  const names=new Map(results.golfers.map(p=>[p.id,p.display_name]))
  const rows=round ? query.data ? kind==='managers'?query.data.managers.map(m=>({...m,id:m.user_id})):query.data.golfers.map(g=>({...g,id:g.player_id})) : [] : results[kind]
  // Fresh authorized inventories prune identifiers only. Missing/error/loading
  // projections never erase the user's viewing preference during reconnect.
  const ready=!refreshing&&(!round||!!query.data&&!query.error&&!query.isFetching)
  const validDetail=rows.some(row=>row.id===detail)
  useEffect(()=>{
    if(!ready)return
    if(round!==viewing.round)setViewing({round,detail:null})
    else if(detail&&!validDetail)setViewing({detail:null})
  },[ready,round,viewing.round,detail,validDetail,setViewing])
  return <section className="fantasy-section" aria-labelledby="fantasy-board-heading">
    <h2 id="fantasy-board-heading">Poengtavler</h2>
    <div className="fantasy-tabs" role="group" aria-label="Velg poengtavle">
      <button aria-pressed={kind==='managers'} onClick={()=>setViewing({kind:'managers',detail:null})}>Fantasy-lag</button>
      <button aria-pressed={kind==='golfers'} onClick={()=>setViewing({kind:'golfers',detail:null})}>Spillerpoeng</button>
    </div>
    <p>{kind==='managers'?'Fire spillere per lag. Kapteinen dobler hele sitt bidrag.':'Alle golfspillernes grunnpoeng, uavhengig av valg og kapteiner.'}</p>
    <label htmlFor="fantasy-board-round">Vis poeng for</label>
    <select id="fantasy-board-round" value={round} onChange={e=>setViewing({round:e.target.value})}>
      <option value="">Sammenlagt · alle runder</option>
      {results.rounds.map(r=><option value={r.round_id} key={r.round_id}>Runde {r.round_number}: {r.name}</option>)}
    </select>
    {round&&query.error?<ErrorState error={query.error} onRetry={()=>void action.refresh()}/>:round&&!query.data?<LoadingState/>:<>
      <p>Delte poeng gir delt plassering. Avventende og skjulte poeng får ingen plassering.</p>
      {query.isFetching&&round&&<p role="status">Oppdaterer runden …</p>}
      {rows.length===0?<p>Ingen {kind==='managers'?'Fantasy-lag er påmeldt':'spillere er registrert'}.</p>:<ol className="fantasy-standings">
        {rows.map(row=><FantasyStandingRow key={row.id} name={row.display_name} rank={row.rank} points={row.points} kind={kind}
          expanded={detail===row.id} onToggle={()=>setViewing({detail:detail===row.id?null:row.id})} onClose={()=>setViewing({detail:null})}
          totals={!round&&'rounds' in row&&<ul className="fantasy-round-totals">{row.rounds.map(r=><li key={r.round_id}>R{results.rounds.find(item=>item.round_id===r.round_id)?.round_number??'?'}: {pointText(r.points)}</li>)}</ul>}>
          {detail===row.id&&<FantasyBreakdown key={`${kind}:${row.id}:${round}`} tournament={tournament} kind={kind} id={row.id} names={names} roundId={round}/>}
        </FantasyStandingRow>)}
      </ol>}
    </>}
  </section>
}
