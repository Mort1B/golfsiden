import { createContext, useContext, useSyncExternalStore } from 'react'
import type { QueueRuntime } from './runtime'

export const ScoreQueueContext = createContext<QueueRuntime | null>(null)
export function useScoreQueue() {
  const runtime = useContext(ScoreQueueContext)
  if (!runtime) throw new Error('ScoreQueueProvider mangler')
  const snapshot = useSyncExternalStore(runtime.subscribe, runtime.current, runtime.current)
  return { runtime, ...snapshot }
}
