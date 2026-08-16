import { useQuery } from '@tanstack/react-query'
import { api } from '../api/client'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { formatHandicap } from '../features/handicap/format'

export function PlayersPage() {
  const query = useQuery({ queryKey: ['players'], queryFn: api.players })
  return (
    <section className="page">
      <header className="page-header"><p className="brand">Guttas Golf</p><h1>Spillere</h1></header>
      {query.isPending && <LoadingState />}
      {query.error && <ErrorState error={query.error} />}
      {query.data?.length === 0 && <EmptyState>Ingen spillere</EmptyState>}
      <div className="player-list">
        {query.data?.map((player) => (
          <div className="player-row" key={player.id}>
            <span className="avatar" aria-hidden="true">{player.display_name.slice(0, 1).toUpperCase()}</span>
            <div><h2>{player.display_name}</h2><p>{player.email ?? 'Ingen e-post'}</p></div>
            <strong className="handicap">HCP {formatHandicap(player.current_handicap_index)}</strong>
          </div>
        ))}
      </div>
    </section>
  )
}
