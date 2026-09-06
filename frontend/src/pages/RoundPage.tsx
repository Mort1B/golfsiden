import { useQuery } from '@tanstack/react-query'
import { ChevronLeft, Flag, MapPin } from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import { api } from '../api/client'
import { ErrorState, LoadingState } from '../ui/AsyncState'
import { StatusBadge } from '../ui/StatusBadge'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { roundManagementUrl } from '../features/tournaments/lifecycle/lifecycleState'
import { pairingApi, pairingKeys } from '../api/pairings'
import { RoundGroups } from '../features/rounds/RoundGroups'
import { scoringSearch } from '../features/scoring/selection'
import { leaderboardSearch } from '../features/leaderboards/selection'
import { isCanonicalUuid } from '../api/decoder'

export function RoundPage() {
  const { roundId = '' } = useParams()
  const { session } = useAuth()
  if (!isCanonicalUuid(roundId)) return <section className="page"><ErrorState error={new Error('Ugyldig runde.')} /></section>
  if (!session) return <section className="page"><LoadingState /></section>
  return <RoundWorkspace key={`${session.user_id}:${roundId}`} roundId={roundId} />
}

function RoundWorkspace({ roundId }: { roundId: string }) {
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const round = useQuery({ queryKey: tournamentKeys.round(userId, roundId), queryFn: () => api.round(roundId) })
  const memberships = useQuery({ queryKey: tournamentKeys.mine(userId), queryFn: api.myTournaments, enabled: userId.length > 0 })
  const isAdmin = !memberships.error && memberships.data?.some((entry) => entry.tournament.id === round.data?.tournament_id && entry.role === 'admin')
  useTournamentLive(round.data?.tournament_id ?? '')
  const pairings = useQuery({
    queryKey: pairingKeys.detail(userId, roundId),
    queryFn: () => pairingApi.get(roundId, round.data?.tournament_id ?? ''),
    enabled: round.data !== undefined && !round.error,
  })
  const retry = () => { void round.refetch(); void pairings.refetch() }
  if (round.isPending) return <section className="page"><LoadingState /></section>
  if (round.error) return <section className="page"><ErrorState error={round.error} onRetry={() => { void round.refetch() }} /></section>
  if (pairings.error) return <section className="page"><ErrorState error={pairings.error} onRetry={retry} /></section>
  if (pairings.data && (pairings.data.status !== round.data.status || pairings.data.scoring_format !== round.data.scoring_format)) {
    return <section className="page"><ErrorState error={new Error('Runden er endret. Oppdater for å se gjeldende oppsett.')} onRetry={retry} /></section>
  }
  return (
    <section className="page round-details">
      <header className="detail-header">
        <Link to={`/tournaments/${round.data.tournament_id}`} className="back-button" aria-label="Tilbake til turnering"><ChevronLeft /></Link>
        <div><p className="brand">Runde {round.data.round_number}</p><h1>{round.data.name}</h1></div>
        <StatusBadge status={round.data.status} />
      </header>
      {isAdmin && <div className="tournament-admin-actions"><Link to={roundManagementUrl(round.data.tournament_id, roundId)}>Administrer runden</Link></div>}
      <div className="round-meta"><span><MapPin aria-hidden="true" />{round.data.course_id ? round.data.course_name : 'Bane ikke satt opp'}</span><span><Flag aria-hidden="true" />{round.data.tee_id ? round.data.tee_name : 'Utslagssted ikke satt opp'} · {round.data.number_of_holes} hull</span></div>
      <nav className="round-detail-links" aria-label="Rundevalg">
        {round.data.status !== 'draft' && <Link to={`/score?${scoringSearch({ tournamentId: round.data.tournament_id, roundId, view: 'summary' })}`}>Åpne scorekort</Link>}
        <Link to={`/leaderboard?${leaderboardSearch(round.data.tournament_id, 'round', roundId, 'net')}`}>Se rundens resultater</Link>
      </nav>
      {round.data.status === 'draft' && <p>Scorekort blir tilgjengelig når runden åpnes.</p>}
      {pairings.isFetching && <LoadingState />}
      {pairings.data && <RoundGroups pairings={pairings.data} />}
    </section>
  )
}
