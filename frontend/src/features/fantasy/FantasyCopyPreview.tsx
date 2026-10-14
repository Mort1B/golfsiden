import { useEffect, useRef, useState } from 'react'
import type { Receipt, RoundView } from '../../api/fantasy'
import type { Round } from '../../api/types'
import { openFantasyRound, validFantasyDraft } from './fantasyLineups'
import { useFantasyDrafts, type FantasyDraft, type FantasyRecovery } from './fantasyDrafts'

interface CopyRow {
  round:Round;view:RoundView|undefined;record:FantasyRecovery|undefined;receipt:Receipt|null|undefined;
  choices:FantasyDraft|Receipt|null|undefined;fresh:boolean;open:boolean;revision:number;valid:boolean;unresolved:boolean;stale:boolean
}
interface Preview { source:string; destinations:{round:string;fingerprint:string;replace:boolean}[] }
// Only local input and identity/revision tokens are kept while the preview is open.
function fingerprint(row:CopyRow):string {
  return JSON.stringify([row.round.id,row.revision,row.record?.draft??null,row.record?.uncertain??null,row.record?.rejected??null,
    row.receipt?.id,row.choices])
}
export function FantasyCopyPreview({source,rows,busy}:{source:CopyRow;rows:CopyRow[];busy:boolean}) {
  const {update,action,batch}=useFantasyDrafts(),[preview,setPreview]=useState<Preview|null>(null)
  const trigger=useRef<HTMLButtonElement>(null),panel=useRef<HTMLDivElement>(null),previewOpen=!!preview
  useEffect(()=>{if(previewOpen)panel.current?.focus()},[previewOpen])
  const close=()=>{setPreview(null);trigger.current?.focus()}
  const fits=(row:CopyRow)=>!!row.view&&!!source.choices&&validFantasyDraft({...source.choices,revision:row.revision},row.view)
  const candidates=rows.filter(row=>row.round.id!==source.round.id&&row.fresh&&row.open&&!row.unresolved&&!row.stale&&fits(row))
  const enabled=source.fresh&&source.valid&&!source.unresolved&&!source.stale&&candidates.length>0
  const stale=!!preview&&(!source.fresh||!source.valid||source.unresolved||source.stale||preview.source!==fingerprint(source)||preview.destinations.some(destination=>{
    const row=rows.find(row=>row.round.id===destination.round)
    return !row||!row.fresh||!row.view||!openFantasyRound(row.view)||!fits(row)||row.unresolved||fingerprint(row)!==destination.fingerprint
  }))
  const apply=()=>{
    if(!preview||stale||busy||batch.isRunning()||!source.choices)return
    for(const destination of preview.destinations){
      const row=rows.find(row=>row.round.id===destination.round)
      if(!row||row.record?.draft&&!destination.replace)continue
      update(row.round.id,{draft:{picks:[...source.choices.picks],captain:source.choices.captain,revision:row.revision},receipt:null,rejected:null})
    }
    action.clear();close()
  }
  return <div>
    <button ref={trigger} disabled={busy||!enabled} onClick={()=>setPreview({source:fingerprint(source),destinations:candidates.map(row=>({round:row.round.id,fingerprint:fingerprint(row),replace:false}))})}>Kopier fireren til andre åpne runder</button>
    {preview&&<div ref={panel} tabIndex={-1} className="fantasy-notice" role="region" aria-label="Gjennomgå kopiering" onKeyDown={event=>{if(event.key==='Escape'&&!busy){event.preventDefault();close()}}}>
      <h3>Kopier fra runde {source.round.round_number}</h3><p>Dette lager lokale utkast. Gjeldende lag erstattes først når du lagrer. Stengte og uavklarte runder er utelatt.</p>
      <ul>{preview.destinations.map(destination=>{
        const row=rows.find(row=>row.round.id===destination.round)
        if(!row)return null
        return <li key={destination.round}>Runde {row.round.round_number}: {row.round.name} — {row.receipt?`erstatter lagret revisjon ${row.revision}`:'ingen lagret firer'}
          {row.record?.draft&&<label className="fantasy-check"><input type="checkbox" checked={destination.replace} disabled={busy||stale} onChange={event=>setPreview({...preview,destinations:preview.destinations.map(item=>item.round===destination.round?{...item,replace:event.target.checked}:item)})}/>Erstatt også mine ulagrede valg i runde {row.round.round_number}</label>}
        </li>
      })}</ul>
      {stale&&<p role="alert">Grunnlaget er endret. Åpne en ny forhåndsvisning og bekreft erstatning på nytt.</p>}
      <div className="fantasy-actions"><button disabled={busy||stale} onClick={apply}>Forbered valgte runder</button><button disabled={busy} onClick={close}>Avbryt kopiering</button></div>
    </div>}
  </div>
}
