import type { OwnerCompletionProgress, ScoreOwner, ScorecardHole, ScoringScorecard } from '../../api/scorecards'
import type { Round, Tournament } from '../../api/types'
import { useAuth } from '../auth/authContext'
import { useScoringGuard } from './scoringGuardContext'
import { HoleEntry } from './HoleEntry'
import { ScorecardSummaryView } from './ScorecardSummaryView'
import { ScoreSelectors } from './ScoreSelectors'
import { writableOwnerProgress, type ScoreView } from './selection'
import { useHoleScoreSync } from './useHoleScoreSync'
import { useScorecardConfirmation } from './useScorecardConfirmation'
import { useEffect, useState } from 'react'
import { useBlocker } from 'react-router-dom'
import { WritableCardSwitcher } from './WritableCardSwitcher'

interface ScoringExperienceProps {
  tournaments: Tournament[]
  rounds: Round[]
  round: Round
  owners: OwnerCompletionProgress[]
  writableOwners: ScoreOwner[]
  selectedOwner: OwnerCompletionProgress
  card: ScoringScorecard
  hole: ScorecardHole
  view: ScoreView
  onTournament: (id: string) => void
  onRound: (id: string) => void
  onOwner: (id: string) => void
  onQuickOwner: (owner: ScoreOwner) => void
  onPrefetchOwner: (owner: ScoreOwner) => void
  onHole: (number: number, adjacent?: boolean) => void
  onView: (view: ScoreView) => void
  canWrite: boolean
}

export function ScoringExperience(props: ScoringExperienceProps) {
  const ownerKey = `${props.round.id}:${props.selectedOwner.owner.type}:${props.selectedOwner.owner.id}`
  const [correctionKey, setCorrectionKey] = useState<string | null>(null)
  const correctionMode = correctionKey === ownerKey
  const editableRound = props.round.status === 'open' || props.round.status === 'completed'
  const auth = useAuth()
  const { setBlocked } = useScoringGuard()
  const csrfToken = auth.session?.csrf_token ?? null

  const sync = useHoleScoreSync({
    round: props.round,
    tournamentId: props.round.tournament_id,
    owner: props.selectedOwner.owner,
    holeId: props.hole.hole_id,
    serverValue: props.hole.score?.gross_strokes ?? null,
    csrfToken,
    onVerified: (card) => {
      if (card.confirmed) setCorrectionKey(null)
    },
    onTerminal: () => setCorrectionKey(null),
  })
  const confirmation = useScorecardConfirmation({
    round: props.round,
    tournamentId: props.round.tournament_id,
    owner: props.selectedOwner.owner,
    card: props.card,
    csrfToken,
    onConfirmed: () => setCorrectionKey(null),
    onTerminal: () => setCorrectionKey(null),
  })
  const navigationLocked = sync.navigationLocked || confirmation.confirming
  const blocker = useBlocker(navigationLocked)
  const [navigationWarning, setNavigationWarning] = useState(false)
  const canEdit = editableRound
    && csrfToken !== null
    && props.canWrite
    && (!props.card.confirmed || correctionMode)
  const writableCards = writableOwnerProgress(props.owners, props.writableOwners)

  useEffect(() => {
    if (blocker.state !== 'blocked') return
    setNavigationWarning(true)
    blocker.reset()
  }, [blocker])
  useEffect(() => {
    if (!navigationLocked) setNavigationWarning(false)
  }, [navigationLocked])
  useEffect(() => {
    setBlocked(navigationLocked)
    return () => setBlocked(false)
  }, [navigationLocked, setBlocked])
  useEffect(() => {
    if (!navigationLocked) return
    const preventUnload = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', preventUnload)
    return () => window.removeEventListener('beforeunload', preventUnload)
  }, [navigationLocked])

  return (
    <>
      <ScoreSelectors
        tournaments={props.tournaments}
        rounds={props.rounds}
        owners={props.owners}
        holes={props.card.holes}
        tournamentId={props.round.tournament_id}
        roundId={props.round.id}
        owner={props.selectedOwner.owner}
        holeNumber={props.hole.hole_number}
        view={props.view}
        disabled={navigationLocked}
        onTournament={props.onTournament}
        onRound={props.onRound}
        onOwner={props.onOwner}
        onHole={(number) => props.onHole(number)}
        onView={props.onView}
      />

      <WritableCardSwitcher
        owners={writableCards}
        selectedOwner={props.selectedOwner.owner}
        disabled={navigationLocked}
        onSelect={props.onQuickOwner}
        onPrefetch={props.onPrefetchOwner}
      />

      <header className="scorecard-owner">
        <div><p>{props.selectedOwner.owner.type === 'team' ? 'Lagscore' : 'Individuell score'}</p><h2>{props.selectedOwner.owner_name}</h2></div>
        <span>{props.round.name}</span>
      </header>

      {!csrfToken && <div className="scoring-notice error" role="alert">Økten er utløpt. Logg inn på nytt for å lagre.</div>}
      {!props.canWrite && <div className="scoring-notice">Du kan se dette scorekortet, men ikke føre score for det.</div>}
      {navigationWarning && <div className="scoring-notice warning" role="alert">Fullfør eller forkast den pågående scoreendringen før du går videre.</div>}
      {props.round.status === 'locked' && <div className="scoring-notice">Runden er låst. Scorekortet er skrivebeskyttet.</div>}
      {props.round.status === 'completed' && <div className="scoring-notice">Runden er fullført. Korrigering er mulig frem til låsing.</div>}
      {props.card.confirmed && editableRound && !correctionMode && (
        <div className="correction-gate">
          <p>Scorekortet er bekreftet og skrivebeskyttet.</p>
          <button type="button" disabled={navigationLocked || !csrfToken || !props.canWrite} onClick={() => setCorrectionKey(ownerKey)}>Korriger score</button>
        </div>
      )}
      {correctionMode && props.card.confirmed && <div className="scoring-notice warning">Korrigeringsmodus er aktiv. Første endring fjerner bekreftelsen.</div>}

      <dl className="scorecard-strip">
        <div><dt>Brutto</dt><dd>{props.card.holes_scored > 0 ? props.card.gross_total : '–'}</dd></div>
        <div><dt>Netto</dt><dd>{props.card.holes_scored > 0 ? props.card.net_total : '–'}</dd></div>
        <div><dt>Hull</dt><dd>{props.card.holes_scored}/{props.card.number_of_holes}</dd></div>
        <div><dt>Spille-HCP</dt><dd>{props.card.playing_handicap}</dd></div>
      </dl>

      {props.view === 'hole' ? (
        <HoleEntry
          card={props.card}
          hole={props.hole}
          sync={sync.snapshot}
          canEdit={canEdit}
          navigationLocked={navigationLocked}
          onScore={sync.setScore}
          onRetry={sync.retry}
          onDiscard={sync.discard}
          onPrevious={() => props.onHole(props.hole.hole_number - 1, true)}
          onNext={() => props.onHole(props.hole.hole_number + 1, true)}
        />
      ) : (
        <ScorecardSummaryView
          card={props.card}
          disabled={navigationLocked}
          readOnly={!editableRound || !csrfToken || !props.canWrite}
          confirming={confirmation.confirming}
          confirmationError={confirmation.errorMessage}
          confirmationRetryable={confirmation.retryable}
          onHole={(number) => props.onHole(number)}
          onConfirm={confirmation.confirm}
        />
      )}
    </>
  )
}
