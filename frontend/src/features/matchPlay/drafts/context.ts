import { createContext, useContext, useSyncExternalStore } from 'react'
import type { MatchNoteStore } from './store'
export const MatchNoteContext = createContext<MatchNoteStore | null>(null)
export function useMatchNoteDrafts() {
  const store = useContext(MatchNoteContext)
  if (!store) throw new Error('MatchNoteProvider mangler')
  const drafts = useSyncExternalStore(store.subscribe, store.current, store.current)
  return { store, drafts }
}
