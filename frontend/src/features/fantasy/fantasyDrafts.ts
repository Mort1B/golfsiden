import { createContext, useContext } from 'react'
import type { Receipt, Save } from '../../api/fantasy'
import type { useFantasyAction } from './useFantasy'
export interface FantasyDraft { picks:string[]; captain:string; revision:number }
export interface FantasyRecovery { draft:FantasyDraft|null; uncertain:Save|null; receipt:Receipt|null; rejected?:Save|null }
export const emptyRecovery:FantasyRecovery={draft:null,uncertain:null,receipt:null}
export interface FantasyViewing {kind:'managers'|'golfers';round:string;detail:string|null;lineupRound:string}
export const FantasyDraftContext=createContext<{
  viewing:FantasyViewing
  setViewing:(patch:Partial<FantasyViewing>)=>void
  records:Record<string,FantasyRecovery>
  update:(round:string,patch:Partial<FantasyRecovery>)=>void
  action:ReturnType<typeof useFantasyAction>
}|null>(null)
export function useFantasyDrafts(){
  const value=useContext(FantasyDraftContext)
  if(!value)throw new Error('Fantasy drafts require their workspace owner')
  return value
}
