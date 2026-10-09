import { useCallback, useEffect, useState, type ReactNode } from 'react'
import { useBlocker } from 'react-router-dom'
import { useScoringGuard } from '../scoring/scoringGuardContext'
import { emptyRecovery, FantasyDraftContext, useFantasyDrafts, type FantasyRecovery } from './fantasyDrafts'
import { FantasyActionFeedback } from './FantasyActionFeedback'
import { useFantasyAction } from './useFantasy'

// This owner outlives query-driven children. It retains only user input and own
// submission receipts, never roster, visibility, eligibility or result projections.
export function FantasyDraftProvider({tournament,children}:{tournament:string;children:ReactNode}){
  const [records,setRecords]=useState<Record<string,FantasyRecovery>>({})
  const action=useFantasyAction(tournament)
  const update=useCallback((round:string,patch:Partial<FantasyRecovery>)=>setRecords(previous=>({...previous,[round]:{...(previous[round]??emptyRecovery),...patch}})),[])
  const blocked=action.pending||action.phase==='refresh_failed'||Object.values(records).some(record=>record.draft||record.uncertain)
  const blocker=useBlocker(blocked),{setBlocked}=useScoringGuard()
  useEffect(()=>{setBlocked(blocked);return()=>setBlocked(false)},[blocked,setBlocked])
  useEffect(()=>{if(blocker.state==='blocked')blocker.reset()},[blocker])
  useEffect(()=>{
    if(!blocked)return
    const warn=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue=''}
    window.addEventListener('beforeunload',warn)
    return()=>window.removeEventListener('beforeunload',warn)
  },[blocked])
  return <FantasyDraftContext value={{records,update,action}}>{children}</FantasyDraftContext>
}
export function FantasyRecoveryStatus(){
  const {records,update,action}=useFantasyDrafts()
  const blocked=action.pending||action.phase==='refresh_failed'||Object.values(records).some(record=>record.draft||record.uncertain)
  return <>
    {blocked&&<p role="status">Fantasy-valgene beholdes i denne økten. Avklar innsendingen eller forkast ulagrede valg før du forlater siden. Ikke lukk nettleseren.</p>}
    {!action.pending&&Object.values(records).some(record=>record.draft&&!record.uncertain)&&<button type="button" onClick={()=>{for(const [round,record] of Object.entries(records))if(!record.uncertain)update(round,{draft:null})}}>Forkast ulagrede Fantasy-valg</button>}
    {action.phase==='refresh_failed'&&Object.entries(records).filter(([,record])=>record.receipt).map(([round,record])=><p key={round}>Kvittering beholdt: innsending revisjon {record.receipt?.revision} er bekreftet.</p>)}
    <FantasyActionFeedback action={action}/>
  </>
}
