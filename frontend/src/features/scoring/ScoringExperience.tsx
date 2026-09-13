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
import { useScoreQueue } from './offline/context'
import { expectedScore } from '../../api/scorecards/conditional'
import { useEffect, useState } from 'react'
import { useBlocker } from 'react-router-dom'
import { WritableCardSwitcher } from './WritableCardSwitcher'

interface ScoringExperienceProps {
  tournaments: Tournament[]
  rounds: Round[]
  round: Round
  owners: OwnerCompletionProgress[]
  writableOwners: ScoreOwner[]
  selectedOwner: Pick<OwnerCompletionProgress, 'owner' | 'owner_name'>
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
  recovering?: boolean
}

export function ScoringExperience(props: ScoringExperienceProps) {
  const queue = useScoreQueue()
  const localScores = new Map([...queue.refreshing.map(value => value.item), ...queue.items]
    .filter(item => item.protocol !== 'four_ball_v1')
    .filter(item => item.roundId === props.round.id && item.owner.type === props.selectedOwner.owner.type && item.owner.id === props.selectedOwner.owner.id)
    .map(item => [item.holeId, item.desired]))
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
    holeNumber: props.hole.hole_number,
    expected: expectedScore(props.card.holes.find(hole => hole.hole_id === props.hole.hole_id)?.score ?? null),
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
    && sync.storageReady
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
      <header className="scorecard-owner">
        <div><p>{props.selectedOwner.owner.type === 'team' ? 'Lagscore' : 'Individuell score'}</p><h2>{props.selectedOwner.owner_name}</h2></div>
        <span>{props.tournaments.find((item) => item.id === props.round.tournament_id)?.name}<br />Runde {props.round.round_number}: {props.round.name}</span>
      </header>

      {!csrfToken && <div className="scoring-notice error" role="alert">Økten er utløpt. Logg inn på nytt for å lagre.</div>}
      {!props.canWrite && <div className="scoring-notice">Du kan se dette scorekortet, men ikke føre score for det.</div>}
      {props.recovering && <div className="scoring-notice" role="status">Oppdaterer scoretilgang og rundestatus. Du kan føre hull på dette kortet; endringer lagres først på enheten. Bekreftelse venter til forbindelsen er tilbake.</div>}
      {navigationWarning && <div className="scoring-notice warning" role="alert">Fullfør eller forkast den pågående scoreendringen før du går videre.</div>}
      {props.round.status === 'locked' && <div className="scoring-notice">Runden er låst. Scorekortet er skrivebeskyttet.</div>}
      {props.round.status === 'completed' && <div className="scoring-notice">Runden er fullført. Korrigering er mulig frem til låsing.</div>}
      {props.card.confirmed && editableRound && !correctionMode && (
        <div className="correction-gate">
          <p>Scorekortet er bekreftet og skrivebeskyttet.</p>
          <button type="button" disabled={navigationLocked || !csrfToken || !props.canWrite || !sync.storageReady} onClick={() => setCorrectionKey(ownerKey)}>Korriger score</button>
        </div>
      )}
      {correctionMode && props.card.confirmed && <div className="scoring-notice warning">Korrigeringsmodus er aktiv. Første endring fjerner bekreftelsen.</div>}

      {props.view === 'hole' ? (
        <HoleEntry
          card={props.card}
          hole={props.hole}
          sync={sync.snapshot}
          canEdit={canEdit}
          navigationLocked={navigationLocked}
          retryDisabled={false}
          onScore={sync.setScore}
          onRetry={sync.retry}
          onDiscard={sync.discard}
          onPrevious={() => props.onHole(props.hole.hole_number - 1, true)}
          onNext={() => props.onHole(props.hole.hole_number + 1, true)}
        />
      ) : (
        <ScorecardSummaryView
          card={props.card}
          localScores={localScores}
          disabled={navigationLocked}
          readOnly={!editableRound || !csrfToken || !props.canWrite}
          confirmationDisabled={confirmation.blocked || props.recovering === true}
          confirming={confirmation.confirming}
          confirmationError={confirmation.errorMessage}
          confirmationRetryable={confirmation.retryable}
          onHole={(number) => props.onHole(number)}
          onConfirm={confirmation.confirm}
        />
      )}

      {props.recovering && <button className="score-recovery-toggle" type="button" disabled={navigationLocked} onClick={() => props.onView(props.view === 'hole' ? 'summary' : 'hole')}>{props.view === 'hole' ? 'Oppsummering' : 'Ett hull'}</button>}
      {!props.recovering && <ScoreSelectors
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
      />}

      {!props.recovering && <WritableCardSwitcher
        owners={writableCards}
        selectedOwner={props.selectedOwner.owner}
        disabled={navigationLocked}
        onSelect={props.onQuickOwner}
        onPrefetch={props.onPrefetchOwner}
      />}

      <dl className="scorecard-strip" aria-label="Summer fra serveren">
        <div><dt>Brutto</dt><dd>{props.card.holes_scored > 0 ? props.card.gross_total : '–'}</dd></div>
        <div><dt>Netto</dt><dd>{props.card.holes_scored > 0 ? props.card.net_total : '–'}</dd></div>
        <div><dt>Hull</dt><dd>{props.card.holes_scored}/{props.card.number_of_holes}</dd></div>
        <div><dt>Spille-HCP</dt><dd>{props.card.playing_handicap}</dd></div>
      </dl>

    </>
  )
}
