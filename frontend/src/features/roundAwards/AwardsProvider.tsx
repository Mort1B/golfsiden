import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { useBlocker } from 'react-router-dom'
import { awardLabels, type AwardType } from '../../api/roundAwards/types'
import { useScoringGuard } from '../scoring/scoringGuardContext'
import { AwardsContext, recoveryKey, type AwardRecovery } from './context'
import { useAwardActions } from './useAwardActions'
import './awards.css'
export function AwardsProvider({tournament,children}:{tournament:string;children:ReactNode}){
 const [records,setRecords]=useState<Record<string,AwardRecovery>>({}),ref=useRef(records)
 const update=useCallback((round:string,type:AwardType,patch:AwardRecovery)=>{const key=recoveryKey(round,type);ref.current={...ref.current,[key]:{...ref.current[key],...patch}};setRecords(ref.current)},[])
 const action=useAwardActions(tournament,ref,update)
 const blocked=action.pending||Object.values(records).some(r=>r.draft||r.uncertain)
 const blocker=useBlocker(({currentLocation,nextLocation})=>blocked&&currentLocation.pathname!==nextLocation.pathname)
 const {setBlocked}=useScoringGuard()
 useEffect(()=>{setBlocked(blocked);return()=>setBlocked(false)},[blocked,setBlocked])
 useEffect(()=>{if(blocker.state==='blocked')blocker.reset()},[blocker])
 useEffect(()=>{if(!blocked)return;const warn=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue=''};window.addEventListener('beforeunload',warn);return()=>window.removeEventListener('beforeunload',warn)},[blocked])
 return <AwardsContext value={{records,pending:action.pending,send:action.send,refresh:action.refresh,update:(round,type,patch)=>{if(!action.isBusy())update(round,type,patch)}}}>
 {(blocked||Object.values(records).some(r=>r.refreshFailed))&&<div className="award-recovery">
 {blocked&&<p role="status">Ulagrede utmerkelser beholdes i denne økten. Avklar innsendingen eller forkast utkast før du forlater siden.</p>}
 {!action.pending&&Object.values(records).some(r=>r.draft&&!r.uncertain)&&<button type="button" onClick={()=>{for(const key of Object.keys(records)){const record=records[key];if(record?.draft&&!record.uncertain){const split=key.lastIndexOf(':');const type=key.slice(split+1);if(type==='closest_to_pin'||type==='longest_drive')update(key.slice(0,split),type,{draft:undefined,rejected:false})}}}}>Forkast ulagrede utmerkelser</button>}
 {Object.entries(records).filter(([,r])=>r.receipt&&r.refreshFailed).map(([key,r])=><p role="status" key={key}>Innsending revisjon {r.receipt?.revision} er bekreftet. <button disabled={action.pending} onClick={()=>void action.refresh(r.receipt?.round_id??'')}>Prøv oppdatering igjen</button></p>)}
 {Object.entries(records).filter(([,r])=>r.uncertain).map(([key,r])=>{const command=r.uncertain;if(!command)return null;const round=key.slice(0,key.lastIndexOf(':'));return <section key={key} aria-label={`Uavklart ${awardLabels[command.award_type]}`}><p>Bekreftelsen for {awardLabels[command.award_type]} mangler. Samme innsending beholdes selv om rundedata ikke kan vises.</p>{r.message&&<p role="status">{r.message}</p>}<button type="button" disabled={action.pending} onClick={()=>void action.send(round,command.award_type,true)}>Avklar samme {awardLabels[command.award_type]}-innsending</button></section>})}
 </div>}
 {children}</AwardsContext>
}
