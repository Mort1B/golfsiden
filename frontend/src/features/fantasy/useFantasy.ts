import { useCallback, useLayoutEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { authKeys, type AuthSession } from '../../api/auth'
import { fantasyKeys } from '../../api/fantasy'
import { ApiHttpError } from '../../api/http'
import { loadPrivateResult } from '../../api/privateResults'
import { useAuth } from '../auth/authContext'
export function fantasyError(error: unknown): string {
  if (error instanceof ApiHttpError) {
    if ([401,403,404].includes(error.status)) return 'Du har ikke tilgang til disse Fantasy-dataene. Oppdater siden og kontroller innloggingen.'
    if (error.code === 'fantasy_closed') return 'Fristen er passert. Nye valg kan ikke lagres. Kontroller det låste laget.'
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
  const [feedback,setFeedback]=useState<{text:string;error:boolean}|null>(null)
  const [pending,setPending]=useState(false)
  useLayoutEffect(()=>{alive.current=true;return()=>{alive.current=false}},[])
  const current=useCallback(()=>{const cached=client.getQueryData<AuthSession|null>(authKeys.session);return alive.current && !!session && cached?.user_id===session.user_id && cached.csrf_token===session.csrf_token},[client,session])
  const mutation=useMutation({mutationFn:(operation:()=>Promise<unknown>)=>operation(),retry:false,gcTime:0,networkMode:'always'})
  const refresh=useCallback(async()=>{if(current() && session) await client.invalidateQueries({queryKey:fantasyKeys.root(session.user_id,t)})},[client,current,session,t])
  const execute=async<T,>(operation:(session:AuthSession)=>Promise<T>,success:(value:T)=>void=()=>{},failure?:(error:unknown)=>void):Promise<void>=>{
    if(busy.current || !current() || !session) return
    if(!navigator.onLine){setFeedback({error:true,text:'Du er frakoblet. Fantasy-valg lagres bare på nett.'});return}
    busy.current=true;setPending(true);setFeedback(null)
    try{await mutation.mutateAsync(async()=>{if(!current())throw new Error('Sesjonen er endret.');const value=await operation(session);if(current())success(value)})
      if(!current())return
      await refresh()
      if(current())setFeedback({error:false,text:'Handlingen er bekreftet. Visningen er oppdatert.'})
    }catch(error){if(!current())return;setFeedback({error:true,text:fantasyError(error)});failure?.(error);await refresh()}
    finally{busy.current=false;mutation.reset();if(current())setPending(false)}
  }
  return {pending,feedback,current,refresh,execute,clear:()=>setFeedback(null)}
}
