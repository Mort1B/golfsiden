import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router-dom'
import { matchApi, matchKeys, type MatchCard, type MatchScoringCard } from '../api/matchPlay'
import { api } from '../api/client'
import { tournamentKeys } from '../api/tournaments'
import { isCanonicalUuid } from '../api/decoder'
import { useAuth } from '../features/auth/authContext'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { MatchRound, MatchReadView } from '../features/matchPlay/MatchRound'
import { MatchScoring } from '../features/matchPlay/MatchScoring'
import { ErrorState, LoadingState } from '../ui/AsyncState'
import { matchUrl, tableUrl } from '../features/matchPlay/format'
import { MatchPending } from '../features/matchPlay/MatchPending'
import { useMatchQueue } from '../features/matchPlay/offline/context'
export function MatchPage({ scoring = false }: { scoring?: boolean }) {
  const { roundId = '', matchId } = useParams(), { session } = useAuth()
  if (!isCanonicalUuid(roundId) || matchId && !isCanonicalUuid(matchId)) return <section className="page"><ErrorState error={new Error('Ugyldig matchadresse.')} /></section>
  return <MatchWorkspace key={`${session?.user_id}:${session?.csrf_token}:${roundId}:${matchId}:${scoring}`} roundId={roundId} matchId={matchId} scoring={scoring} />
}
function MatchWorkspace({ roundId, matchId, scoring }: { roundId: string; matchId?: string; scoring: boolean }) {
  const { session } = useAuth(), user = session?.user_id ?? '', queue = useMatchQueue()
  const round = useQuery({ queryKey: tournamentKeys.round(user, roundId), queryFn: () => api.round(roundId) })
  const memberships = useQuery({ queryKey: tournamentKeys.mine(user), queryFn: api.myTournaments })
  const disconnected = useTournamentLive(round.data?.tournament_id ?? '')
  const card = useQuery<MatchCard | MatchScoringCard>({ queryKey: scoring ? matchKeys.scoring(user, roundId, matchId ?? '') : matchKeys.read(user, roundId, matchId ?? ''),
    queryFn: () => scoring ? matchApi.scoring(roundId, matchId ?? '') : matchApi.read(roundId, matchId ?? ''), enabled: !!matchId && !!round.data, retry: false, networkMode: 'always' })
  const admin = !memberships.error && memberships.data?.some(m => m.tournament.id === round.data?.tournament_id && m.role === 'admin') === true
  const pending = queue.items.find(i => i.matchId === matchId && (i.action || i.notes.length))
  if (round.error && !round.data) return <section className="page"><ErrorState error={round.error} onRetry={() => void round.refetch()} /></section>
  if (!round.data) return <section className="page"><LoadingState /></section>
  return <section className="page match-page"><header className="page-header"><p className="brand">Matchspill · singel</p><h1>{round.data.name}</h1></header>
    {round.error && <ErrorState error={round.error} onRetry={() => void round.refetch()} />}
    <nav className="match-actions" aria-label="Matchvalg"><Link to={`/rounds/${roundId}`}>Runden</Link><Link to={matchUrl(roundId)}>Alle matcher</Link><Link to={tableUrl(round.data.tournament_id)}>Matchpoeng og historikk</Link></nav>
    {!matchId ? <MatchRound roundId={roundId} /> : <>
      {card.error && <ErrorState error={card.error} onRetry={() => void card.refetch()} />}
      {card.error && pending && <MatchPending item={pending} />}
      {!card.data && !card.error && <LoadingState />}
      {card.data && (scoring && 'revision' in card.data ? <MatchScoring card={card.data} admin={admin} recoveryOnly={[card.error, round.error].some(e => e && 'status' in e && [401,403,404].includes(Number(e.status)))} recovering={!!round.error || disconnected || card.isFetching || !!card.error || memberships.isFetching || !!memberships.error} /> : !card.error && <MatchReadView card={card.data} />)}
      {scoring && <Link to={matchUrl(roundId, matchId)}>Åpne tillatt, skrivebeskyttet matchvisning</Link>}
    </>}
  </section>
}
