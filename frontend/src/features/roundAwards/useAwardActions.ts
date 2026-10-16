import { useLayoutEffect, useRef, useState, type MutableRefObject } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { awardsApi, awardsKey } from '../../api/roundAwards'
import type { AwardsView, AwardType, AwardCommand } from '../../api/roundAwards/types'
import { validAwardReason } from '../../api/roundAwards/decode'
import { ApiHttpError } from '../../api/http'
import { denyPrivateResults, isPrivateResultDenial } from '../../api/privateResults'
import { invalidateLiveQueries } from '../../api/liveInvalidation'
import { useAuth } from '../auth/authContext'
import { awardQuery, currentAwardSession } from './query'
import { recoveryKey, type AwardRecovery } from './context'
export function useAwardActions(tournament:string,records:MutableRefObject<Record<string,AwardRecovery>>,update:(round:string,type:AwardType,patch:AwardRecovery)=>void){
 const {session}=useAuth(),client=useQueryClient(),alive=useRef(false),busy=useRef(false)
 const [pending,setPending]=useState(false)
 useLayoutEffect(()=>{alive.current=true;return()=>{alive.current=false}},[])
 const current=()=>alive.current&&currentAwardSession(client,session)
 async function read(round:string):Promise<boolean>{
  if(!current()||!navigator.onLine)return false
  try{await client.fetchQuery(awardQuery(client,session,tournament,round));return current()}catch{return false}
 }
 async function refresh(round:string){
  if(busy.current||!current())return
  busy.current=true;setPending(true)
  try{const ok=await read(round);if(current())for(const type of ['closest_to_pin','longest_drive'] as const){const record=records.current[recoveryKey(round,type)];if(record?.receipt)update(round,type,{refreshFailed:!ok,message:ok?'Innsendingen er bekreftet. Gjeldende utmerkelser er oppdatert.':'Innsendingen er bekreftet. Visningen kunne ikke oppdateres; prøv kun oppdatering igjen.'})}}
  finally{busy.current=false;if(current())setPending(false)}
 }
 async function send(round:string,type:AwardType,replay=false){
  if(busy.current||!current()||!session)return
  const record=records.current[recoveryKey(round,type)]??{}
  if(!navigator.onLine){update(round,type,{message:'Du er frakoblet. Utmerkelser kan bare lagres på nett.'});return}
  if(record.refreshFailed||record.rejected&&!replay)return
  const key=awardsKey(session.user_id,round),state=client.getQueryState<AwardsView>(key),view=state?.data
  const slot=view?.awards.find(a=>a.award_type===type),draft=record.draft
  let command:AwardCommand|undefined=replay?record.uncertain:undefined
  if(!replay){
   if(record.uncertain||!draft||!slot||!view||state?.status!=='success'||state.isInvalidated||state.fetchStatus!=='idle'||view.availability!=='available'||draft.revision!==slot.revision||!validAwardReason(draft.reason.trim())||draft.player===slot.player_id||draft.player!==null&&!view.eligible_players.some(p=>p.player_id===draft.player)||(view.correction_required||slot.revision!=='0'||draft.player===null)&&!draft.correction)return
   command={version:1,request_id:crypto.randomUUID(),award_type:type,player_id:draft.player,expected_revision:draft.revision,reason:draft.reason.trim(),correction:draft.correction}
  }
  if(!command)return
  busy.current=true;setPending(true);update(round,type,{uncertain:command,message:'Sender …'})
  try{
   let receipt
   try{receipt=await awardsApi.write(round,command,session.csrf_token,session.user_id);if(receipt.actor_id!==session.user_id)throw new Error('Kvitteringen gjelder en annen administrator.')}catch(error){
    if(!current())return
    if(isPrivateResultDenial(error))denyPrivateResults(client,{userId:session.user_id,tournamentId:tournament,roundId:round},key,error)
    const definite=error instanceof ApiHttpError && (!replay && error.status===400 && error.code==='validation_error' || error.status===409 && ['round_award_revision_conflict','round_award_request_conflict','round_award_not_completed','round_award_unsupported_format','round_award_correction_required'].includes(error.code))
    update(round,type,definite?{uncertain:undefined,rejected:true,message:'Innsendingen ble ikke godtatt. Oppdater og gjennomgå gjeldende utmerkelse før en ny innsending.'}:{message:isPrivateResultDenial(error)?'Tilgangen er avvist. Utfallet av den opprinnelige innsendingen er fortsatt uavklart. Du kan oppgi lokal gjenoppretting og forlate siden.':'Bekreftelsen mangler. Avklar samme innsending før du gjør nye endringer.'})
    if(!isPrivateResultDenial(error))await read(round)
    return
   }
   if(!current())return
   update(round,type,{receipt,draft:undefined,uncertain:undefined,rejected:false,message:'Innsendingen er bekreftet. Oppdaterer …'})
   // Acknowledgement is recorded before reads; no read failure may replay the write.
   let ok=false
   try{await invalidateLiveQueries(client,session.user_id);ok=await read(round)}catch{ok=false}
   if(current())update(round,type,{refreshFailed:!ok,message:ok?'Innsendingen er bekreftet. Gjeldende utmerkelser er oppdatert.':'Innsendingen er bekreftet. Visningen kunne ikke oppdateres; prøv kun oppdatering igjen.'})
  }finally{busy.current=false;if(current())setPending(false)}
 }
 return {pending,send,refresh,isBusy:()=>busy.current}
}
