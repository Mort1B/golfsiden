import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Navigate, useParams, useSearchParams } from 'react-router-dom'
import { useEffect } from 'react'
import { api } from '../api/client'
import { leaderboardKeys } from '../api/leaderboards'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { parseDrilldownMetric } from '../features/leaderboards/drilldownRoutes'
import { PlayerHistory } from '../features/leaderboards/PlayerHistory'
import { hasPlayerHistoryBackgroundError } from '../features/leaderboards/playerHistory'
import { loadTournamentLeaderboardAfterRounds } from '../features/leaderboards/tournamentLoader'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { useVisibilityRefetch } from '../features/visibility/useVisibilityRefetch'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'

export function PlayerHistoryPage() {
  const { tournamentId = '', playerId = '' } = useParams()
  const [searchParams] = useSearchParams()
  const metric = parseDrilldownMetric(searchParams.get('metric'))
  const userId = useAuth().session?.user_id ?? ''
  const queryClient = useQueryClient()
  useTournamentLive(tournamentId)
  useEffect(() => {
    window.scrollTo({ top: 0 })
  }, [playerId, tournamentId])
  const roundsKey = tournamentKeys.rounds(userId, tournamentId)
  const roundsQuery = useQuery({ queryKey: roundsKey, queryFn: () => api.rounds(tournamentId), enabled: tournamentId !== '' })
  const leaderboardQuery = useQuery({
    queryKey: leaderboardKeys.tournament(userId, tournamentId, metric),
    queryFn: () => loadTournamentLeaderboardAfterRounds({
      queryClient,
      roundsQueryKey: roundsKey,
      loadRounds: () => api.rounds(tournamentId),
      loadLeaderboard: () => api.tournamentLeaderboard(tournamentId, metric),
    }),
    enabled: tournamentId !== '' && playerId !== '' && roundsQuery.data !== undefined && !roundsQuery.error,
  })
  useVisibilityRefetch(leaderboardQuery.data?.visibility, leaderboardQuery.refetch)
  const player = leaderboardQuery.data?.entries.find((entry) => entry.player_id === playerId)
  const canonical = new URLSearchParams({ metric })

  if (searchParams.toString() !== canonical.toString()) {
    return <Navigate replace to={`/tournaments/${tournamentId}/results/players/${playerId}?${canonical}`} />
  }
  return (
    <section className="page leaderboard-page" key={`${tournamentId}:${playerId}:${metric}`}>
      <header className="page-header leaderboard-header">
        <div><p className="brand">Guttas Golf</p><h1>Spillerhistorikk</h1></div>
        {(roundsQuery.isFetching || leaderboardQuery.isFetching) && !leaderboardQuery.isPending && <span role="status">Oppdaterer …</span>}
      </header>
      {hasPlayerHistoryBackgroundError(
        roundsQuery.error, roundsQuery.data, leaderboardQuery.error, leaderboardQuery.data,
      ) && (
        <div className="background-query-error" role="alert"><p>Noe kunne ikke oppdateres. Viste data beholdes.</p><button type="button" onClick={() => void Promise.all([roundsQuery.refetch(), leaderboardQuery.refetch()])}>Prøv oppdatering</button></div>
      )}
      {(roundsQuery.isPending || (roundsQuery.data !== undefined && leaderboardQuery.isPending)) && <LoadingState />}
      {roundsQuery.error && !roundsQuery.data && <ErrorState error={roundsQuery.error} onRetry={() => void roundsQuery.refetch()} />}
      {leaderboardQuery.error && !leaderboardQuery.data && <ErrorState error={leaderboardQuery.error} onRetry={() => void leaderboardQuery.refetch()} />}
      {leaderboardQuery.data && !player && <EmptyState>Spilleren finnes ikke blant de synlige resultatene i denne turneringen.</EmptyState>}
      {leaderboardQuery.data && player && roundsQuery.data && <PlayerHistory leaderboard={leaderboardQuery.data} player={player} rounds={roundsQuery.data} />}
    </section>
  )
}
