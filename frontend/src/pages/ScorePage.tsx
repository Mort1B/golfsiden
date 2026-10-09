import { useOnline } from '../features/scoring/prepared/useOnline'
import { PreparedReturnOffer, PrepareScoreVisit } from '../features/scoring/prepared/PreparedReturnOffer'
import { usePreparedScore } from '../features/scoring/prepared/context'
import { isScoreResumeSearch, usePublishTournamentNavigation } from '../routing/tournamentNavigation'
import { useScoreDrafts } from '../features/scoring/recovery/context'
import { ScoreRecovery } from '../features/scoring/recovery/ScoreRecovery'
import { MatchRound } from '../features/matchPlay/MatchRound'
import { StablefordExperience } from '../features/scoring/stableford/StablefordExperience'
import { FourBallExperience } from '../features/scoring/fourBall/FourBallExperience'
import { PendingScores } from '../features/scoring/offline/PendingScores'
import { useScoreWorkspaceData } from '../features/scoring/useScoreWorkspaceData'
import { Navigate, useLocation, useSearchParams } from 'react-router-dom'
import type { ReactNode } from 'react'
import { ScoringExperience } from '../features/scoring/ScoringExperience'
import { ReadScorecardExperience } from '../features/scoring/ReadScorecardExperience'
import {
  quickOwnerSelection,
  replaceScoreHistory,
  scoringSearch,
  type ScoreHistoryAction,
  type ScoreSelection,
  type ScoreView,
} from '../features/scoring/selection'
import { EmptyState, ErrorState, LoadingState } from '../ui/AsyncState'

export function ScorePage() {
  const location = useLocation()
  const resume = isScoreResumeSearch(location.search)
  return <><PendingScores />{resume && <PreparedReturnOffer />}<PreparedWorkspace key={resume ? location.key : 'selected'} resume={resume} /></>
}

function PreparedWorkspace({ resume }: { resume: boolean }) {
  const [params, setParams] = useSearchParams(), { prepared, available } = usePreparedScore()
  const { drafts } = useScoreDrafts()
  const returning = params.get('prepared') === '1', online = useOnline()
  const unavailable = returning && (!prepared || !available() || params.get('tournament') !== prepared.tournamentId
    || params.get('round') !== prepared.roundId || params.get('owner_type') !== prepared.owner.type
    || params.get('owner') !== prepared.owner.id)
  usePublishTournamentNavigation(null, unavailable)
  if (unavailable) return drafts.length ? <ScoreRecovery /> : <ScoreState><EmptyState>Koble til nettet og åpne scorekortet på nytt. Det klargjorte kortet er ikke lenger tilgjengelig.</EmptyState>
    <button className="score-recovery-toggle" type="button" disabled={!online} onClick={() => setParams({ tournament: params.get('tournament') ?? '', round: params.get('round') ?? '', resume: '1' }, { replace: true })}>Hent scorekort på nytt</button></ScoreState>
  return <ScoreWorkspace resume={resume} preparedReturn={returning} />
}

function ScoreWorkspace({ resume, preparedReturn }: { resume: boolean; preparedReturn: boolean }) {
  const { drafts } = useScoreDrafts()
  const [searchParams, setSearchParams] = useSearchParams()
  const { tournamentsQuery, tournaments, tournament, roundsQuery, eligibleRounds, round,
    completionQuery, accessQuery, progressOwners, writableOwners, owner, effectiveRoundStatus,
    canWrite, cardQuery, terminalScoringError, view, hole, prefetchOwner, retainingScorer,
    connectionLost, retryLive, deniedError, verificationPending } = useScoreWorkspaceData(searchParams, resume, preparedReturn)

  usePublishTournamentNavigation(tournament && !tournamentsQuery.error && !roundsQuery.error && !deniedError && !terminalScoringError
    ? { tournamentId: tournament.id, roundId: round?.id ?? null } : null,
    !tournamentsQuery.isFetching && !tournamentsQuery.isPending
      && (!tournament || !roundsQuery.isFetching && !roundsQuery.isPending))

  const loading = connectionLost
    ? <ErrorState error={new Error('Forbindelsen er brutt. Prøver å koble til igjen.')} onRetry={retryLive} />
    : <LoadingState />

  const navigate = (selection: ScoreSelection, action: ScoreHistoryAction) => {
    const search = scoringSearch(selection)
    if (verificationPending) search.set('prepared', '1')
    setSearchParams(search, { replace: replaceScoreHistory(action) })
  }

  if (drafts.length > 0 && (drafts.some(draft => draft.recovery) || effectiveRoundStatus === 'locked'
    || deniedError || terminalScoringError || (!preparedReturn && (tournamentsQuery.error || roundsQuery.error
    || completionQuery.error || accessQuery.error || cardQuery.error)) || !cardQuery.data || !owner || !canWrite
    || drafts.some(draft => draft.target.roundId !== round?.id || draft.target.tournamentId !== tournament?.id
      || (draft.kind === 'four_ball' ? draft.target.sideId : draft.target.owner.id) !== owner.owner.id
      || (draft.kind === 'four_ball' ? 'team' : draft.target.owner.type) !== owner.owner.type || draft.target.holeId !== hole?.hole_id))) return <ScoreRecovery />

  if (round?.scoring_format === 'singles_match_play') return <section className="page match-page"><h1>Score · matchspill</h1><label>Velg turnering<select value={tournament?.id ?? ''} onChange={e => setSearchParams({ tournament: e.target.value })}>{tournaments.map(t => <option key={t.id} value={t.id}>{t.name}</option>)}</select></label><label>Velg runde<select value={round.id} onChange={e => setSearchParams({ tournament: round.tournament_id, round: e.target.value })}>{eligibleRounds.map(r => <option key={r.id} value={r.id}>{r.name}</option>)}</select></label><MatchRound roundId={round.id} /></section>

  if (deniedError) return <ScoreState><ErrorState error={deniedError} onRetry={retryLive} /></ScoreState>

  if (resume) {
    const required = [tournamentsQuery, ...(tournament ? [roundsQuery] : []),
      ...(round ? [completionQuery, accessQuery] : []), ...(owner ? [cardQuery] : [])]
    const failed = required.find((query) => query.isError)
    if (failed) return <ScoreState><ErrorState error={failed.error} onRetry={() => void failed.refetch()} /></ScoreState>
    if (required.some((query) => !query.isFetchedAfterMount || query.isFetching)) {
      return <ScoreState>{loading}</ScoreState>
    }
  }

  if (tournamentsQuery.isPending) return <ScoreState>{loading}</ScoreState>
  if (tournamentsQuery.error && tournaments.length === 0) {
    return <ScoreState><ErrorState error={tournamentsQuery.error} onRetry={() => void tournamentsQuery.refetch()} /></ScoreState>
  }
  if (!tournament) return <ScoreState><EmptyState>Ingen turneringer er opprettet</EmptyState></ScoreState>
  if (roundsQuery.isPending) return <ScoreState>{loading}</ScoreState>
  if (roundsQuery.error && !roundsQuery.data) {
    return <ScoreState><ErrorState error={roundsQuery.error} onRetry={() => void roundsQuery.refetch()} /></ScoreState>
  }
  if (!round) return <ScoreState><EmptyState>Turneringen har ingen åpne, fullførte eller låste runder</EmptyState></ScoreState>
  if ((completionQuery.isPending && !retainingScorer) || accessQuery.isPending) return <ScoreState>{loading}</ScoreState>
  if (completionQuery.error && !completionQuery.data && !retainingScorer) {
    return <ScoreState><ErrorState error={completionQuery.error} onRetry={() => void completionQuery.refetch()} /></ScoreState>
  }
  if (accessQuery.error && !accessQuery.data) {
    return <ScoreState><ErrorState error={accessQuery.error} onRetry={() => void accessQuery.refetch()} /></ScoreState>
  }
  if (!owner) return <ScoreState><EmptyState>Runden har ingen kvalifiserte scorekort</EmptyState></ScoreState>
  if (cardQuery.isPending) return <ScoreState>{loading}</ScoreState>
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
  if (verificationPending) canonical.set('prepared', '1')
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
      <PrepareScoreVisit target={{ tournamentId: tournament.id, roundId: round.id, owner: owner.owner, holeNumber: hole.hole_number }}
        ready={canWrite && cardQuery.data.projection === 'scoring' && (preparedReturn || retainingScorer || [tournamentsQuery, roundsQuery, completionQuery, accessQuery, cardQuery].every(query => query.isSuccess && !query.isFetching))} />
      {connectionLost && <div className="background-query-error" role="status">
        <p>Forbindelsen er brutt. Prøver å koble til igjen.</p>
        <button type="button" onClick={retryLive}>Koble til igjen</button>
      </div>}
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
      {'format' in cardQuery.data && cardQuery.data.format === 'individual_stableford' ? <StablefordExperience tournaments={tournaments} rounds={eligibleRounds}
        round={{ ...round, status: effectiveRoundStatus ?? round.status }} owners={verificationPending ? [] : progressOwners} card={cardQuery.data}
        writableOwners={writableOwners} onQuickOwner={next=>navigate(quickOwnerSelection(base(),next),'quick-owner')} onPrefetchOwner={prefetchOwner}
        holeNumber={hole.hole_number} view={view} canWrite={canWrite} recovering={verificationPending || retainingScorer || connectionLost || accessQuery.error !== null}
        onTournament={id => navigate({ tournamentId: id, view: 'hole' }, 'tournament')}
        onRound={id => navigate({ tournamentId: tournament.id, roundId: id, view: 'hole' }, 'round')}
        onOwner={id => { const next = progressOwners.find(item => item.owner.id === id); if (next) navigate({ ...base('hole'), owner: next.owner, holeNumber: 1 }, 'owner') }}
        onHole={number => navigate({ ...base('hole'), holeNumber: number }, 'hole')}
        onView={nextView => navigate(base(nextView), 'view')} />
        : 'format' in cardQuery.data && cardQuery.data.format === 'four_ball_stroke_play' ? <FourBallExperience tournaments={tournaments} rounds={eligibleRounds}
        round={{ ...round, status: effectiveRoundStatus ?? round.status }} owners={verificationPending ? [] : progressOwners} card={cardQuery.data}
        writableOwners={writableOwners} onQuickOwner={next=>navigate(quickOwnerSelection(base(),next),'quick-owner')} onPrefetchOwner={prefetchOwner}
        holeNumber={hole.hole_number} view={view} canWrite={canWrite} recovering={verificationPending || retainingScorer || connectionLost || accessQuery.error !== null}
        onTournament={id => navigate({ tournamentId: id, view: 'hole' }, 'tournament')}
        onRound={id => navigate({ tournamentId: tournament.id, roundId: id, view: 'hole' }, 'round')}
        onOwner={id => { const next = progressOwners.find(item => item.owner.id === id); if (next) navigate({ ...base('hole'), owner: next.owner, holeNumber: 1 }, 'owner') }}
        onHole={number => navigate({ ...base('hole'), holeNumber: number }, 'hole')}
        onView={nextView => navigate(base(nextView), 'view')} />
        : 'players' in hole || 'gross_points' in hole ? null : cardQuery.data.projection === 'scoring' ? <ScoringExperience
        tournaments={tournaments}
        rounds={eligibleRounds}
        round={{ ...round, status: effectiveRoundStatus ?? round.status }}
        owners={verificationPending ? [] : progressOwners}
        writableOwners={writableOwners}
        selectedOwner={owner}
        card={cardQuery.data}
        hole={hole}
        view={view}
        canWrite={canWrite}
        recovering={verificationPending || retainingScorer || connectionLost || accessQuery.error !== null}
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
        owners={verificationPending ? [] : progressOwners}
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
