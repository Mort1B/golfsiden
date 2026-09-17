import { useCallback, useState, type ReactNode } from 'react'
import { ScoringGuardContext } from './scoringGuardContext'

export function ScoringGuardProvider({ children }: { children: ReactNode }) {
  const [owners, setOwners] = useState<ReadonlySet<string>>(new Set())
  const register = useCallback((id: string, blocked: boolean) => {
    setOwners(previous => {
      if (previous.has(id) === blocked) return previous
      const next = new Set(previous)
      if (blocked) next.add(id); else next.delete(id)
      return next
    })
  }, [])
  return (
    <ScoringGuardContext.Provider value={{ blocked: owners.size > 0, register }}>
      {children}
    </ScoringGuardContext.Provider>
  )
}
