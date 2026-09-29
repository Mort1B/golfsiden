import { createContext, useContext } from 'react'
import type { PreparedSelection } from './eligibility'
interface PreparedState {
  prepared: PreparedSelection | null
  prepare: (target: PreparedSelection) => void
  available: () => PreparedSelection | null
}
export const PreparedContext = createContext<PreparedState>({ prepared: null, prepare: () => undefined, available: () => null })
export const usePreparedScore = () => useContext(PreparedContext)
