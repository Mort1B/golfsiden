import { useState } from 'react'
import { fantasyApi, type Game, type RoundView } from '../../api/fantasy'
import type { Round } from '../../api/types'
import { deadlineIso, fantasyDate, localDeadline } from './format'
import { useFantasyDrafts } from './fantasyDrafts'
export function FantasySetup({tournament,game,rounds,refreshing,selectedRound}:{tournament:string;game:Game|null;rounds:Round[];refreshing:boolean;selectedRound?:string}){
  const {action}=useFantasyDrafts(),allDraft=rounds.every(r=>r.status==='draft')
  return <section className="fantasy-section"><h2>Fantasy-oppsett</h2><p>Spillet må aktiveres før første golfrunde åpnes. Aktivering og deaktivering er sperret etter første valgfrist. Lagrede lag og historikk beholdes.</p><button disabled={action.pending||action.phase==='refresh_failed'||refreshing||!allDraft} onClick={()=>void action.execute(s=>fantasyApi.configure(tournament,!game?.enabled,s.csrf_token),undefined,undefined,{kind:'game',round:selectedRound})}>{action.pending?'Lagrer …':game?.enabled?'Deaktiver Fantasy':'Aktiver Fantasy'}</button>{!allDraft&&<p>Oppsettet kan ikke endres etter at en runde er åpnet.</p>}</section>
}
export function FantasyDeadline({tournament,round,view,refreshing}:{tournament:string;round:Round;view:RoundView;refreshing:boolean}){
  const {action}=useFantasyDrafts(),[draft,setDraft]=useState<string|null>(null),[invalid,setInvalid]=useState(false)
  const value=draft??localDeadline(view.window.deadline),closed=!!view.window.locked_at||!!view.window.opened_at
  return <section className="fantasy-section"><h3>Valgfrist · {round.name}</h3>{closed?<p>Fristen er låst og kan ikke flyttes eller fjernes.</p>:<form onSubmit={e=>{e.preventDefault();try{const deadline=deadlineIso(value);setInvalid(false);void action.execute(s=>fantasyApi.deadline(tournament,round.id,deadline,s.csrf_token),()=>setDraft(null),undefined,{kind:'round',round:round.id})}catch{setInvalid(true)}}}>
    <label htmlFor="fantasy-deadline">Tidligere frist (din lokale tid)</label><input id="fantasy-deadline" type="datetime-local" value={value} disabled={action.pending||action.phase==='refresh_failed'||refreshing} onChange={e=>{setDraft(e.target.value);setInvalid(false);action.clear()}}/>
    <p>Tomt felt bruker åpningen av golfrunden. Serveren avviser utløpte frister; en lukket frist kan ikke åpnes igjen.</p>{view.window.deadline&&<p>Publisert frist: {fantasyDate(view.window.deadline)}</p>}
    <button disabled={action.pending||action.phase==='refresh_failed'||refreshing}>{action.pending?'Lagrer …':'Lagre valgfrist'}</button>{invalid&&<p role="alert">Velg en gyldig lokal dato og tid.</p>}</form>}

  </section>
}
