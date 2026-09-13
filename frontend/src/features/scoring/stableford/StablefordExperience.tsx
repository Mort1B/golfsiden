import { useEffect, useState } from 'react'
import { useBlocker } from 'react-router-dom'
import type { StablefordCard, StablefordScoringCard } from '../../../api/stableford'
import type { Tournament, Round } from '../../../api/types'
import type { OwnerCompletionProgress } from '../../../api/scorecards'
import type { ScoreView } from '../selection'
import { ScoreSelectors } from '../ScoreSelectors'
import { useAuth } from '../../auth/authContext'
import { useScoringGuard } from '../scoringGuardContext'
import { PartnerInput } from '../fourBall/PartnerInput'
import { StablefordReadView, StablefordResult, StablefordTotals } from './StablefordReadView'
import { useStablefordSync } from './useStablefordSync'
import { useStablefordConfirmation } from './useStablefordConfirmation'
interface Props {
  tournaments: Tournament[]; rounds: Round[]; round: Round; owners: OwnerCompletionProgress[]
  card: StablefordCard; holeNumber: number; view: ScoreView; canWrite: boolean; recovering: boolean
  onTournament: (id: string) => void; onRound: (id: string) => void; onOwner: (id: string) => void
  onHole: (number: number) => void; onView: (view: ScoreView) => void
}
function Selectors(props: Props & { disabled: boolean }) {
  return <ScoreSelectors tournaments={props.tournaments} rounds={props.rounds} owners={props.owners} holes={props.card.holes}
    tournamentId={props.round.tournament_id} roundId={props.round.id} owner={props.card.owner} holeNumber={props.holeNumber}
    view={props.view} disabled={props.disabled} onTournament={props.onTournament} onRound={props.onRound}
    onOwner={props.onOwner} onHole={props.onHole} onView={props.onView} />
}
export function StablefordExperience(props: Props) {
  return props.card.projection === 'scoring' ? <EditableCard key={`${props.card.round_id}:${props.card.owner.id}`} {...props} card={props.card} />
    : <><h2>{props.card.owner_name}</h2><p>Stableford · Spille-HCP {props.card.playing_handicap}</p>
      <Selectors {...props} disabled={false} /><p>{props.round.status === 'locked' ? 'Runden er låst. Scorekortet er skrivebeskyttet.' : 'Dette scorekortet er skrivebeskyttet.'}</p>
      <StablefordReadView card={props.card} view={props.view} holeNumber={props.holeNumber} onHole={props.onHole} /></>
}
function EditableCard(props: Props & { card: StablefordScoringCard }) {
  const hole = props.card.holes.find(h => h.hole_number === props.holeNumber)
  return hole ? <CardInput {...props} hole={hole} /> : <p>Ingen synlige hull.</p>
}
function CardInput(props: Props & { card: StablefordScoringCard; hole: StablefordScoringCard['holes'][number] }) {
  const { card, hole } = props
  const sync = useStablefordSync(card, hole, props.round.tournament_id)
  const [correction, setCorrection] = useState(false)
  const [navigationWarning, setNavigationWarning] = useState(false)
  const auth = useAuth()
  const confirmation = useStablefordConfirmation({ round: props.round, tournamentId: props.round.tournament_id, owner: card.owner, card,
    csrfToken: auth.session?.csrf_token ?? null, onConfirmed: () => setCorrection(false), onTerminal: () => setCorrection(false) })
  const locked = sync.navigationLocked || confirmation.confirming
  const blocker = useBlocker(locked)
  const { setBlocked } = useScoringGuard()
  useEffect(() => { setBlocked(locked); return () => setBlocked(false) }, [locked, setBlocked])
  useEffect(() => { if (blocker.state === 'blocked') { setNavigationWarning(true); blocker.reset() } }, [blocker])
  useEffect(() => { if (!locked) setNavigationWarning(false) }, [locked])
  useEffect(() => {
    if (!locked) return
    const prevent = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = '' }
    window.addEventListener('beforeunload', prevent)
    return () => window.removeEventListener('beforeunload', prevent)
  }, [locked])
  const editable = props.canWrite && Boolean(auth.session) && (props.round.status === 'open' || props.round.status === 'completed')
    && (!card.confirmed || correction) && !confirmation.confirming
  return <>
    <header className="scorecard-owner"><div><p>Stableford · individuelt</p><h2>{card.owner_name}</h2></div></header>
    <Selectors {...props} disabled={locked} />
    {props.recovering && <p role="status">Oppdaterer tilgang og rundestatus. Endringer lagres først på enheten. Bekreftelse venter.</p>}
    {navigationWarning && <p role="alert">Fullfør eller forkast den ulagrede endringen før du går videre.</p>}
    {props.round.status === 'completed' && <p>Runden er fullført. Score kan korrigeres frem til låsing.</p>}
    {card.confirmed && !correction && <div className="correction-gate"><p>Scorekortet er bekreftet.</p>
      <button type="button" disabled={locked || !props.canWrite} onClick={() => setCorrection(true)}>Korriger score</button></div>}
    {correction && card.confirmed && <p>En endring fjerner bekreftelsen, også når poengsummen er lik.</p>}
    {props.view === 'hole' ? <>
      <h3>Hull {hole.hole_number} · par {hole.par} · indeks {hole.stroke_index}</h3>
      <PartnerInput key={hole.hole_id} stableford partner={{ player_id: card.owner.id, display_name: card.owner_name, playing_handicap: card.playing_handicap }}
        sync={sync} par={hole.par} hole={hole.hole_number} strokes={hole.handicap_strokes} disabled={!editable || !sync.storageReady} />
      <StablefordResult hole={hole} />
      <div className="four-ball-actions"><button type="button" disabled={locked || hole.hole_number <= 1} onClick={() => props.onHole(hole.hole_number - 1)}>Forrige hull</button>
        <button type="button" disabled={locked || hole.hole_number >= 18} onClick={() => props.onHole(hole.hole_number + 1)}>Neste hull</button></div>
      <StablefordTotals card={card} />
    </> : <>
      <StablefordReadView card={card} view="summary" holeNumber={props.holeNumber} onHole={props.onHole} disabled={locked} />
      <section className="four-ball-confirm" aria-label="Bekreft scorekort">
        {!card.complete && <p>Alle 18 hull må ha numerisk score eller eksplisitt «Plukket opp» før bekreftelse. Tomme hull fylles ikke automatisk.</p>}
        {card.complete && !card.confirmed && <button type="button" disabled={locked || !props.canWrite || confirmation.blocked || props.recovering} onClick={confirmation.confirm}>
          {confirmation.confirming ? 'Bekrefter …' : 'Bekreft scorekort'}</button>}
        {confirmation.blocked && <p>Alle endringer må være levert og kontrollert før bekreftelse.</p>}
        {confirmation.errorMessage && <p role="alert">{confirmation.errorMessage}</p>}
      </section>
    </>}
  </>
}
