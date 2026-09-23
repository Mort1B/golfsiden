import { usePublishTournamentNavigation } from '../routing/tournamentNavigation'
import { usePrivateResultQuery } from '../features/leaderboards/usePrivateResultQuery'
import { Navigate, useParams, useSearchParams } from 'react-router-dom'
import { useEffect } from 'react'
import { stablefordApi, type StablefordReadCard } from '../api/stableford'
import { StablefordReadView } from '../features/scoring/stableford/StablefordReadView'
import { fourBallApi, type FourBallReadCard } from '../api/fourBall'
import { FourBallReadView } from '../features/scoring/fourBall/FourBallReadView'
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

  const roundsQuery = usePrivateResultQuery({ userId, tournamentId }, {
    queryKey: tournamentKeys.rounds(userId, tournamentId),
    queryFn: () => api.rounds(tournamentId),
    enabled: tournamentId !== '' && roundId !== '' && targetOwner !== null,
  })
  const round = matchingRound(roundsQuery.data ?? [], tournamentId, roundId)
  const leaderboardQuery = usePrivateResultQuery({ userId, tournamentId }, {
    queryKey: leaderboardKeys.round(userId, roundId, metric),
    queryFn: () => api.roundLeaderboard(roundId, tournamentId, metric),
    enabled: round !== null && targetOwner !== null,
    retry: false,
  })
  const ownerEntry = targetOwner === null || leaderboardQuery.data === undefined
    ? null
    : projectedOwner(leaderboardQuery.data, tournamentId, roundId, targetOwner)
  const cardQuery = usePrivateResultQuery<ReadScorecard | FourBallReadCard | StablefordReadCard>({ userId, tournamentId, roundId }, {
    queryKey: scoringKeys.read(userId, roundId, targetOwner ?? { type: 'player', id: '' }),
    queryFn: () => {
      if (ownerEntry === null) throw new Error('Scorekortmålet er ikke synlig i runden.')
      return round?.scoring_format === 'individual_stableford' ? stablefordApi.read(roundId, ownerEntry.owner.id) : round?.scoring_format === 'four_ball_stroke_play' ? fourBallApi.read(roundId, ownerEntry.owner.id) : api.scorecardRead(roundId, ownerEntry.owner)
    },
    enabled: ownerEntry !== null,
    retry: false,
  })

  usePublishTournamentNavigation(round && !roundsQuery.error ? { tournamentId, roundId: round.id, metric } : null,
    targetOwner === null || !roundsQuery.isPending && !roundsQuery.isFetching)

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
        'format' in cardQuery.data ? <><h2>{cardQuery.data.owner_name}</h2><div className="four-ball-actions"><button type="button" aria-pressed={view === 'summary'} onClick={() => setSearchParams(scorecardSearch(metric, 'summary'))}>Oppsummering</button><button type="button" aria-pressed={view === 'hole'} onClick={() => setSearchParams(scorecardSearch(metric, 'hole', hole.hole_number))}>Ett hull</button></div>{cardQuery.data.format === 'individual_stableford' ? <StablefordReadView card={cardQuery.data} view={view} holeNumber={hole.hole_number} onHole={number => setSearchParams(scorecardSearch(metric, 'hole', number))} /> : <FourBallReadView card={cardQuery.data} view={view} holeNumber={hole.hole_number} onHole={number => setSearchParams(scorecardSearch(metric, 'hole', number))} />}</> : 'players' in hole || 'gross_points' in hole ? null : <DirectScorecardView round={round} projectedOwner={ownerEntry} card={cardQuery.data} metric={metric} view={view} hole={hole}
          onHole={(nextHole) => setSearchParams(scorecardSearch(metric, 'hole', nextHole))} />
      )}
    </section>
  )
}
