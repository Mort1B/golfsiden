import { awardLabels, type AwardSlot, type AwardsView } from '../../api/roundAwards/types'
import { validAwardReason } from '../../api/roundAwards/decode'
import { recoveryKey, useAwards } from './context'
export function AwardEditor({view,slot,refreshing}:{view:AwardsView;slot:AwardSlot;refreshing:boolean}){
 const {records,update,send,pending}=useAwards(),round=view.round_id,type=slot.award_type
 const record=records[recoveryKey(round,type)]??{},draft=record.draft
 const input=draft??{player:slot.player_id,revision:slot.revision,reason:'',correction:false}
 const correction=view.correction_required||slot.revision!=='0'||input.player===null
 const stale=!!draft&&draft.revision!==slot.revision
 const disabled=pending||refreshing||!!record.uncertain||!!record.refreshFailed
 const valid=validAwardReason(input.reason.trim()) && input.player!==slot.player_id && (!correction||input.correction) && (input.player===null||view.eligible_players.some(p=>p.player_id===input.player))
 const patch=(values:Partial<typeof input>)=>update(round,type,{draft:{...input,...values},message:undefined})
 const name=(id:string|null)=>id===null?'Ingen vinner':view.eligible_players.find(p=>p.player_id===id)?.display_name??'Tidligere deltaker'
 return <fieldset className="award-editor">
 <legend>{awardLabels[type]} · valgfri −1</legend>
 <p>Gjeldende: {name(slot.player_id)} · revisjon {slot.revision}</p>
 {record.message&&<p role="status">{record.message}</p>}
 {record.receipt&&record.receipt.revision!==slot.revision&&<p>Din bekreftede kvittering gjelder en tidligere revisjon ({record.receipt.revision}). Gjeldende vinner vises over.</p>}
 {record.uncertain?<><p>Innsendingen er uavklart. Samme forespørsel brukes ved nytt forsøk.</p></>:<>
 <label>Vinner av {awardLabels[type]}<select disabled={disabled||stale||record.rejected} value={input.player??''} onChange={event=>patch({player:event.target.value||null})}><option value="">Ingen vinner / fjern utmerkelse</option>{view.eligible_players.map(player=><option key={player.player_id} value={player.player_id}>{player.display_name}</option>)}</select></label>
 <label>Begrunnelse for {awardLabels[type]}<textarea disabled={disabled||stale||record.rejected} value={input.reason} onChange={event=>patch({reason:event.target.value})}/></label>
 <p>Skriv en kort begrunnelse.</p>
 {input.reason&&!validAwardReason(input.reason.trim())&&<p role="alert">Skriv en kortere, gyldig begrunnelse.</p>}
 {correction&&<label className="award-ack"><input type="checkbox" disabled={disabled||stale||record.rejected} checked={input.correction} onChange={event=>patch({correction:event.target.checked})}/>Jeg bekrefter korrigeringen av {awardLabels[type]}{view.correction_required?' i en låst runde':''}.</label>}
 {(stale||record.rejected)&&<><p role="alert">Grunnlaget må gjennomgås på nytt. Kontroller gjeldende vinner før du beholder utkastet.</p><button type="button" disabled={disabled} onClick={()=>update(round,type,{draft:{...input,revision:slot.revision,correction:false},rejected:false,message:undefined})}>Behold utkast mot gjennomgått revisjon</button></>}
 <button type="button" disabled={disabled||stale||record.rejected||!draft||!valid} onClick={()=>void send(round,type)}>Lagre {awardLabels[type]}</button>
 {draft&&<button type="button" disabled={disabled} onClick={()=>update(round,type,{draft:undefined,rejected:false,message:undefined})}>Forkast {awardLabels[type]}-utkast</button>}
 </>}
 </fieldset>
}
