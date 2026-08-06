import { useEffect, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { BarChart3, ClipboardPen, LogIn, LogOut, Settings, Trophy, Users } from 'lucide-react'
import { Link, NavLink, Outlet } from 'react-router-dom'
import { api } from '../api/client'
import { useAuth } from '../features/auth/authContext'
import { useScoringGuard } from '../features/scoring/scoringGuardContext'

const baseNavItems = [
  { to: '/', label: 'Turnering', icon: Trophy },
  { to: '/score', label: 'Score', icon: ClipboardPen },
  { to: '/leaderboard', label: 'Resultater', icon: BarChart3 },
  { to: '/players', label: 'Spillere', icon: Users },
]

export function AppShell() {
  const queryClient = useQueryClient()
  const auth = useAuth()
  const scoringGuard = useScoringGuard()
  const [signOutError, setSignOutError] = useState<string | null>(null)
  const navItems = auth.session?.role === 'admin'
    ? [...baseNavItems, { to: '/admin', label: 'Admin', icon: Settings }]
    : baseNavItems

  useEffect(() => {
    const events = new EventSource(api.liveUrl, { withCredentials: true })
    const invalidate = () => void queryClient.invalidateQueries()
    for (const type of ['player', 'tournament', 'round', 'team', 'score']) {
      events.addEventListener(type, invalidate)
    }
    return () => events.close()
  }, [queryClient])

  return (
    <div className="app-shell">
      <aside className="session-bar" aria-label="Brukerkonto">
        {auth.session ? (
          <><span>{auth.session.display_name}</span><button type="button" aria-label="Logg ut" title={scoringGuard.blocked ? 'Fullfør scoreendringen før du logger ut' : 'Logg ut'} disabled={scoringGuard.blocked} onClick={() => {
            setSignOutError(null)
            void auth.signOut().catch((error: unknown) => {
              setSignOutError(error instanceof Error ? error.message : 'Kunne ikke logge ut')
            })
          }}><LogOut aria-hidden="true" /></button></>
        ) : (
          <Link to="/login" aria-label="Logg inn"><LogIn aria-hidden="true" /><span>Logg inn</span></Link>
        )}
      </aside>
      {signOutError && <p className="session-error" role="alert">{signOutError}</p>}
      <main className="main-content">
        <Outlet />
      </main>
      <nav className={`bottom-nav nav-count-${navItems.length}`} aria-label="Hovedmeny">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink key={to} to={to} end={to === '/'} className={({ isActive }) => isActive ? 'nav-link active' : 'nav-link'}>
            <Icon aria-hidden="true" size={21} strokeWidth={2} />
            <span>{label}</span>
          </NavLink>
        ))}
      </nav>
    </div>
  )
}
