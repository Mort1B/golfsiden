import { createContext, useContext } from 'react'
import type { Receipt, Save } from '../../api/fantasy'
import type { useFantasyAction } from './useFantasy'
export interface FantasyDraft { picks:string[]; captain:string; revision:number }
export interface FantasyRecovery { draft:FantasyDraft|null; uncertain:Save|null; receipt:Receipt|null }
export const emptyRecovery:FantasyRecovery={draft:null,uncertain:null,receipt:null}
export const FantasyDraftContext=createContext<{
  records:Record<string,FantasyRecovery>
  update:(round:string,patch:Partial<FantasyRecovery>)=>void
  action:ReturnType<typeof useFantasyAction>
}|null>(null)
export function useFantasyDrafts(){
  const value=useContext(FantasyDraftContext)
  if(!value)throw new Error('Fantasy drafts require their workspace owner')
  return value
}
