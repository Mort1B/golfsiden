import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { useBlocker } from 'react-router-dom'
import type { AuthSession } from '../../api/auth'
import type { FantasyReadTarget } from './refreshFantasy'
import { useScoringGuard } from '../scoring/scoringGuardContext'
import { emptyRecovery, FantasyDraftContext, useFantasyDrafts, type FantasyRecovery, type FantasyViewing } from './fantasyDrafts'
import { FantasyActionFeedback } from './FantasyActionFeedback'
import { useFantasyBatch } from './useFantasyBatch'
import { useFantasyAction } from './useFantasy'

// This owner outlives query-driven children. It retains only user input and own
// submission receipts, never roster, visibility, eligibility or result projections.
export function FantasyDraftProvider({tournament,children}:{tournament:string;children:ReactNode}){
  const [records,setRecords]=useState<Record<string,FantasyRecovery>>({})
  const [viewing,setPreferences]=useState<FantasyViewing>({section:'lineup',kind:'managers',round:'',detail:null,lineupRound:''})
  const setViewing=useCallback((patch:Partial<FantasyViewing>)=>setPreferences(previous=>({...previous,...patch})),[])
  const action=useFantasyAction(tournament)
  const recordsRef=useRef(records)
  const update=useCallback((round:string,patch:Partial<FantasyRecovery>)=>{
    recordsRef.current={...recordsRef.current,[round]:{...(recordsRef.current[round]??emptyRecovery),...patch}}
    setRecords(recordsRef.current)
  },[])
  const batch=useFantasyBatch(tournament,recordsRef,update,action)
  const execute=async<T,>(operation:(session:AuthSession)=>Promise<T>,success?:(value:T)=>void,failure?:(error:unknown)=>void,target?:FantasyReadTarget)=>{
    if(batch.isRunning())return 'cancelled' as const
    return action.execute(operation,success,failure,target)
  }
  const guardedAction={...action,pending:action.pending||batch.running,execute}
  const guardedUpdate=(round:string,patch:Partial<FantasyRecovery>)=>{if(!batch.isRunning())update(round,patch)}
  const blocked=batch.running||action.phase==='writing'||Object.values(records).some(record=>record.draft||record.uncertain)
  const blocker=useBlocker(blocked),{setBlocked}=useScoringGuard()
  useEffect(()=>{setBlocked(blocked);return()=>setBlocked(false)},[blocked,setBlocked])
  useEffect(()=>{if(blocker.state==='blocked')blocker.reset()},[blocker])
  useEffect(()=>{
    if(!blocked)return
    const warn=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue=''}
    window.addEventListener('beforeunload',warn)
    return()=>window.removeEventListener('beforeunload',warn)
  },[blocked])
  return <FantasyDraftContext value={{records,update:guardedUpdate,action:guardedAction,viewing,setViewing,batch}}>{children}</FantasyDraftContext>
}
export function FantasyRecoveryStatus(){
  const {records,update,action,viewing,setViewing,batch}=useFantasyDrafts()
  const blocked=batch.running||action.phase==='writing'||Object.values(records).some(record=>record.draft||record.uncertain)
  return <>
    {blocked&&<p role="status">Fantasy-valgene beholdes i denne økten. Avklar innsendingen eller forkast ulagrede valg før du forlater siden. Ikke lukk nettleseren.</p>}
    {!batch.running&&!action.pending&&Object.values(records).some(record=>record.draft&&!record.uncertain)&&<button type="button" onClick={()=>{for(const [round,record] of Object.entries(records))if(!record.uncertain)update(round,{draft:null,rejected:null})}}>Forkast ulagrede Fantasy-valg</button>}
    {action.phase==='refresh_failed'&&Object.entries(records).filter(([,record])=>record.receipt).map(([round,record])=><p key={round}>Kvittering beholdt: innsending revisjon {record.receipt?.revision} er bekreftet.</p>)}
    {viewing.section==='boards'&&Object.entries(records).filter(([,record])=>record.uncertain||record.rejected).map(([round])=><button key={round} onClick={()=>setViewing({section:'lineup',lineupRound:round})}>Se innsendingen i Min firer</button>)}
    {batch.total>0&&<p role="status">{batch.completed.length} av {batch.total} innsendinger bekreftet. {batch.message}</p>}
    <FantasyActionFeedback action={action}/>
  </>
}
