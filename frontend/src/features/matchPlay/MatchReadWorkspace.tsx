import { usePublishTournamentNavigation } from '../../routing/tournamentNavigation'
import { Link } from 'react-router-dom'
import { api } from '../../api/client'
import { matchApi, matchKeys } from '../../api/matchPlay'
import { tournamentKeys } from '../../api/tournaments'
import { useAuth } from '../auth/authContext'
import { usePrivateResultQuery } from '../leaderboards/usePrivateResultQuery'
import { useTournamentLive } from '../live/useTournamentLive'
import { ErrorState, LoadingState } from '../../ui/AsyncState'
import { MatchReadView, MatchRound } from './MatchRound'
import { matchUrl, tableUrl } from './format'

export function MatchReadWorkspace({ roundId, matchId }: { roundId: string; matchId?: string }) {
  const userId = useAuth().session?.user_id ?? ''
  const round = usePrivateResultQuery({ userId, roundId }, { queryKey: tournamentKeys.round(userId, roundId), queryFn: () => api.round(roundId) })
  const tournamentId = round.data?.tournament_id
  useTournamentLive(tournamentId ?? '')
  const card = usePrivateResultQuery({ userId, roundId, tournamentId }, {
    queryKey: matchKeys.read(userId, roundId, matchId ?? ''),
    queryFn: () => matchApi.read(roundId, matchId ?? ''), enabled: !!matchId && !!round.data, retry: false,
  })
  usePublishTournamentNavigation(round.data && !round.error ? { tournamentId: round.data.tournament_id, roundId: round.data.id } : null,
    !round.isPending && !round.isFetching)

  if (round.error && !round.data) return <section className="page"><ErrorState error={round.error} onRetry={() => void round.refetch()} /></section>
  if (!round.data) return <section className="page"><LoadingState /></section>
  return <section className="page match-page">
    <header className="page-header"><p className="brand">Matchspill · singel</p><h1>{round.data.name}</h1></header>
    {round.error && <ErrorState error={round.error} onRetry={() => void round.refetch()} />}
    <nav className="match-actions" aria-label="Matchvalg"><Link to={`/rounds/${roundId}`}>Runden</Link><Link to={matchUrl(roundId)}>Alle matcher</Link><Link to={tableUrl(round.data.tournament_id)}>Matchpoeng og historikk</Link></nav>
    {!matchId ? <MatchRound roundId={roundId} /> : <>
      {card.error && <ErrorState error={card.error} onRetry={() => void card.refetch()} />}
      {!card.data && !card.error && <LoadingState />}
      {card.data && <MatchReadView card={card.data} />}
    </>}
  </section>
}
