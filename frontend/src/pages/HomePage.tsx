import { ArrowRight, LogIn } from 'lucide-react'
import type { ReactNode } from 'react'
import { Link, Navigate } from 'react-router-dom'
import { useAuth } from '../features/auth/authContext'
import { ErrorState, LoadingState } from '../ui/AsyncState'

export function HomePage() {
  const auth = useAuth()
  if (auth.loading) return <StartState><LoadingState /></StartState>
  if (auth.error) return <StartState><ErrorState error={auth.error} onRetry={() => void auth.retry()} /></StartState>
  if (auth.session) return <Navigate replace to="/tournaments" />

  return (
    <main className="home-page">
      <section className="home-panel" aria-labelledby="home-heading">
        <p className="brand">Guttas Golf</p>
        <h1 id="home-heading">Start golfturen her</h1>
        <p>Opprett turneringen, planlegg rundene og inviter spillerne når alt er klart.</p>
        <div className="home-actions">
          <Link className="button primary" to="/create">Opprett turnering <ArrowRight aria-hidden="true" /></Link>
          <Link className="button secondary" to="/login?returnTo=%2Ftournaments"><LogIn aria-hidden="true" /> Logg inn</Link>
        </div>
      </section>
    </main>
  )
}

function StartState({ children }: { children: ReactNode }) {
  return <main className="home-page"><section className="home-panel"><p className="brand">Guttas Golf</p>{children}</section></main>
}
