import { useCallback, useState, type ReactNode } from 'react'
import { useLocation } from 'react-router-dom'
import { useAuth } from '../features/auth/authContext'
import { TournamentNavigationContext, type TournamentNavigationSelection } from './tournamentNavigation'

interface State {
  userId: string | null
  routeKey: string
  published: boolean
  current: TournamentNavigationSelection | null
  remembered: TournamentNavigationSelection | null
}

export function TournamentNavigationProvider({ children }: { children: ReactNode }) {
  const userId = useAuth().session?.user_id ?? null
  const { key: routeKey, pathname } = useLocation()
  const [state, setState] = useState<State>({ userId, routeKey, published: false, current: null, remembered: null })
  // Clear synchronously before another identity or route can render old links.
  if (state.userId !== userId || state.routeKey !== routeKey) {
    setState({ userId, routeKey, published: false, current: null, remembered: state.userId === userId ? state.remembered : null })
  }
  const publish = useCallback((selection: TournamentNavigationSelection | null, validRoundIds?: readonly string[]) => {
    setState(previous => {
      // A delayed callback from an old route/account cannot replace new context.
      if (previous.userId !== userId || previous.routeKey !== routeKey) return previous
      // Omission keeps a same-tournament hint; null explicitly clears a round.
      const candidate = selection?.roundId === undefined && selection?.tournamentId === previous.remembered?.tournamentId
        ? previous.remembered?.roundId : selection?.roundId
      const roundId = candidate && (!validRoundIds || validRoundIds.includes(candidate)) ? candidate : undefined
      const next = selection ? { ...selection, roundId,
        scope: selection.scope ?? previous.remembered?.scope,
        metric: selection.metric ?? previous.remembered?.metric } : null
      if (previous.published && previous.current?.tournamentId === next?.tournamentId
        && previous.current?.roundId === next?.roundId && previous.current?.scope === next?.scope
        && previous.current?.metric === next?.metric) return previous
      return { ...previous, published: true, current: next, remembered: next }
    })
  }, [userId, routeKey])
  const path = pathname.replace(/\/+$/, '')
  const neutral = path === '/profile' || path === '/tournaments'
  const current = state.userId === userId && state.routeKey === routeKey
  const selection = current ? neutral ? state.remembered : state.current : null
  return <TournamentNavigationContext.Provider value={{ selection, pending: !neutral && (!current || !state.published), publish }}>{children}</TournamentNavigationContext.Provider>
}
