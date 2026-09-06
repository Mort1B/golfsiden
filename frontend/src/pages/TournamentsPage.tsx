import { useQuery } from '@tanstack/react-query'
import { CalendarDays, ChevronRight, Flag } from 'lucide-react'
import { Link, useSearchParams } from 'react-router-dom'
import { api } from '../api/client'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { StatusBadge } from '../ui/StatusBadge'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { TOURNAMENT_LIST_VIEWS, matchesTournamentView, tournamentListView, tournamentViewSearch } from '../features/tournaments/tournamentListView'
import '../features/tournaments/tournamentList.css'

const formatDate = (date: string) => new Intl.DateTimeFormat('nb-NO', { day: 'numeric', month: 'short', year: 'numeric' }).format(new Date(`${date}T12:00:00`))

export function TournamentsPage() {
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const query = useQuery({ queryKey: tournamentKeys.mine(userId), queryFn: api.myTournaments, enabled: userId.length > 0 })
  const [params] = useSearchParams()
  const view = tournamentListView(params)
  const data = query.error || auth.error ? undefined : query.data
  const visible = data?.filter(({ tournament }) => matchesTournamentView(tournament, view))
  return (
    <section className="page tournament-list-page">
      <header className="page-header"><p className="brand">Guttas Golf</p><h1>Dine turneringer</h1></header>
      <nav className="tournament-list-views" aria-label="Turneringsvisning">
        {TOURNAMENT_LIST_VIEWS.map((option) => <Link key={option.id} to={{ search: tournamentViewSearch(params, option.id) }} aria-current={view === option.id ? 'page' : undefined}>
          {option.label}{data && ` (${data.filter(({ tournament }) => matchesTournamentView(tournament, option.id)).length})`}
        </Link>)}
      </nav>
      <button className="tournament-list-refresh" type="button" disabled={query.isFetching || !!auth.error} onClick={() => void query.refetch()}>Oppdater turneringer</button>
      {auth.error && <ErrorState error={auth.error} onRetry={() => void auth.retry()} />}
      {query.isPending && <LoadingState />}
      {!query.isPending && query.isFetching && <p role="status">Oppdaterer turneringer …</p>}
      {query.error && <ErrorState error={query.error} onRetry={() => void query.refetch()} />}
      {data?.length === 0 && <EmptyState>Du er ikke med i noen turneringer ennå.</EmptyState>}
      {data && data.length > 0 && visible?.length === 0 && <EmptyState>{view === 'archived' ? 'Ingen arkiverte turneringer ennå.' : 'Ingen nåværende turneringer. Tidligere turneringer finnes i Arkiv.'}</EmptyState>}
      <div className="item-list">
        {visible?.map(({ tournament, role }) => (
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
