import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Navigate, useSearchParams } from 'react-router-dom'
import { api } from '../api/client'
import { leaderboardKeys } from '../api/leaderboards'
import { tournamentKeys } from '../api/tournaments'
import { LeaderboardControls } from '../features/leaderboards/LeaderboardControls'
import { RoundStandings } from '../features/leaderboards/RoundStandings'
import {
  leaderboardSearch,
  parseMetric,
  parseScope,
  preferredRound,
  type LeaderboardScope,
} from '../features/leaderboards/selection'
import { TournamentStandings } from '../features/leaderboards/TournamentStandings'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { useAuth } from '../features/auth/authContext'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { useVisibilityRefetch } from '../features/visibility/useVisibilityRefetch'
import { loadTournamentLeaderboardAfterRounds } from '../features/leaderboards/tournamentLoader'

export function LeaderboardPage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const auth = useAuth()
  const queryClient = useQueryClient()
  const userId = auth.session?.user_id ?? ''
  const tournamentsQuery = useQuery({ queryKey: tournamentKeys.list(userId), queryFn: api.tournaments })
  const tournaments = tournamentsQuery.data ?? []
  const requestedTournamentId = searchParams.get('tournament')
  const selectedTournament = tournaments.find((item) => item.id === requestedTournamentId)
    ?? tournaments.find((item) => item.status === 'active')
    ?? tournaments[0]
  const tournamentId = selectedTournament?.id ?? ''
  useTournamentLive(tournamentId)
  const scope = parseScope(searchParams.get('scope'))
  const metric = parseMetric(searchParams.get('metric'))

  const roundsQuery = useQuery({
    queryKey: tournamentKeys.rounds(userId, tournamentId),
    queryFn: () => api.rounds(tournamentId),
    enabled: tournamentId.length > 0,
  })
  const roundsQueryKey = tournamentKeys.rounds(userId, tournamentId)
  const rounds = roundsQuery.data ?? []
  const requestedRoundId = searchParams.get('round')
  const selectedRound = rounds.find((round) => round.id === requestedRoundId) ?? preferredRound(rounds)

  const roundLeaderboardQuery = useQuery({
    queryKey: leaderboardKeys.round(userId, selectedRound?.id ?? '', metric),
    queryFn: () => api.roundLeaderboard(selectedRound?.id ?? '', tournamentId, metric),
    enabled: scope === 'round' && selectedRound !== undefined,
  })
  const tournamentLeaderboardQuery = useQuery({
    queryKey: leaderboardKeys.tournament(userId, tournamentId, metric),
    queryFn: () => loadTournamentLeaderboardAfterRounds({
      queryClient,
      roundsQueryKey,
      loadRounds: () => api.rounds(tournamentId),
      loadLeaderboard: () => api.tournamentLeaderboard(tournamentId, metric),
    }),
    enabled: scope === 'tournament'
      && tournamentId.length > 0
      && roundsQuery.data !== undefined
      && !roundsQuery.error,
  })
  useVisibilityRefetch(roundLeaderboardQuery.data?.visibility, roundLeaderboardQuery.refetch)
  useVisibilityRefetch(tournamentLeaderboardQuery.data?.visibility, tournamentLeaderboardQuery.refetch)

  if (tournamentsQuery.isPending) return <section className="page leaderboard-page"><LoadingState /></section>
  if (tournamentsQuery.error && tournaments.length === 0) {
    return (
      <section className="page leaderboard-page">
        <header className="page-header"><p className="brand">Guttas Golf</p><h1>Resultater</h1></header>
        <ErrorState error={tournamentsQuery.error} onRetry={() => void tournamentsQuery.refetch()} />
      </section>
    )
  }
  if (!selectedTournament) {
    return (
      <section className="page leaderboard-page">
        <header className="page-header"><p className="brand">Guttas Golf</p><h1>Resultater</h1></header>
        <EmptyState>Ingen turneringer er opprettet</EmptyState>
      </section>
    )
  }

  if (roundsQuery.data) {
    const canonical = leaderboardSearch(tournamentId, scope, selectedRound?.id, metric)
    if (searchParams.toString() !== canonical.toString()) {
      return <Navigate replace to={`/leaderboard?${canonical.toString()}`} />
    }
  }

  const setSelection = (
    nextTournamentId: string,
    nextScope: LeaderboardScope,
    nextRoundId: string | undefined,
    nextMetric: typeof metric,
  ) => setSearchParams(leaderboardSearch(nextTournamentId, nextScope, nextRoundId, nextMetric))

  const activeQuery = scope === 'round' ? roundLeaderboardQuery : tournamentLeaderboardQuery

  return (
    <section className="page leaderboard-page" key={tournamentId}>
      <header className="page-header leaderboard-header">
        <div><p className="brand">Guttas Golf</p><h1>Resultater</h1></div>
        {activeQuery.isFetching && !activeQuery.isPending && <span role="status">Oppdaterer …</span>}
      </header>

      <LeaderboardControls
        tournaments={tournaments}
        rounds={rounds}
        tournamentId={tournamentId}
        roundId={selectedRound?.id}
        scope={scope}
        metric={metric}
        roundsPending={roundsQuery.isPending}
        onTournamentChange={(id) => setSelection(id, scope, undefined, metric)}
        onRoundChange={(id) => setSelection(tournamentId, scope, id, metric)}
        onScopeChange={(nextScope) => setSelection(tournamentId, nextScope, selectedRound?.id, metric)}
        onMetricChange={(nextMetric) => setSelection(tournamentId, scope, selectedRound?.id, nextMetric)}
      />

      {tournamentsQuery.error && tournaments.length > 0 && (
        <ErrorState error={tournamentsQuery.error} onRetry={() => void tournamentsQuery.refetch()} />
      )}

      {roundsQuery.error && (
        <ErrorState error={roundsQuery.error} onRetry={() => void roundsQuery.refetch()} />
      )}

      {scope === 'round' && !roundsQuery.error && roundsQuery.isPending && <LoadingState />}
      {scope === 'tournament' && roundsQuery.isPending && !activeQuery.isPending && <LoadingState />}
      {scope === 'round' && !roundsQuery.error && !roundsQuery.isPending && rounds.length === 0 && (
        <EmptyState>Turneringen har ingen runder ennå</EmptyState>
      )}

      {activeQuery.isPending
        && !(scope === 'round' && (roundsQuery.isPending || rounds.length === 0))
        && !(scope === 'tournament' && roundsQuery.error)
        && <LoadingState />}
      {activeQuery.error && (
        <ErrorState error={activeQuery.error} onRetry={() => void activeQuery.refetch()} />
      )}

      {scope === 'round' && roundLeaderboardQuery.data?.entries.length === 0 && (
        <EmptyState>
          {roundLeaderboardQuery.data.status === 'draft'
            ? 'Runden er fortsatt en kladd. Resultater vises når runden åpnes.'
            : 'Runden har ingen resultatlinjer ennå.'}
        </EmptyState>
      )}
      {scope === 'round' && roundLeaderboardQuery.data?.visibility.mode === 'front_nine' && (
        <p className="leaderboard-visibility-notice" role="status">Hull 10–18 er skjult til finaleresultatene frigis.</p>
      )}
      {scope === 'round' && roundLeaderboardQuery.data && roundLeaderboardQuery.data.entries.length > 0 && (
        <RoundStandings leaderboard={roundLeaderboardQuery.data} />
      )}

      {scope === 'tournament' && !roundsQuery.isPending && !roundsQuery.error && tournamentLeaderboardQuery.data?.entries.length === 0 && (
        <EmptyState>Ingen spillere er registrert i turneringen</EmptyState>
      )}
      {scope === 'tournament' && tournamentLeaderboardQuery.data?.visibility.mode === 'front_nine' && (
        <p className="leaderboard-visibility-notice" role="status">Finalen viser bare synlige resultater. Hull 10–18 er skjult til frigivelse.</p>
      )}
      {scope === 'tournament' && !roundsQuery.isPending && !roundsQuery.error && tournamentLeaderboardQuery.data && tournamentLeaderboardQuery.data.entries.length > 0 && (
        <TournamentStandings leaderboard={tournamentLeaderboardQuery.data} rounds={rounds} />
      )}
    </section>
  )
}
