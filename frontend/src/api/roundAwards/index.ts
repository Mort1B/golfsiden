import { jsonRequest, requestDecoded } from '../http'
import { privateWorkspaceKeys } from '../privateWorkspace'
import { decodeAwardReceipt, decodeAwardsView } from './decode'
import type { AwardCommand } from './types'
import { invalidData } from '../decoder'
export const awardsKey = (user: string, round: string) => [...privateWorkspaceKeys.user(user), 'rounds', round, 'awards'] as const
export const awardsApi = {
  read: (round: string, tournament: string, signal?: AbortSignal) => requestDecoded(`/api/rounds/${round}/awards`, value=>decodeAwardsView(value,round,tournament), { signal }),
  write: (round: string, command: AwardCommand, csrf: string, actor: string) => requestDecoded(`/api/rounds/${round}/awards`, value=>{
    const receipt=decodeAwardReceipt(value)
    if(receipt.actor_id!==actor || receipt.round_id!==round || receipt.request_id!==command.request_id || receipt.award_type!==command.award_type || receipt.player_id!==command.player_id || receipt.reason!==command.reason || receipt.correction!==command.correction || BigInt(receipt.revision)!==BigInt(command.expected_revision)+1n) invalidData('rundeutmerkelser','receipt identity')
    return receipt
  }, jsonRequest('PUT',command,csrf)),
}
