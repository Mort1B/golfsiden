import { useQuery } from '@tanstack/react-query'
import { Navigate, useSearchParams } from 'react-router-dom'
import type { ReactNode } from 'react'
import { api } from '../api/client'
import { ownerEquals, scoringKeys } from '../api/scorecards'
import { tournamentKeys } from '../api/tournaments'
import { privateWorkspaceKeys } from '../api/privateWorkspace'
import { ScoringExperience } from '../features/scoring/ScoringExperience'
import {
  parseHoleNumber,
  parseScoreView,
  preferredScoreRound,
  replaceScoreHistory,
  scoreableRounds,
  scoringSearch,
  selectedOwner,
  type ScoreHistoryAction,
  type ScoreSelection,
  type ScoreView,
} from '../features/scoring/selection'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'
import { useAuth } from '../features/auth/authContext'

export function ScorePage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const auth = useAuth()
  const userId = auth.session?.user_id ?? ''
  const tournamentsQuery = useQuery({ queryKey: tournamentKeys.list(userId), queryFn: api.tournaments })
  const tournaments = tournamentsQuery.data ?? []
  const tournament = tournaments.find((item) => item.id === searchParams.get('tournament'))
    ?? tournaments.find((item) => item.status === 'active')
    ?? tournaments[0]
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
  const canWrite = owner !== undefined
    && writableOwners.some((writable) => ownerEquals(writable, owner.owner))
  const cardQuery = useQuery({
    queryKey: scoringKeys.scorecard(round?.id ?? '', owner?.owner ?? { type: 'player', id: '' }),
    queryFn: () => api.scorecard(round?.id ?? '', owner?.owner ?? { type: 'player', id: '' }),
    enabled: round !== undefined && owner !== undefined,
  })
  const view = parseScoreView(searchParams.get('view'))
  const requestedHole = parseHoleNumber(searchParams.get('hole'))
  const hole = cardQuery.data?.holes.find((item) => item.hole_number === requestedHole)
    ?? cardQuery.data?.holes[0]

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
  if (cardQuery.error && !cardQuery.data) {
    return <ScoreState><ErrorState error={cardQuery.error} onRetry={() => void cardQuery.refetch()} /></ScoreState>
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
      <ScoringExperience
        tournaments={tournaments}
        rounds={eligibleRounds}
        round={{ ...round, status: completionQuery.data?.status ?? round.status }}
        owners={progressOwners}
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
        onHole={(number, adjacent) => navigate({ ...base('hole'), holeNumber: number }, adjacent ? (number < hole.hole_number ? 'previous' : 'next') : 'hole')}
        onView={(nextView) => navigate(base(nextView), 'view')}
      />
    </section>
  )
}

function ScoreState({ children }: { children: ReactNode }) {
  return <section className="page score-page"><header className="page-header"><p className="brand">Guttas Golf</p><h1>Score</h1></header>{children}</section>
}
