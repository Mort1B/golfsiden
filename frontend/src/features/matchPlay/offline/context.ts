import { createContext, useContext, useSyncExternalStore } from 'react'
import type { MatchRuntime } from './runtime'
export const MatchQueueContext = createContext<MatchRuntime | null>(null)
export function useMatchQueue() {
  const runtime = useContext(MatchQueueContext)
  if (!runtime) throw new Error('MatchQueueProvider mangler')
  return { runtime, ...useSyncExternalStore(runtime.subscribe, runtime.current, runtime.current) }
}
