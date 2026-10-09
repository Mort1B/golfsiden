import { TournamentScoreAction } from '../features/tournaments/TournamentScoreAction'
import { ApiHttpError } from '../api/http'
import '../features/tournaments/tournament-overview.css'
import { usePublishTournamentNavigation } from '../routing/tournamentNavigation'
import { useQuery } from '@tanstack/react-query'
import { ChevronLeft, ChevronRight, Flag, Settings, MapPin, Users } from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import { api } from '../api/client'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { StatusBadge } from '../ui/StatusBadge'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { TournamentPlayerSection } from '../features/tournaments/TournamentPlayerSection'
import { useTournamentLive } from '../features/live/useTournamentLive'

export function TournamentPage() {
  const { tournamentId = '' } = useParams()
  return <TournamentWorkspace key={tournamentId} tournamentId={tournamentId} />
}

function TournamentWorkspace({ tournamentId }: { tournamentId: string }) {
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const disconnected=useTournamentLive(tournamentId)
  const tournament = useQuery({ queryKey: tournamentKeys.detail(userId, tournamentId), queryFn: () => api.tournament(tournamentId),retry:false })
  const players = useQuery({ queryKey: tournamentKeys.players(userId, tournamentId), queryFn: () => api.tournamentPlayers(tournamentId) })
  const rounds = useQuery({ queryKey: tournamentKeys.rounds(userId, tournamentId), queryFn: () => api.rounds(tournamentId),retry:false })
  const memberships = useQuery({ queryKey: tournamentKeys.mine(userId), queryFn: api.myTournaments, enabled: userId.length > 0 })
  const isTournamentAdmin = memberships.data?.some((entry) => entry.tournament.id === tournamentId && entry.role === 'admin') ?? false

  usePublishTournamentNavigation(tournament.data && !tournament.error && !rounds.error ? { tournamentId: tournament.data.id } : null,
    !tournament.isPending && !tournament.isFetching && !rounds.isPending && !rounds.isFetching, rounds.data)

  const message=(error:Error,subject:string)=>new Error(error instanceof ApiHttpError&&[401,403,404].includes(error.status)?'Turneringen er ikke tilgjengelig for denne innloggingen. Kontroller kontoen eller kontakt arrangøren.':`Kunne ikke hente ${subject}. Kontroller forbindelsen og prøv igjen.`)
  if (tournament.isPending) return <section className="page"><LoadingState /></section>
  if (tournament.error) return <section className="page"><ErrorState error={message(tournament.error,'turneringen')} onRetry={()=>void tournament.refetch()}/></section>
  return (
    <section className="page tournament-overview">
      <header className="detail-header">
        <Link to="/tournaments" className="back-button" aria-label="Tilbake til turneringer"><ChevronLeft /></Link>
        <div><p className="brand">Turnering</p><h1>{tournament.data.name}</h1></div>
        <StatusBadge status={tournament.data.status} />
      </header>
      <div className="tournament-score-action"><TournamentScoreAction key={`${userId}:${auth.session?.csrf_token}`} tournament={tournamentId} rounds={rounds.data??[]} ready={!disconnected&&tournament.isSuccess&&!tournament.isFetching&&rounds.isSuccess&&!rounds.isFetching}/></div>
      {tournament.data.description && <p className="description">{tournament.data.description}</p>}
      {isTournamentAdmin && <div className="tournament-admin-actions"><Link to={`/manage/tournaments/${tournamentId}`}><Settings aria-hidden="true" />Åpne administrasjon</Link><Link to={`/manage/tournaments/${tournamentId}#entrants`}><Users aria-hidden="true" />Administrer spillere</Link></div>}
      <div className="tournament-admin-actions"><Link to={`/tournaments/${tournamentId}/fantasy`}>Fantasy · min firer og poengtavler</Link></div>
      <div className="summary-strip">
        <div><Flag /><strong>{tournament.data.number_of_rounds}</strong><span>Runder</span></div>
        <div><Users /><strong>{players.data?.players.length ?? '–'}</strong><span>Spillere</span></div>
      </div>
      <div className="section-heading"><h2>Runder</h2><span>{rounds.data?.length ?? '–'} av {tournament.data.number_of_rounds}</span></div>
      {rounds.error?<ErrorState error={message(rounds.error,'rundene')} onRetry={()=>void rounds.refetch()}/>:rounds.isPending?<LoadingState/>:rounds.data.length===0?<EmptyState>Ingen runder er opprettet ennå. Arrangøren kan opprette runder i administrasjonen.</EmptyState>:(
<div className="round-list">
        {rounds.data?.map((round) => (
          <Link to={`/rounds/${round.id}`} className="round-row" key={round.id}>
            <span className="round-number">{round.round_number}</span>
            <div><h3>{round.name}</h3><p><MapPin size={14} /> {round.course_id && round.tee_id ? `${round.course_name} · ${round.tee_name}` : 'Bane ikke konfigurert'}</p></div>
            <div className="round-end"><StatusBadge status={round.status} /><ChevronRight size={18} /></div>
          </Link>
        ))}
      </div>
      )}
      <TournamentPlayerSection
        tournamentId={tournamentId}
        isAdmin={isTournamentAdmin}
        roster={players.data}
        pending={players.isPending}
        error={players.error?message(players.error,'spillerne'):null}
        onRetry={() => void players.refetch()}
        recoveryAccessPending={memberships.isFetching || memberships.fetchStatus === 'paused' || players.isFetching || players.fetchStatus === 'paused'}
        adminAccessPending={userId.length > 0 && memberships.isPending}
        adminAccessError={userId.length > 0 ? memberships.error : null}
      />
    </section>
  )
}
