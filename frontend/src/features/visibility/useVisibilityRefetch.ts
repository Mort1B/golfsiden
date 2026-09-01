import { useEffect, useRef } from 'react'
import type { ScoreVisibility } from '../../api/visibility'
import { visibilityRefetchDelay } from '../../api/visibility'

export function useVisibilityRefetch(
  visibility: ScoreVisibility | undefined,
  refetch: () => Promise<unknown>,
): void {
  const refetchRef = useRef(refetch)
  refetchRef.current = refetch

  useEffect(() => {
    if (visibility === undefined) return
    const delay = visibilityRefetchDelay(visibility)
    if (delay === null) return
    const timer = window.setTimeout(() => void refetchRef.current(), delay)
    return () => window.clearTimeout(timer)
  }, [visibility])
}
