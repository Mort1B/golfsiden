import { useQuery } from '@tanstack/react-query'
import { ChevronLeft, ChevronRight, Flag, MapPin, Users } from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import { api } from '../api/client'
import { ErrorState, LoadingState } from '../ui/AsyncState'
import { StatusBadge } from '../ui/StatusBadge'
import { tournamentKeys } from '../api/tournaments'

export function TournamentPage() {
  const { tournamentId = '' } = useParams()
  const tournament = useQuery({ queryKey: tournamentKeys.detail(tournamentId), queryFn: () => api.tournament(tournamentId) })
  const players = useQuery({ queryKey: tournamentKeys.players(tournamentId), queryFn: () => api.tournamentPlayers(tournamentId) })
  const rounds = useQuery({ queryKey: tournamentKeys.rounds(tournamentId), queryFn: () => api.rounds(tournamentId) })

  if (tournament.isPending) return <section className="page"><LoadingState /></section>
  if (tournament.error) return <section className="page"><ErrorState error={tournament.error} /></section>
  return (
    <section className="page">
      <header className="detail-header">
        <Link to="/tournaments" className="back-button" aria-label="Tilbake til turneringer"><ChevronLeft /></Link>
        <div><p className="brand">Turnering</p><h1>{tournament.data.name}</h1></div>
        <StatusBadge status={tournament.data.status} />
      </header>
      {tournament.data.description && <p className="description">{tournament.data.description}</p>}
      <div className="summary-strip">
        <div><Flag /><strong>{tournament.data.number_of_rounds}</strong><span>Runder</span></div>
        <div><Users /><strong>{players.data?.length ?? '–'}</strong><span>Spillere</span></div>
      </div>
      <div className="section-heading"><h2>Runder</h2><span>{rounds.data?.length ?? 0} av {tournament.data.number_of_rounds}</span></div>
      {rounds.error && <ErrorState error={rounds.error} />}
      <div className="round-list">
        {rounds.data?.map((round) => (
          <Link to={`/rounds/${round.id}`} className="round-row" key={round.id}>
            <span className="round-number">{round.round_number}</span>
            <div><h3>{round.name}</h3><p><MapPin size={14} /> {round.course_id && round.tee_id ? `${round.course_name} · ${round.tee_name}` : 'Bane ikke konfigurert'}</p></div>
            <div className="round-end"><StatusBadge status={round.status} /><ChevronRight size={18} /></div>
          </Link>
        ))}
      </div>
      <div className="section-heading"><h2>Spillere</h2><span>{players.data?.length ?? 0}</span></div>
      <div className="compact-player-grid">
        {players.data?.map((player) => <div key={player.player_id}><span>{player.display_name}</span><strong>{player.tournament_handicap.toFixed(1)}</strong></div>)}
      </div>
    </section>
  )
}
