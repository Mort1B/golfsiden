import { useId, useRef, type ReactNode } from 'react'
import type { Points } from '../../api/fantasy'
import { pointText } from './format'

export function FantasyStandingRow({name,rank,points,expanded,onToggle,onClose,kind,totals,children}:{
  name:string;rank:number|null;points:Points;expanded:boolean;onToggle:()=>void;onClose:()=>void;
  kind:'managers'|'golfers';totals:ReactNode;children:ReactNode
}) {
  const control=useRef<HTMLButtonElement>(null),detailId=useId()
  const close=()=>{onClose();control.current?.focus()}
  return <li>
    <span className="fantasy-rank" aria-label={rank?`Plass ${rank}`:'Ingen plassering'}>{rank??'–'}</span>
    <button ref={control} onClick={onToggle} aria-expanded={expanded} aria-controls={expanded?detailId:undefined}>
      <strong>{name}</strong><span>{pointText(points)}</span><small>Vis {kind==='managers'?'lag og bidrag':'hull og poeng'}</small>
    </button>
    {totals}
    {expanded&&<div id={detailId} className="fantasy-detail" role="region" aria-label={`${name} · poengdetaljer`} onKeyDown={event=>{if(event.key==='Escape'){event.stopPropagation();close()}}}>
      <button onClick={close}>Lukk poengdetaljer</button>{children}
    </div>}
  </li>
}
