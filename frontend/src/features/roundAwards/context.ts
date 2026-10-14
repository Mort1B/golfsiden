import { createContext, useContext } from 'react'
import type { AwardCommand, AwardReceipt, AwardType } from '../../api/roundAwards/types'
export interface AwardDraft { player: string | null; revision: string; reason: string; correction: boolean }
export interface AwardRecovery { draft?: AwardDraft; uncertain?: AwardCommand; rejected?: boolean; receipt?: AwardReceipt; refreshFailed?: boolean; message?: string }
export const recoveryKey = (round: string, type: AwardType): string => `${round}:${type}`
export interface AwardsContextValue {
 records: Record<string,AwardRecovery>; pending: boolean
 update: (round:string,type:AwardType,patch:AwardRecovery)=>void
 send: (round:string,type:AwardType,replay?:boolean)=>Promise<void>
 refresh: (round:string)=>Promise<void>
}
export const AwardsContext=createContext<AwardsContextValue|null>(null)
export function useAwards(){const context=useContext(AwardsContext);if(!context)throw new Error('AwardsProvider required');return context}
