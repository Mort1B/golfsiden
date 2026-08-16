import { useQuery } from '@tanstack/react-query'
import { CalendarDays, ChevronRight, Flag } from 'lucide-react'
import { Link } from 'react-router-dom'
import { api } from '../api/client'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { StatusBadge } from '../ui/StatusBadge'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'

const formatDate = (date: string) => new Intl.DateTimeFormat('nb-NO', { day: 'numeric', month: 'short', year: 'numeric' }).format(new Date(`${date}T12:00:00`))

export function TournamentsPage() {
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const query = useQuery({ queryKey: tournamentKeys.mine(userId), queryFn: api.myTournaments, enabled: userId.length > 0 })
  return (
    <section className="page">
      <header className="page-header"><p className="brand">Guttas Golf</p><h1>Dine turneringer</h1></header>
      {query.isPending && <LoadingState />}
      {query.error && <ErrorState error={query.error} onRetry={() => void query.refetch()} />}
      {query.data?.length === 0 && <EmptyState>Du er ikke med i noen turneringer ennå.</EmptyState>}
      <div className="item-list">
        {query.data?.map(({ tournament, role }) => (
          <Link className="list-card" to={`/tournaments/${tournament.id}`} key={tournament.id}>
            <div className="card-topline"><StatusBadge status={tournament.status} /><span className="membership-role">{role === 'admin' ? 'Administrator' : role === 'scorer' ? 'Scorefører' : role === 'player' ? 'Spiller' : 'Tilskuer'}</span></div>
            <h2>{tournament.name}</h2>
            <p className="muted"><CalendarDays size={16} /> {formatDate(tournament.start_date)} – {formatDate(tournament.end_date)}</p>
            <p className="muted"><Flag size={15} /> {tournament.number_of_rounds} runder</p>
            <ChevronRight className="card-chevron" aria-hidden="true" />
          </Link>
        ))}
      </div>
    </section>
  )
}
