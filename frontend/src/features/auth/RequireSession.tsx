import type { ReactNode } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import { useAuth } from './authContext'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import { signInTarget } from './navigation'

interface RequireSessionProps {
  children: ReactNode
}

export function RequireSession({ children }: RequireSessionProps) {
  const auth = useAuth()
  const location = useLocation()
  if (auth.loading) return <RouteState><LoadingState /></RouteState>
  if (auth.error) return <RouteState><ErrorState error={auth.error} onRetry={() => void auth.retry()} /></RouteState>
  if (!auth.session) {
    return <Navigate replace to={signInTarget(location.pathname, location.search, location.hash)} />
  }
  return children
}

function RouteState({ children }: { children: ReactNode }) {
  return <section className="page"><header className="page-header"><p className="brand">Guttas Golf</p><h1>Tilgang</h1></header>{children}</section>
}
