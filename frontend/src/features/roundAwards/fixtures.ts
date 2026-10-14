import type { AwardsView, AwardReceipt } from '../../api/roundAwards/types'
import type { AuthSession } from '../../api/auth'
export const id=(n:number)=>`00000000-0000-0000-0000-${String(n).padStart(12,'0')}`
export const tournament=id(1),round=id(2),player=id(3)
export const session:AuthSession={user_id:id(4),player_id:player,username:'admin',role:'admin',csrf_token:'token',display_name:'Admin',expires_at:'2027-01-01T00:00:00Z'}
export const view:AwardsView={round_id:round,tournament_id:tournament,availability:'available',correction_required:false,eligible_players:[{player_id:player,display_name:'Spiller med langt navn'},{player_id:id(5),display_name:'Partner'}],awards:[{award_type:'closest_to_pin',player_id:null,revision:'0'},{award_type:'longest_drive',player_id:null,revision:'0'}],history:[]}
export const receipt:AwardReceipt={request_id:id(6),round_id:round,award_type:'closest_to_pin',player_id:player,previous_player_id:null,revision:'1',reason:'Målt',correction:false,actor_id:session.user_id,created_at:'2026-10-14T10:00:00Z'}
