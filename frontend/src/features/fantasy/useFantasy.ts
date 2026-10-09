import { useCallback, useLayoutEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { fantasyKeys } from '../../api/fantasy'
import { ApiHttpError } from '../../api/http'
import { loadPrivateResult } from '../../api/privateResults'
import { refreshFantasyTarget, type FantasyReadTarget } from './refreshFantasy'
import { useAuth } from '../auth/authContext'
export function fantasyError(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if ([401,403,404].includes(error.status)) return 'Du har ikke tilgang til disse Fantasy-dataene. Oppdater siden og kontroller innloggingen.'
    if (error.code === 'fantasy_closed') return 'Fristen er passert. Nye valg kan ikke lagres. Kontroller det låste laget.'
    if (error.code === 'fantasy_revision_conflict') return 'Denne innsendingen ble ikke lagret. Et annet lag er lagret fra en annen økt. Kontroller gjeldende lag og velg hva du vil beholde.'
    if (error.code === 'fantasy_conflict') return 'Grunnlaget ble endret. Oppdater og kontroller gjeldende valg før du prøver igjen.'
    if (error.code === 'fantasy_unavailable') return 'Handlingen er ikke tilgjengelig nå. Kontroller om spillet er aktivert og runden er åpnet eller låst.'
    if (error.status === 400) return 'Kontroller valgene og begrunnelsen. Serveren kunne ikke godta forespørselen.'
  }
  return 'Kunne ikke hente en bekreftelse fra serveren. Kontroller forbindelsen og oppdater.'
}
export function useFantasyQuery<T>(t: string, parts: string[], load: (signal: AbortSignal) => Promise<T>, enabled = true) {
  const { session } = useAuth(), client = useQueryClient(), user = session?.user_id ?? ''
  const key = fantasyKeys.query(user,t,...parts)
  return useQuery({ queryKey: key, retry: (count,error)=>count<2&&error instanceof ApiHttpError&&error.status===409&&error.code==='fantasy_conflict', retryDelay: 250, gcTime: 0, enabled: enabled && !!session,
    queryFn: ({signal}) => loadPrivateResult(client,{userId:user,tournamentId:t},key,signal,async () => {
      const current = () => { const cached=client.getQueryData<AuthSession|null>(authKeys.session); return !!session && cached?.user_id===session.user_id && cached.csrf_token===session.csrf_token }
      if (!current()) throw new Error('Sesjonen er endret. Oppdater siden.')
      const data=await load(signal)
      signal.throwIfAborted()
      if (!current()) throw new Error('Sesjonen er endret. Oppdater siden.')
      return data
    }),
  })
}
export function useFantasyAction(t: string) {
  const { session } = useAuth(), client=useQueryClient(), alive=useRef(false), busy=useRef(false)
  const acknowledged=useRef(false),generation=useRef(0),refreshGeneration=useRef(0)
  const required=useRef<FantasyReadTarget|null>(null)
  const [feedback,setFeedback]=useState<{text:string;error:boolean}|null>(null)
  const [pending,setPending]=useState(false)
  const [phase,setPhase]=useState<'idle'|'writing'|'write_failed'|'uncertain'|'refreshing'|'refresh_failed'|'success'>('idle')
  useLayoutEffect(()=>{alive.current=true;return()=>{alive.current=false}},[])
  const current=useCallback(()=>{const cached=client.getQueryData<AuthSession|null>(authKeys.session);return alive.current && !!session && cached?.user_id===session.user_id && cached.csrf_token===session.csrf_token},[client,session])
  const mutation=useMutation({mutationFn:(operation:()=>Promise<unknown>)=>operation(),retry:false,gcTime:0,networkMode:'always'})
  const readRefresh=useCallback(async():Promise<boolean>=>{
    if(!current()||!session||!navigator.onLine)return false
    const target=required.current
    const queryKey=fantasyKeys.root(session.user_id,t)
    const targets=client.getQueryCache().findAll({queryKey,type:'active'}).map(query=>query.queryKey)
    try {
      await client.invalidateQueries({queryKey},{throwOnError:true})
      if(target && current())await refreshFantasyTarget(client,t,session,target,current)
      const visible=[...targets,...client.getQueryCache().findAll({queryKey,type:'active'}).map(query=>query.queryKey)]
      return current() && visible.length>0 && visible.every(key=>{
        const state=client.getQueryState(key)
        return state?.status==='success' && state.data!==undefined && !state.isInvalidated && state.fetchStatus==='idle'
      })
    }catch{return false}
  },[client,current,session,t])
  const publishRefresh=useCallback((ok:boolean)=>{
    if(!current())return
    setPhase(ok?'success':'refresh_failed')
    setFeedback({error:!ok,text:ok?'Handlingen er bekreftet. Visningen er oppdatert.':'Handlingen er bekreftet, men visningen kunne ikke oppdateres. Prøv oppdatering igjen; innsendingen sendes ikke på nytt.'})
  },[current])
  const refresh=useCallback(async()=>{const version=generation.current,run=++refreshGeneration.current,wasAcknowledged=acknowledged.current;const ok=await readRefresh();if(wasAcknowledged&&version===generation.current&&run===refreshGeneration.current)publishRefresh(ok);return ok},[readRefresh,publishRefresh])
  const retryRefresh=async()=>{
    if(busy.current||!current())return
    busy.current=true;setPending(true)
    try{await refresh()}finally{busy.current=false;if(current())setPending(false)}
  }
  const execute=async<T,>(operation:(session:AuthSession)=>Promise<T>,success:(value:T)=>void=()=>{},failure?:((error:unknown)=>void),target?:FantasyReadTarget):Promise<void>=>{
    if(busy.current || phase==='refresh_failed' || !current() || !session) return
    if(!navigator.onLine){setFeedback({error:true,text:'Du er frakoblet. Fantasy-valg lagres bare på nett.'});return}
    generation.current++;acknowledged.current=false
    required.current=target??{kind:'game'}
    busy.current=true;setPending(true);setFeedback(null);setPhase('writing')
    try{
      let result:{value:T}|undefined
      try{
        await mutation.mutateAsync(async()=>{if(!current())throw new Error('Sesjonen er endret.');result={value:await operation(session)}})
      }catch(error){
        if(!current())return
        setPhase(error instanceof ApiHttpError&&error.status<500?'write_failed':'uncertain')
        setFeedback({error:true,text:fantasyError(error)});failure?.(error)
        required.current=null
        await refresh()
        return
      }
      if(!current()||!result)return
      acknowledged.current=true;setPhase('refreshing')
      success(result.value)
      await refresh()
    }finally{busy.current=false;mutation.reset();if(current())setPending(false)}
  }
  return {pending,phase,feedback,current,refresh,retryRefresh,execute,clear:()=>{if(phase!=='refresh_failed'){generation.current++;acknowledged.current=false;required.current=null;setFeedback(null);setPhase('idle')}}}
}
