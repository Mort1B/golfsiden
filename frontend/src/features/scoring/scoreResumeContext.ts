import { createContext, useContext } from 'react'
import type { ScoreOwner } from '../../api/scorecards'

export interface ScoreResumeSelection {
  tournamentId: string
  roundId: string
  owner: ScoreOwner
}

export const ScoreResumeContext = createContext<{
  selection: ScoreResumeSelection | null
  remember: (selection: ScoreResumeSelection) => void
} | null>(null)

export function useScoreResume() {
  const context = useContext(ScoreResumeContext)
  if (!context) throw new Error('ScoreResumeProvider is missing')
  return context
}
