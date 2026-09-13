import { MatchPendingLink } from '../features/matchPlay/MatchPending'
import { MatchQueueProvider } from '../features/matchPlay/offline/MatchQueueProvider'
import { ScoreQueueProvider } from '../features/scoring/offline/ScoreQueueProvider'
import { PendingScoreLink } from '../features/scoring/offline/PendingScores'
import { useState } from 'react'
import { BarChart3, ClipboardPen, LogIn, LogOut, Trophy, UserRound } from 'lucide-react'
import { Link, NavLink, Outlet, useLocation } from 'react-router-dom'
import { useAuth } from '../features/auth/authContext'
import { useScoringGuard } from '../features/scoring/scoringGuardContext'

const baseNavItems = [
  { to: '/tournaments', label: 'Turnering', icon: Trophy },
  { to: '/score', label: 'Score', icon: ClipboardPen },
  { to: '/leaderboard', label: 'Resultater', icon: BarChart3 },
  { to: '/profile', label: 'Profil', icon: UserRound },
]

export function AppShell() {
  return <ScoreQueueProvider><MatchQueueProvider><PrivateShell /></MatchQueueProvider></ScoreQueueProvider>
}

function PrivateShell() {
  const auth = useAuth()
  const scoringGuard = useScoringGuard()
  const [signOutError, setSignOutError] = useState<string | null>(null)
  const path = useLocation().pathname
  const matchScoring = /\/matches(?:\/[^/]+\/score)?$/.test(path)
  const matchResults = path.includes('/match-results') || /\/matches\/[^/]+$/.test(path)
  const navItems = baseNavItems

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
        <PendingScoreLink /><MatchPendingLink />
        <Outlet />
      </main>
      <nav className={`bottom-nav nav-count-${navItems.length}`} aria-label="Hovedmeny">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink key={to} to={to} aria-current={to === '/score' && matchScoring || to === '/leaderboard' && matchResults ? 'page' : undefined} className={({ isActive }) => isActive || to === '/score' && matchScoring || to === '/leaderboard' && matchResults ? 'nav-link active' : 'nav-link'}>
            <Icon aria-hidden="true" size={21} strokeWidth={2} />
            <span>{label}</span>
          </NavLink>
        ))}
      </nav>
    </div>
  )
}
