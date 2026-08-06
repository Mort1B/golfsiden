import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { BarChart3, ClipboardPen, Settings, Trophy, Users } from 'lucide-react'
import { NavLink, Outlet } from 'react-router-dom'
import { api } from '../api/client'

const navItems = [
  { to: '/', label: 'Turnering', icon: Trophy },
  { to: '/score', label: 'Score', icon: ClipboardPen },
  { to: '/leaderboard', label: 'Resultater', icon: BarChart3 },
  { to: '/players', label: 'Spillere', icon: Users },
  { to: '/admin', label: 'Admin', icon: Settings },
]

export function AppShell() {
  const queryClient = useQueryClient()

  useEffect(() => {
    const events = new EventSource(api.liveUrl)
    const invalidate = () => void queryClient.invalidateQueries()
    for (const type of ['player', 'tournament', 'round', 'team', 'score']) {
      events.addEventListener(type, invalidate)
    }
    return () => events.close()
  }, [queryClient])

  return (
    <div className="app-shell">
      <main className="main-content">
        <Outlet />
      </main>
      <nav className="bottom-nav" aria-label="Hovedmeny">
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
