import { onlineManager } from '@tanstack/react-query'
import { useSyncExternalStore } from 'react'
export function useOnline(): boolean {
  return useSyncExternalStore(onlineManager.subscribe, () => onlineManager.isOnline())
}
