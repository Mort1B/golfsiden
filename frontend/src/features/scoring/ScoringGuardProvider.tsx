import { useState, type ReactNode } from 'react'
import { ScoringGuardContext } from './scoringGuardContext'

export function ScoringGuardProvider({ children }: { children: ReactNode }) {
  const [blocked, setBlocked] = useState(false)
  return (
    <ScoringGuardContext.Provider value={{ blocked, setBlocked }}>
      {children}
    </ScoringGuardContext.Provider>
  )
}
