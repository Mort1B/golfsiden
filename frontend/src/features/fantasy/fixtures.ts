import type { AuthSession } from '../../api/auth'
import type { Receipt, RoundView } from '../../api/fantasy'
import type { Round, TournamentPlayer } from '../../api/types'
export const id=(n:number)=>`00000000-0000-0000-0000-${String(n).padStart(12,'0')}`
export const tournament=id(1),roundId=id(2),user=id(3),picks=[id(10),id(11),id(12),id(13)]
export const session:AuthSession={user_id:user,username:'fantasy',display_name:'Fantasy manager',role:'player',player_id:null,csrf_token:'fixture-csrf',expires_at:'2099-01-01T12:00:00Z'}
export const players:TournamentPlayer[]=Array.from({length:5},(_,i)=>({player_id:id(10+i),tournament_id:tournament,display_name:`Spiller ${i+1} med et langt navn som må brytes på små skjermer`,player_active:true,tournament_handicap:18,seed:null,status:'active',created_at:'2026-01-01T12:00:00Z',updated_at:'2026-01-01T12:00:00Z'}))
export const receipt:Receipt={id:id(20),round_id:roundId,user_id:user,revision:1,request_id:id(21),expected_revision:0,picks,captain:picks[0]??'',origin:'submitted',source_round:null,accepted_at:'2026-01-01T12:00:00Z'}
export const round:Round={id:roundId,tournament_id:tournament,round_number:1,name:'Fantasy-runden',round_date:'2026-10-08',course_id:null,course_name:'Bane',tee_id:null,tee_name:'Gul',number_of_holes:18,status:'draft',handicap_enabled:true,handicap_allowance_percent:100,scoring_format:'individual_stroke_play',created_at:'2026-01-01T12:00:00Z',updated_at:'2026-01-01T12:00:00Z'}
export const view:RoundView={window:{round_id:roundId,deadline:null,opened_at:null,locked_at:null},entered:true,eligible_players:players.map(p=>p.player_id),selections:[],carry_forward_preview:null,carry_forward_eligible:false,selection_availability:'open'}
