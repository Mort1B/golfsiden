import { useQuery } from '@tanstack/react-query'
import { ChevronLeft, Flag, MapPin, Users } from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import { api } from '../api/client'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { StatusBadge } from '../ui/StatusBadge'

export function RoundPage() {
  const { roundId = '' } = useParams()
  const round = useQuery({ queryKey: ['round', roundId], queryFn: () => api.round(roundId) })
  const teams = useQuery({ queryKey: ['round', roundId, 'teams'], queryFn: () => api.teams(roundId) })
  if (round.isPending) return <section className="page"><LoadingState /></section>
  if (round.error) return <section className="page"><ErrorState error={round.error} /></section>
  return (
    <section className="page">
      <header className="detail-header">
        <Link to={`/tournaments/${round.data.tournament_id}`} className="back-button" aria-label="Tilbake til turnering"><ChevronLeft /></Link>
        <div><p className="brand">Runde {round.data.round_number}</p><h1>{round.data.name}</h1></div>
        <StatusBadge status={round.data.status} />
      </header>
      <div className="round-meta"><span><MapPin />{round.data.course_name}</span><span><Flag />{round.data.tee_name} · {round.data.number_of_holes} hull</span></div>
      <div className="section-heading"><h2>Lag</h2><span>{teams.data?.length ?? 0}</span></div>
      {teams.isPending && <LoadingState />}
      {teams.error && <ErrorState error={teams.error} />}
      {teams.data?.length === 0 && <EmptyState>Ingen lag er satt opp</EmptyState>}
      <div className="team-grid">
        {teams.data?.map((team) => (
          <article className="team-card" key={team.id}>
            <header><div><p>Start {team.starting_hole ? `hull ${team.starting_hole}` : 'ikke satt'}</p><h2>{team.name}</h2></div><Users aria-hidden="true" /></header>
            <ol>{team.members.map((member) => <li key={member.player_id}><span>{member.display_name}</span></li>)}</ol>
          </article>
        ))}
      </div>
    </section>
  )
}
