import { createContext, useContext, useSyncExternalStore } from 'react'
import type { ScoreDraftStore } from './store'
export const ScoreDraftContext = createContext<ScoreDraftStore | null>(null)
export function useScoreDrafts() {
  const store = useContext(ScoreDraftContext)
  if (!store) throw new Error('ScoreDraftContext mangler')
  const drafts = useSyncExternalStore(store.subscribe, store.current, store.current)
  return { store, drafts }
}
