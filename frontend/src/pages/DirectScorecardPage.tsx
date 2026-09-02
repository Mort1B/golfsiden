import { useQuery } from '@tanstack/react-query'
import { Navigate, useParams, useSearchParams } from 'react-router-dom'
import { useEffect } from 'react'
import { api } from '../api/client'
import { leaderboardKeys } from '../api/leaderboards'
import { scoringKeys, type ReadScorecard, type ScoreOwner } from '../api/scorecards'
import { tournamentKeys } from '../api/tournaments'
import { useAuth } from '../features/auth/authContext'
import { parseDrilldownMetric, parseOwnerType, scorecardSearch } from '../features/leaderboards/drilldownRoutes'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { DirectScorecardView } from '../features/scoring/DirectScorecardView'
import { matchingRound, projectedOwner } from '../features/scoring/directScorecard'
import { canonicalVisibleHole, parseHoleNumber, type ScoreView } from '../features/scoring/selection'
import { useVisibilityRefetch } from '../features/visibility/useVisibilityRefetch'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'

export function DirectScorecardPage() {
  const { tournamentId = '', roundId = '', ownerType: ownerTypeParam, ownerId = '' } = useParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const userId = useAuth().session?.user_id ?? ''
  const metric = parseDrilldownMetric(searchParams.get('metric'))
  const view: ScoreView = searchParams.get('view') === 'hole' ? 'hole' : 'summary'
  const requestedHole = parseHoleNumber(searchParams.get('hole'))
  const ownerType = parseOwnerType(ownerTypeParam)
  const targetOwner: ScoreOwner | null = ownerType === null || ownerId === '' ? null : { type: ownerType, id: ownerId }
  useTournamentLive(tournamentId)
  useEffect(() => {
    window.scrollTo({ top: 0 })
  }, [ownerId, ownerTypeParam, roundId, tournamentId])

  const roundsQuery = useQuery({
    queryKey: tournamentKeys.rounds(userId, tournamentId),
    queryFn: () => api.rounds(tournamentId),
    enabled: tournamentId !== '' && roundId !== '' && targetOwner !== null,
  })
  const round = matchingRound(roundsQuery.data ?? [], tournamentId, roundId)
  const leaderboardQuery = useQuery({
    queryKey: leaderboardKeys.round(userId, roundId, metric),
    queryFn: () => api.roundLeaderboard(roundId, tournamentId, metric),
    enabled: round !== null && targetOwner !== null,
    retry: false,
  })
  const ownerEntry = targetOwner === null || leaderboardQuery.data === undefined
    ? null
    : projectedOwner(leaderboardQuery.data, tournamentId, roundId, targetOwner)
  const cardQuery = useQuery<ReadScorecard>({
    queryKey: scoringKeys.read(userId, roundId, targetOwner ?? { type: 'player', id: '' }),
    queryFn: () => {
      if (ownerEntry === null) throw new Error('Scorekortmålet er ikke synlig i runden.')
      return api.scorecardRead(roundId, ownerEntry.owner)
    },
    enabled: ownerEntry !== null,
    retry: false,
  })
  useVisibilityRefetch(leaderboardQuery.data?.visibility, leaderboardQuery.refetch)
  useVisibilityRefetch(cardQuery.data?.visibility, cardQuery.refetch)

  const visibleHole = canonicalVisibleHole(cardQuery.data?.holes.map((hole) => hole.hole_number) ?? [], requestedHole)
  const hole = cardQuery.data?.holes.find((candidate) => candidate.hole_number === visibleHole)
  const canonical = scorecardSearch(metric, view, view === 'hole' ? visibleHole : undefined)
  const canCanonicalize = view === 'summary' || visibleHole !== undefined
  if (canCanonicalize && searchParams.toString() !== canonical.toString()) {
    return <Navigate replace to={`/tournaments/${tournamentId}/rounds/${roundId}/scorecards/${ownerTypeParam ?? ''}/${ownerId}?${canonical}`} />
  }

  const retryAll = () => void Promise.all([roundsQuery.refetch(), leaderboardQuery.refetch(), cardQuery.refetch()])
  const hasBackgroundError = Boolean((roundsQuery.error && roundsQuery.data)
    || (leaderboardQuery.error && leaderboardQuery.data) || (cardQuery.error && cardQuery.data))
  return (
    <section className="page score-page" key={`${tournamentId}:${roundId}:${ownerTypeParam ?? ''}:${ownerId}:${metric}`}>
      <header className="page-header leaderboard-header">
        <div><p className="brand">Guttas Golf</p><h1>Scorekort</h1></div>
        {(roundsQuery.isFetching || leaderboardQuery.isFetching || cardQuery.isFetching) && !cardQuery.isPending && <span role="status">Oppdaterer …</span>}
      </header>
      {hasBackgroundError && <div className="background-query-error" role="alert"><p>Noe kunne ikke oppdateres. Viste data beholdes.</p><button type="button" onClick={retryAll}>Prøv oppdatering</button></div>}
      {targetOwner === null && <EmptyState>Scorekortadressen har en ugyldig eiertype.</EmptyState>}
      {targetOwner !== null && roundsQuery.isPending && <LoadingState />}
      {targetOwner !== null && roundsQuery.error && !roundsQuery.data && <ErrorState error={roundsQuery.error} onRetry={() => void roundsQuery.refetch()} />}
      {targetOwner !== null && roundsQuery.data && round === null && <EmptyState>Runden tilhører ikke denne turneringen.</EmptyState>}
      {round !== null && leaderboardQuery.isPending && <LoadingState />}
      {round !== null && leaderboardQuery.error && !leaderboardQuery.data && <ErrorState error={leaderboardQuery.error} onRetry={() => void leaderboardQuery.refetch()} />}
      {leaderboardQuery.data && ownerEntry === null && <EmptyState>Scorekortet finnes ikke blant de synlige resultatene i denne runden.</EmptyState>}
      {ownerEntry !== null && cardQuery.isPending && <LoadingState />}
      {ownerEntry !== null && cardQuery.error && !cardQuery.data && <ErrorState error={cardQuery.error} onRetry={() => void cardQuery.refetch()} />}
      {cardQuery.data && cardQuery.data.holes.length === 0 && <EmptyState>Scorekortet har ingen synlige hull.</EmptyState>}
      {round && ownerEntry && cardQuery.data && hole && (
        <DirectScorecardView round={round} projectedOwner={ownerEntry} card={cardQuery.data} metric={metric} view={view} hole={hole}
          onHole={(nextHole) => setSearchParams(scorecardSearch(metric, 'hole', nextHole))} />
      )}
    </section>
  )
}
