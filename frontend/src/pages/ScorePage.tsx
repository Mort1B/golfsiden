import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Navigate, useSearchParams } from 'react-router-dom'
import { useCallback, useEffect, type ReactNode } from 'react'
import { api } from '../api/client'
import { ownerEquals, scoringKeys, type ScorecardSummary } from '../api/scorecards'
import { ApiHttpError } from '../api/http'
import { tournamentKeys } from '../api/tournaments'
import { privateWorkspaceKeys } from '../api/privateWorkspace'
import { ScoringExperience } from '../features/scoring/ScoringExperience'
import { ReadScorecardExperience } from '../features/scoring/ReadScorecardExperience'
import {
  parseHoleNumber,
  parseScoreView,
  preferredScoreRound,
  quickOwnerSelection,
  replaceScoreHistory,
  scoreableRounds,
  scoringSearch,
  selectedOwner,
  adjacentWritableOwners,
  writableOwnerProgress,
  type ScoreHistoryAction,
  type ScoreSelection,
  type ScoreView,
  canonicalVisibleHole,
} from '../features/scoring/selection'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { useAuth } from '../features/auth/authContext'
import { useTournamentLive } from '../features/live/useTournamentLive'
import { useVisibilityRefetch } from '../features/visibility/useVisibilityRefetch'

export function ScorePage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const queryClient = useQueryClient()
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const tournamentsQuery = useQuery({ queryKey: tournamentKeys.list(userId), queryFn: api.tournaments })
  const tournaments = tournamentsQuery.data ?? []
  const tournament = tournaments.find((item) => item.id === searchParams.get('tournament'))
    ?? tournaments.find((item) => item.status === 'active')
    ?? tournaments[0]
  useTournamentLive(tournament?.id ?? '')
  const roundsQuery = useQuery({
    queryKey: tournamentKeys.rounds(userId, tournament?.id ?? ''),
    queryFn: () => api.rounds(tournament?.id ?? ''),
    enabled: tournament !== undefined,
  })
  const eligibleRounds = scoreableRounds(roundsQuery.data ?? [])
  const round = eligibleRounds.find((item) => item.id === searchParams.get('round'))
    ?? preferredScoreRound(eligibleRounds)
  const completionQuery = useQuery({
    queryKey: privateWorkspaceKeys.completion(userId, round?.id ?? ''),
    queryFn: () => api.completionValidation(round?.id ?? '', round?.scoring_format ?? 'individual_stroke_play'),
    enabled: round !== undefined,
  })
  const accessQuery = useQuery({
    queryKey: privateWorkspaceKeys.scoreAccess(userId, round?.id ?? ''),
    queryFn: () => api.scoreAccess(round?.id ?? ''),
    enabled: round !== undefined,
    retry: false,
  })
  const progressOwners = completionQuery.data?.owners ?? []
  const writableOwners = accessQuery.data?.writable_owners ?? []
  const owner = selectedOwner(
    progressOwners,
    searchParams.get('owner_type'),
    searchParams.get('owner'),
    writableOwners,
  )
  const effectiveRoundStatus = completionQuery.data?.status ?? round?.status
  const canWrite = owner !== undefined
    && (effectiveRoundStatus === 'open' || effectiveRoundStatus === 'completed')
    && writableOwners.some((writable) => ownerEquals(writable, owner.owner))
  const queryOwner = owner?.owner ?? { type: 'player' as const, id: '' }
  const cardQuery = useQuery<ScorecardSummary>({
    queryKey: canWrite
      ? scoringKeys.scoring(userId, round?.id ?? '', queryOwner)
      : scoringKeys.read(userId, round?.id ?? '', queryOwner),
    queryFn: () => canWrite
      ? api.scorecardScoring(round?.id ?? '', queryOwner)
      : api.scorecardRead(round?.id ?? '', queryOwner),
    enabled: round !== undefined && owner !== undefined && accessQuery.data !== undefined,
    retry: false,
  })
  const terminalScoringError = canWrite && cardQuery.error instanceof ApiHttpError
    && (cardQuery.error.status === 401 || cardQuery.error.status === 403
      || cardQuery.error.code === 'round_not_editable')
  const refetchAccess = accessQuery.refetch
  const refetchCompletion = completionQuery.refetch
  const refetchRounds = roundsQuery.refetch
  useEffect(() => {
    if (!terminalScoringError || !round || !owner) return
    queryClient.removeQueries({ queryKey: scoringKeys.scoring(userId, round.id, owner.owner), exact: true })
    void Promise.all([refetchCompletion(), refetchAccess(), refetchRounds()])
  }, [owner, queryClient, refetchAccess, refetchCompletion, refetchRounds, round, terminalScoringError, userId])
  const refetchCard = cardQuery.refetch
  const refetchVisibilityProjection = useCallback(async () => {
    await refetchCompletion()
    if (owner !== undefined) await refetchCard()
  }, [owner, refetchCard, refetchCompletion])
  useVisibilityRefetch(completionQuery.data?.visibility, refetchVisibilityProjection)
  const view = parseScoreView(searchParams.get('view'))
  const requestedHole = parseHoleNumber(searchParams.get('hole'))
  const visibleHole = canonicalVisibleHole(cardQuery.data?.holes.map((item) => item.hole_number) ?? [], requestedHole)
  const hole = cardQuery.data?.holes.find((item) => item.hole_number === visibleHole)

  const prefetchOwner = useCallback((nextOwner: NonNullable<typeof owner>['owner']) => {
    if (!round) return
    void queryClient.prefetchQuery({
      queryKey: scoringKeys.scoring(userId, round.id, nextOwner),
      queryFn: () => api.scorecardScoring(round.id, nextOwner),
    })
  }, [queryClient, round, userId])

  useEffect(() => {
    if (!owner) return
    const writableCards = writableOwnerProgress(
      completionQuery.data?.owners ?? [],
      accessQuery.data?.writable_owners ?? [],
    )
    for (const neighbor of adjacentWritableOwners(writableCards, owner.owner)) {
      prefetchOwner(neighbor)
    }
  }, [accessQuery.data?.writable_owners, completionQuery.data?.owners, owner, prefetchOwner])

  const navigate = (selection: ScoreSelection, action: ScoreHistoryAction) => {
    setSearchParams(scoringSearch(selection), { replace: replaceScoreHistory(action) })
  }

  if (tournamentsQuery.isPending) return <ScoreState><LoadingState /></ScoreState>
  if (tournamentsQuery.error && tournaments.length === 0) {
    return <ScoreState><ErrorState error={tournamentsQuery.error} onRetry={() => void tournamentsQuery.refetch()} /></ScoreState>
  }
  if (!tournament) return <ScoreState><EmptyState>Ingen turneringer er opprettet</EmptyState></ScoreState>
  if (roundsQuery.isPending) return <ScoreState><LoadingState /></ScoreState>
  if (roundsQuery.error && !roundsQuery.data) {
    return <ScoreState><ErrorState error={roundsQuery.error} onRetry={() => void roundsQuery.refetch()} /></ScoreState>
  }
  if (!round) return <ScoreState><EmptyState>Turneringen har ingen åpne, fullførte eller låste runder</EmptyState></ScoreState>
  if (completionQuery.isPending || accessQuery.isPending) return <ScoreState><LoadingState /></ScoreState>
  if (completionQuery.error && !completionQuery.data) {
    return <ScoreState><ErrorState error={completionQuery.error} onRetry={() => void completionQuery.refetch()} /></ScoreState>
  }
  if (accessQuery.error && !accessQuery.data) {
    return <ScoreState><ErrorState error={accessQuery.error} onRetry={() => void accessQuery.refetch()} /></ScoreState>
  }
  if (!owner) return <ScoreState><EmptyState>Runden har ingen kvalifiserte scorekort</EmptyState></ScoreState>
  if (cardQuery.isPending) return <ScoreState><LoadingState /></ScoreState>
  if (terminalScoringError || (cardQuery.error && !cardQuery.data)) {
    return <ScoreState><ErrorState error={cardQuery.error ?? new Error('Scorekortet kunne ikke lastes.')} onRetry={() => void cardQuery.refetch()} /></ScoreState>
  }
  if (!cardQuery.data || !hole) return <ScoreState><EmptyState>Scorekortet har ingen hull</EmptyState></ScoreState>

  const canonical = scoringSearch({
    tournamentId: tournament.id,
    roundId: round.id,
    owner: owner.owner,
    holeNumber: hole.hole_number,
    view,
  })
  if (searchParams.toString() !== canonical.toString()) {
    return <Navigate replace to={`/score?${canonical.toString()}`} />
  }

  const base = (nextView: ScoreView = view): ScoreSelection => ({
    tournamentId: tournament.id,
    roundId: round.id,
    owner: owner.owner,
    holeNumber: hole.hole_number,
    view: nextView,
  })

  return (
    <section className="page score-page">
      <header className="page-header"><p className="brand">Guttas Golf</p><h1>Score</h1></header>
      {(roundsQuery.error || completionQuery.error || accessQuery.error || cardQuery.error) && (
        <div className="background-query-error" role="alert">
          <p>Noe kunne ikke oppdateres. Viste data beholdes.</p>
          <button type="button" onClick={() => {
            if (roundsQuery.error) void roundsQuery.refetch()
            if (completionQuery.error) void completionQuery.refetch()
            if (accessQuery.error) void accessQuery.refetch()
            if (cardQuery.error) void cardQuery.refetch()
          }}>Prøv oppdatering</button>
        </div>
      )}
      {cardQuery.data.projection === 'scoring' ? <ScoringExperience
        tournaments={tournaments}
        rounds={eligibleRounds}
        round={{ ...round, status: effectiveRoundStatus ?? round.status }}
        owners={progressOwners}
        writableOwners={writableOwners}
        selectedOwner={owner}
        card={cardQuery.data}
        hole={hole}
        view={view}
        canWrite={canWrite}
        onTournament={(id) => navigate({ tournamentId: id, view: 'hole' }, 'tournament')}
        onRound={(id) => navigate({ tournamentId: tournament.id, roundId: id, view: 'hole' }, 'round')}
        onOwner={(id) => {
          const next = progressOwners.find((item) => item.owner.id === id)
          if (next) navigate({ tournamentId: tournament.id, roundId: round.id, owner: next.owner, holeNumber: 1, view: 'hole' }, 'owner')
        }}
        onQuickOwner={(next) => navigate(quickOwnerSelection(base(), next), 'quick-owner')}
        onPrefetchOwner={prefetchOwner}
        onHole={(number, adjacent) => navigate({ ...base('hole'), holeNumber: number }, adjacent ? (number < hole.hole_number ? 'previous' : 'next') : 'hole')}
        onView={(nextView) => navigate(base(nextView), 'view')}
      /> : <ReadScorecardExperience
        tournaments={tournaments}
        rounds={eligibleRounds}
        round={{ ...round, status: effectiveRoundStatus ?? round.status }}
        owners={progressOwners}
        selectedOwner={owner}
        card={cardQuery.data}
        hole={hole}
        view={view}
        onTournament={(id) => navigate({ tournamentId: id, view: 'hole' }, 'tournament')}
        onRound={(id) => navigate({ tournamentId: tournament.id, roundId: id, view: 'hole' }, 'round')}
        onOwner={(id) => {
          const next = progressOwners.find((item) => item.owner.id === id)
          if (next) navigate({ tournamentId: tournament.id, roundId: round.id, owner: next.owner, holeNumber: 1, view: 'hole' }, 'owner')
        }}
        onHole={(number) => navigate({ ...base('hole'), holeNumber: number }, number < hole.hole_number ? 'previous' : 'next')}
        onView={(nextView) => navigate(base(nextView), 'view')}
      />}
    </section>
  )
}

function ScoreState({ children }: { children: ReactNode }) {
  return <section className="page score-page"><header className="page-header"><p className="brand">Guttas Golf</p><h1>Score</h1></header>{children}</section>
}
