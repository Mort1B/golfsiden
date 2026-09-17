import { createContext, useCallback, useContext, useEffect, useId } from 'react'

export interface ScoringGuardContextValue {
  blocked: boolean
  setBlocked: (blocked: boolean) => void
}

export const ScoringGuardContext = createContext<{ blocked: boolean; register: (id: string, blocked: boolean) => void } | null>(null)

export function useScoringGuard(): ScoringGuardContextValue {
  const context = useContext(ScoringGuardContext)
  if (!context) throw new Error('useScoringGuard must be used inside ScoringGuardProvider')
  const id = useId()
  const register = context.register
  const setBlocked = useCallback((blocked: boolean) => register(id, blocked), [id, register])
  useEffect(() => () => register(id, false), [id, register])
  return { blocked: context.blocked, setBlocked }
}
