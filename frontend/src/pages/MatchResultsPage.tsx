import type { ReactNode } from 'react'
import { usePrivateResultQuery } from '../features/leaderboards/usePrivateResultQuery'
import { Link, useParams, useSearchParams } from 'react-router-dom'
import { api } from '../api/client'
import { matchApi, matchKeys } from '../api/matchPlay'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { MatchRound } from '../features/matchPlay/MatchRound'
import { pointsLabel, tableUrl } from '../features/matchPlay/format'
export function MatchResultsPage() {
  const { tournamentId = '' } = useParams()
  return <MatchResults tournamentId={tournamentId} />
}
export function MatchResults({ tournamentId, selectedPlayerId, tournamentSelector }: { tournamentId: string; selectedPlayerId?: string; tournamentSelector?: ReactNode }) {
  const user = useAuth().session?.user_id ?? '', [search] = useSearchParams(), playerId = selectedPlayerId ?? search.get('player') ?? undefined
  useTournamentLive(tournamentId)
  const rounds = usePrivateResultQuery({ userId: user, tournamentId }, { queryKey: tournamentKeys.rounds(user, tournamentId), queryFn: () => api.rounds(tournamentId) })
  const table = usePrivateResultQuery({ userId: user, tournamentId }, { queryKey: matchKeys.table(user, tournamentId), queryFn: ({ signal }) => matchApi.table(tournamentId, signal), retry: false })
  const error = rounds.error ?? table.error
  if (error) return <section className="page match-page">{tournamentSelector}<ErrorState error={error} onRetry={() => { void rounds.refetch(); void table.refetch() }} /></section>
  if (!rounds.data || !table.data) return <section className="page match-page">{tournamentSelector}<LoadingState /></section>
  const matches = rounds.data.filter(r => r.scoring_format === 'singles_match_play')
  return <section className="page match-page">{tournamentSelector}<header className="page-header"><h1>Matchpoeng</h1></header>
    <nav className="match-actions" aria-label="Resultatvalg"><Link to={`/tournaments/${tournamentId}`}>Turneringen</Link><Link to={tableUrl(tournamentId)}>Alle spillere</Link>{rounds.data.some(r => r.scoring_format !== 'singles_match_play') && <Link to={`/leaderboard?tournament=${tournamentId}&scope=tournament&metric=net`}>Sammenlagt brutto/netto</Link>}</nav>
    <p>Seier gir 1 poeng, delt match ½ og tap 0. Bare bekreftede matcher teller. Skjulte finalematcher inngår først når finalen er frigitt, unntatt i administratorens visning. Matchpoeng inngår ikke i sammenlagt brutto/netto.</p>
    {table.data.entries.length === 0 && <EmptyState>Ingen spillere er registrert.</EmptyState>}
    <ol className="match-table">{table.data.entries.filter(e => !playerId || e.player_id === playerId).map(e => <li key={e.player_id}><div><strong>{e.position ?? '–'} · {e.display_name}</strong><span>{pointsLabel(e.half_points)} poeng · {e.played} spilt · {e.wins} vunnet / {e.draws} delt / {e.losses} tapt</span>{e.position === null && <span>Ingen bekreftede matcher</span>}</div><Link to={tableUrl(tournamentId, e.player_id)}>Matchhistorikk</Link></li>)}</ol>
    {playerId && !table.data.entries.some(e => e.player_id === playerId) && <EmptyState>Spilleren finnes ikke i turneringen.</EmptyState>}
    {matches.map(r => <section key={r.id}><h2>{r.name}</h2><MatchRound roundId={r.id} playerId={playerId} /></section>)}
  </section>
}
