import { useEffect, useRef } from 'react'
import { useAuth } from '../auth/authContext'
import { useScoreQueue } from './offline/context'
import { useScoreDrafts } from './recovery/context'
import { cardProgress, type ProgressCard } from './cardProgress'
export function CardReview({card,tournament,disabled,review,onReview}:{card:ProgressCard;tournament:string;disabled:boolean;review:boolean;onReview:()=>void}) {
  const region=useRef<HTMLElement>(null),requested=useRef(false)
  useEffect(()=>{if(!review&&requested.current){requested.current=false;region.current?.focus()}},[review])
  const {session}=useAuth(),queue=useScoreQueue(),{drafts}=useScoreDrafts()
  const progress=cardProgress(card,session?.user_id??'',tournament,[...queue.refreshing.map(value=>value.item),...queue.items],drafts)
  return <section ref={region} tabIndex={-1} className="card-review" aria-label="Fremdrift på scorekortet">
    <p><strong>{progress.entered} av {progress.total} hull ført på kortet</strong></p>
    <p>{progress.verified} av {progress.total} hull kontrollert på serveren uten lokale endringer.</p>
    {progress.pending>0&&<p role="status">{progress.pending} hull har lokale endringer som venter på levering eller kontroll. Summer og bekreftelse bruker bare serverens scorekort.</p>}
    {'format'in card&&card.format==='four_ball_stroke_play'&&<p>Et hull trenger minst én numerisk partnerscore. To pickuper gir ingen tellende lagscore.</p>}
    {review&&<button type="button" disabled={disabled} onClick={()=>{requested.current=true;onReview()}}>{progress.entered===progress.total?'Se over scorekortet':'Kontroller manglende hull'}</button>}
  </section>
}
