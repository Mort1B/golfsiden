import { createContext, useContext } from 'react'

export interface ScoringGuardContextValue {
  blocked: boolean
  setBlocked: (blocked: boolean) => void
}

export const ScoringGuardContext = createContext<ScoringGuardContextValue | null>(null)

export function useScoringGuard(): ScoringGuardContextValue {
  const context = useContext(ScoringGuardContext)
  if (!context) throw new Error('useScoringGuard must be used inside ScoringGuardProvider')
  return context
}
