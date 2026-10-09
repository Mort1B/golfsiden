import { useEffect, useState } from 'react'
import { fantasyApi, type RoundView, type Save } from '../../api/fantasy'
import { ApiHttpError } from '../../api/http'
import type { Round, TournamentPlayer } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { fantasyDate, selectionLabels } from './format'
import { emptyRecovery, useFantasyDrafts } from './fantasyDrafts'
interface Props { tournament:string; round:string; view:RoundView; players:TournamentPlayer[]; rounds:Round[]; refreshing:boolean }
export function MyFour({tournament,round,view,players,rounds,refreshing}:Props){
  const {session}=useAuth(), {records,update,action}=useFantasyDrafts()
  const own=view.selections.find(s=>s.user_id===session?.user_id), accepted=own?.receipt
  const {draft,uncertain,receipt,rejected}=records[round]??emptyRecovery
  const setDraft=(draft:typeof emptyRecovery.draft)=>update(round,{draft})
  const setUncertain=(uncertain:Save|null)=>update(round,{uncertain})
  const setReceipt=(receipt:typeof emptyRecovery.receipt)=>update(round,{receipt})
  const [clock,setClock]=useState(Date.now())
  useEffect(()=>{const timer=setInterval(()=>setClock(Date.now()),1000);return()=>clearInterval(timer)},[])
  const deadline=view.window.deadline, locallyDue=!!deadline && Date.parse(deadline)<=clock
  const refresh=action.refresh
  useEffect(()=>{if(locallyDue)void refresh()},[locallyDue,refresh])
  const picks=draft?.picks??accepted?.picks??[],captain=draft?.captain??accepted?.captain??''
  const acceptedValid=own?.state==='draft'&&!!accepted&&accepted.picks.every(p=>view.eligible_players.includes(p))
  const revision=accepted?.revision??0,stale=draft!==null && draft.revision!==revision
  const open=view.selection_availability==='open' && !view.window.locked_at
  const disabled=!!rejected||action.pending||action.phase==='refresh_failed'||refreshing||!!uncertain||!open
  const valid=picks.length===4&&picks.includes(captain)&&picks.every(p=>view.eligible_players.includes(p))
  const names=new Map(players.map(p=>[p.player_id,p.display_name]))
  const edit=(next:string[],cap:string)=>{action.clear();setReceipt(null);setDraft({picks:next,captain:cap,revision:draft?.revision??revision})}
  const submit=(body:Save)=>action.execute(s=>{setUncertain(body);return fantasyApi.save(tournament,round,s.user_id,body,s.csrf_token)},rec=>{setUncertain(null);setDraft(null);setReceipt(rec)},error=>{
    if(error instanceof ApiHttpError&&error.code==='fantasy_revision_conflict'){update(round,{uncertain:null,rejected:body});return}
    if(!(error instanceof ApiHttpError)||error.status>=500)setUncertain(body)
    else if(!uncertain||error.code==='fantasy_closed')setUncertain(null)
  },{kind:'round',round})
  return <section className="fantasy-section" aria-labelledby="my-four-heading"><h2 id="my-four-heading">Min firer</h2>
    <p>{view.window.locked_at?`Valgene ble låst ${fantasyDate(view.window.locked_at)}.`:deadline?`Frist ${fantasyDate(deadline)}, eller tidligere hvis golfrunden åpnes.`:'Valgene låses når golfrunden åpnes.'}</p>
    {locallyDue && open && <p role="status">Fristen ser ut til å være passert. Oppdaterer fra serveren; serverens klokke avgjør om nye valg godtas.</p>}
    {!view.entered && <><p>Meld deg på Fantasy for å levere et lag. Allerede låste runder etterfylles ikke.</p><button disabled={action.pending||refreshing} onClick={()=>void action.execute(s=>fantasyApi.enter(tournament,s.csrf_token),undefined,undefined,{kind:'round',round})}>Meld meg på Fantasy</button></>}
    {own && <p>{selectionLabels[own.state]}</p>}
    {view.selection_availability==='insufficient_players'&&<p>Minst fire valgbare spillere kreves. Ingen reservevalg opprettes automatisk.</p>}
    {accepted && <div className="fantasy-receipt"><strong>{accepted.origin==='carried_forward'?'Gjenbrukt lag':'Lagret lag'} · revisjon {accepted.revision}</strong><p>{accepted.picks.map(p=>`${names.get(p)??'Tidligere spiller'}${p===accepted.captain?' (kaptein ×2)':''}`).join(', ')}</p>{accepted.source_round&&<p>Gjenbrukt fra {rounds.find(r=>r.id===accepted.source_round)?.name??'tidligere låst runde'}. Spillerne får denne rundens poeng.</p>}<p>Bekreftet {fantasyDate(accepted.accepted_at)}</p></div>}
    {receipt && <p role="status">Innsending revisjon {receipt.revision} er bekreftet.{accepted && accepted.revision>receipt.revision?' Et nyere lag er allerede lagret. Gjeldende lag vises ovenfor.':''}</p>}
    {!view.window.locked_at && !acceptedValid && view.carry_forward_preview && <div className="fantasy-notice"><strong>Hvis du ikke leverer et nytt gyldig lag</strong><p>{view.carry_forward_preview.picks.map(p=>`${names.get(p)??'Tidligere spiller'}${p===view.carry_forward_preview?.captain?' (kaptein ×2)':''}`).join(', ')}</p><p>{view.carry_forward_eligible?'Dette laget kan gjenbrukes ved fristen. Valgbarheten kontrolleres på nytt da.':'Dette laget kan ikke gjenbrukes nå. Erstatt de ugyldige valgene før fristen.'}</p></div>}
    {open && !accepted && !view.carry_forward_preview && <p>Du har ingen tidligere firer å gjenbruke. Lever ditt første lag før fristen.</p>}
    {uncertain && <div role="alert" className="fantasy-notice"><p>Lagringen er ikke bekreftet. Laget kan være mottatt. Behold samme innsending til svaret er avklart; nye valg er sperret.</p><button disabled={action.pending||refreshing} onClick={()=>void submit(uncertain)}>Avklar samme innsending</button>{accepted?.request_id===uncertain.request_id && accepted.expected_revision===uncertain.expected_revision && accepted.captain===uncertain.captain && accepted.picks.length===uncertain.picks.length && accepted.picks.every((p,i)=>p===uncertain.picks[i]) && <button onClick={()=>{setUncertain(null);setDraft(null)}}>Bruk bekreftet lag fra serveren</button>}</div>}
    {rejected&&<div className="fantasy-notice" role="alert"><p>Den opprinnelige innsendingen ble ikke lagret. Gjeldende lag vises ovenfor. Du velger selv om de tidligere valgene skal beholdes.</p><button disabled={action.pending||refreshing} onClick={()=>{update(round,{draft:null,rejected:null});action.clear()}}>Bruk gjeldende lag</button>{open&&<button disabled={action.pending||refreshing} onClick={()=>{update(round,{draft:{picks:[...rejected.picks],captain:rejected.captain,revision},rejected:null});action.clear()}}>Behold valgene som nytt utkast</button>}</div>}
    {open && <form onSubmit={e=>{e.preventDefault();if(!disabled&&!stale&&valid)void submit({request_id:crypto.randomUUID(),expected_revision:revision,picks:[...picks],captain})}}>
      <fieldset disabled={disabled}><legend>Velg fire spillere ({picks.length}/4)</legend><p>Begge lagpartnere kan velges. Kapteinen dobler hele resultatet, også minuspoeng.</p><div className="fantasy-picks">{players.filter(p=>view.eligible_players.includes(p.player_id)||picks.includes(p.player_id)).map(p=><label key={p.player_id}><input type="checkbox" checked={picks.includes(p.player_id)} disabled={!picks.includes(p.player_id)&&picks.length===4} onChange={e=>{const next=e.target.checked?[...picks,p.player_id]:picks.filter(id=>id!==p.player_id);edit(next,next.includes(captain)?captain:'')}}/><span>{p.display_name}{!view.eligible_players.includes(p.player_id)?' · ikke valgbar – fjern dette valget':''}</span></label>)}</div>
      <label htmlFor="fantasy-captain">Kaptein · doble poeng</label><select id="fantasy-captain" value={captain} onChange={e=>edit(picks,e.target.value)}><option value="">Velg kaptein</option>{picks.map(id=><option key={id} value={id}>{names.get(id)??'Ikke lenger valgbar'}</option>)}</select></fieldset>
      {draft&&<p role="status">Ulagrede endringer</p>}{stale&&<p role="alert">Et annet lag er lagret siden du begynte å redigere. Hent gjeldende lag før du gjør nye valg.</p>}
      {draft&&<button type="button" disabled={action.pending||!!uncertain} onClick={()=>{setDraft(null);setReceipt(null);update(round,{rejected:null});action.clear()}}>Forkast utkast og bruk lagret lag</button>}
      <button type="submit" disabled={disabled||stale||!valid}>{action.pending?'Lagrer …':'Lagre firer og kaptein'}</button>
    </form>}
    {view.selection_availability==='closed'&&!accepted&&<p>Ingen endringer kan leveres. Et gyldig tidligere lag gjenbrukes automatisk; uten dette får runden 0 poeng.</p>}
  </section>
}
