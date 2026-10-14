import { useEffect, useRef, useState } from 'react'
import { useQueries, useQueryClient } from '@tanstack/react-query'
import type { Round, TournamentPlayer } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { useFantasyDrafts } from './fantasyDrafts'
import { fantasyRoundQuery } from './fantasyRoundQuery'
import { openFantasyRound, validFantasyDraft } from './fantasyLineups'
import { fantasyDate } from './format'
import { FantasyCopyPreview } from './FantasyCopyPreview'

export function FantasyPresets({tournament,rounds,players,selected,refreshing}:{
  tournament:string;rounds:Round[];players:TournamentPlayer[];selected:string;refreshing:boolean
}) {
  const {session}=useAuth(),client=useQueryClient(),{records,update,action,batch,setViewing}=useFantasyDrafts()
  const queries=useQueries({queries:rounds.map(round=>fantasyRoundQuery(client,tournament,round.id,session))})
  const [clock,setClock]=useState(Date.now()),expired=useRef(new Set<string>())
  useEffect(()=>{const timer=setInterval(()=>setClock(Date.now()),1000);return()=>clearInterval(timer)},[])
  const refresh=action.refresh
  const due=queries.flatMap((query,index)=>query.data?.window.deadline&&Date.parse(query.data.window.deadline)<=clock&&!query.data.window.locked_at?[rounds[index]?.id??'']:[]).join(',')
  useEffect(()=>{if(due.split(',').some(id=>id&&!expired.current.has(id))){due.split(',').forEach(id=>expired.current.add(id));void refresh()}},[due,refresh])
  const names=new Map(players.map(player=>[player.player_id,player.display_name]))
  const rows=rounds.map((round,index)=>{
    const query=queries[index],view=query?.error?undefined:query?.data,record=records[round.id]
    const receipt=view?.selections.find(s=>s.user_id===session?.user_id)?.receipt
    const draft=record?.draft,choices=draft??receipt
    const fresh=!!view&&!query?.isFetching&&!refreshing
    const open=!!view&&openFantasyRound(view),revision=receipt?.revision??0
    const valid=!!view&&validFantasyDraft(draft??(receipt?{...receipt}:null),view)
    const unresolved=!!record?.uncertain||!!record?.rejected
    const stale=!!draft&&draft.revision!==revision
    const status=!view?(query?.error?'Kunne ikke hente runden':'Henter runden …'):!open?'Stengt / utilgjengelig':unresolved?'Avklar innsending':!choices?'Ikke forberedt':!valid||stale?'Ugyldig / må gjennomgås':draft?'Ulagret utkast':'Lagret'
    return {round,view,record,receipt,choices,fresh,open,revision,valid,unresolved,stale,status}
  })
  const busy=action.pending||batch.running||action.phase==='refresh_failed'||refreshing
  const ready=rows.filter(row=>row.fresh&&row.open&&row.valid&&!row.stale&&!row.unresolved&&row.record?.draft).sort((a,b)=>a.round.round_number-b.round.round_number)
  const drafts=rows.filter(row=>row.record?.draft)
  const source=rows.find(row=>row.round.id===selected)
  return <section className="fantasy-section" aria-labelledby="fantasy-presets-heading">
    <h2 id="fantasy-presets-heading">Planlegg alle runder</h2>
    <p>Forbered fireren før turneringen. Hver runde har egne valg og egen frist, også før golflagene er satt opp.</p>
    <ol className="fantasy-presets">{rows.map(row=><li key={row.round.id}>
      <div><h3>Runde {row.round.round_number}: {row.round.name}</h3><strong>{row.status}</strong>
      {row.view&&<p>{row.view.window.deadline?`Frist ${fantasyDate(row.view.window.deadline)}`:'Frist når golfrunden åpnes'}{row.view.window.locked_at?' · låst':''}</p>}
      {row.view&&row.choices&&<p>{row.choices.picks.map(p=>`${names.get(p)??'Tidligere spiller'}${p===row.choices?.captain?' (kaptein ×2)':''}`).join(', ')}</p>}
      {row.view&&row.record?.draft&&row.receipt&&<p>Erstatter lagret revisjon {row.receipt.revision} ved innsending.</p>}
      {row.view?.selection_availability==='insufficient_players'&&<p>Minst fire valgbare spillere kreves.</p>}
      {row.view?.selection_availability==='not_entered'&&<p>Meld deg på i Min firer først.</p>}
      </div><div className="fantasy-actions"><button type="button" onClick={()=>{setViewing({lineupRound:row.round.id});document.getElementById('fantasy-round')?.focus()}}>Se / endre runde {row.round.round_number}</button>{row.record?.draft&&!row.unresolved&&<button disabled={action.pending||batch.running} onClick={()=>update(row.round.id,{draft:null,rejected:null})}>Forkast utkast for runde {row.round.round_number}</button>}</div>
    </li>)}</ol>
    {queries.some(query=>query.error)&&<button onClick={()=>void action.refresh()}>Oppdater rundeoversikten</button>}
    {source&&<FantasyCopyPreview key={selected} source={source} rows={rows} busy={busy}/>}
    <p>Kontroller utkastene ovenfor. Lagringen skjer i runderekkefølge og stopper ved første problem. Bekreftede runder sendes ikke på nytt.</p>
    <button disabled={busy||!ready.length||ready.length!==drafts.length} onClick={()=>void batch.save(ready.map(row=>row.round.id))}>Lagre gjennomgåtte utkast ({ready.length})</button>
    {!!drafts.length&&ready.length!==drafts.length&&<p>Oppdater eller avklar ugyldige, stengte og uavklarte utkast før felles lagring. Hver runde kan fortsatt håndteres separat.</p>}
  </section>
}
