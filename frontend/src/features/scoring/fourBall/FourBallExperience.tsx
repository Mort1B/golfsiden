import { useEffect, useState } from 'react'
import { useBlocker } from 'react-router-dom'
import type { FourBallCard, FourBallScoringCard } from '../../../api/fourBall'
import type { OwnerCompletionProgress } from '../../../api/scorecards'
import type { Round, Tournament } from '../../../api/types'
import { useAuth } from '../../auth/authContext'
import { ScoreSelectors } from '../ScoreSelectors'
import { useScoringGuard } from '../scoringGuardContext'
import type { ScoreView } from '../selection'
import { FourBallSummary, FourBallResult, FourBallTotals } from './FourBallSummary'
import { FourBallReadView } from './FourBallReadView'
import { PartnerInput } from './PartnerInput'
import { useFourBallSync } from './useFourBallSync'
import { useFourBallConfirmation } from './useFourBallConfirmation'

interface Props {
  tournaments: Tournament[]; rounds: Round[]; round: Round; owners: OwnerCompletionProgress[]
  card: FourBallCard; holeNumber: number; view: ScoreView; canWrite: boolean; recovering: boolean
  onTournament: (id: string) => void; onRound: (id: string) => void; onOwner: (id: string) => void
  onHole: (number: number) => void; onView: (view: ScoreView) => void
}
export function FourBallExperience(props: Props) {
  return props.card.projection === 'scoring'
    ? <EditableSide key={`${props.card.round_id}:${props.card.owner.id}`} {...props} card={props.card} />
    : <><header className="scorecard-owner"><div><p>Four-ball · lagscore</p><h2>{props.card.owner_name}</h2></div></header>
      <SideSelectors {...props} disabled={false} />
      <p>{props.round.status === 'locked' ? 'Runden er låst. Scorekortet er skrivebeskyttet.' : 'Dette scorekortet er skrivebeskyttet.'}</p>
      <FourBallReadView card={props.card} view={props.view} holeNumber={props.holeNumber} onHole={props.onHole} /></>
}
function SideSelectors(props: Props & { disabled: boolean }) {
  return <ScoreSelectors tournaments={props.tournaments} rounds={props.rounds} owners={props.owners}
    holes={props.card.holes} tournamentId={props.round.tournament_id} roundId={props.round.id} owner={props.card.owner}
    holeNumber={props.holeNumber} view={props.view} disabled={props.disabled} onTournament={props.onTournament}
    onRound={props.onRound} onOwner={props.onOwner} onHole={props.onHole} onView={props.onView} />
}
function EditableSide(props: Props & { card: FourBallScoringCard }) {
  const hole = props.card.holes.find(item => item.hole_number === props.holeNumber)
  return hole ? <SideInput {...props} hole={hole} /> : <p>Ingen synlige hull.</p>
}
function SideInput(props: Props & { card: FourBallScoringCard; hole: FourBallScoringCard['holes'][number] }) {
  const { card, hole } = props
  const first = useFourBallSync(card, hole, card.partners[0].player_id, props.round.tournament_id)
  const second = useFourBallSync(card, hole, card.partners[1].player_id, props.round.tournament_id)
  const [correction, setCorrection] = useState(false)
  const [acknowledged, setAcknowledged] = useState(false)
  const [navigationWarning, setNavigationWarning] = useState(false)
  const auth = useAuth()
  const confirmation = useFourBallConfirmation({ round: props.round, tournamentId: props.round.tournament_id,
    owner: card.owner, card, acknowledgeBlanks: acknowledged, csrfToken: auth.session?.csrf_token ?? null,
    onConfirmed: () => { setCorrection(false); setAcknowledged(false) }, onTerminal: () => setCorrection(false) })
  const locked = first.navigationLocked || second.navigationLocked || confirmation.confirming
  const blocker = useBlocker(locked)
  const { setBlocked } = useScoringGuard()
  useEffect(() => { setBlocked(locked); return () => setBlocked(false) }, [locked, setBlocked])
  useEffect(() => {
    if (blocker.state === 'blocked') { setNavigationWarning(true); blocker.reset() }
  }, [blocker])
  useEffect(() => { if (!locked) setNavigationWarning(false) }, [locked])
  useEffect(() => {
    if (!locked) return
    const prevent = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = '' }
    window.addEventListener('beforeunload', prevent)
    return () => window.removeEventListener('beforeunload', prevent)
  }, [locked])
  const editable = props.canWrite && Boolean(auth.session) && (props.round.status === 'open' || props.round.status === 'completed')
    && (!card.confirmed || correction) && !confirmation.confirming
  const blanks = card.holes.some(item => item.players.some(player => player.score === null))
  const syncs = [first, second]
  return <>
    <header className="scorecard-owner"><div><p>Four-ball · lagscore</p><h2>{card.owner_name}</h2></div></header>
    <SideSelectors {...props} disabled={locked} />
    {props.recovering && <p role="status">Oppdaterer tilgang og rundestatus. Endringer lagres først på enheten. Bekreftelse venter.</p>}
    {navigationWarning && <p role="alert">Fullfør eller forkast den ulagrede endringen før du går videre.</p>}
    {props.round.status === 'completed' && <p>Runden er fullført. Score kan korrigeres frem til låsing.</p>}
    {props.round.status === 'completed' && !card.complete && <p role="status">Laget mangler tellende score på minst ett hull. Rundens resultat teller ikke sammenlagt før alle 18 hull har en numerisk lagscore igjen.</p>}
    {card.confirmed && !correction && <div className="correction-gate"><p>Lagets scorekort er bekreftet.</p>
      <button type="button" disabled={locked || !props.canWrite} onClick={() => setCorrection(true)}>Korriger score</button></div>}
    {correction && card.confirmed && <p>Enhver endring fra en partner fjerner lagets bekreftelse.</p>}
    {props.view === 'hole' ? <>
      <h3>Hull {hole.hole_number} · par {hole.par} · indeks {hole.stroke_index}</h3>
      <div className="four-ball-partners">{card.partners.map((partner, index) => {
        const sync = syncs[index]
        const player = hole.players.find(item => item.player_id === partner.player_id)
        return sync && player ? <PartnerInput key={`${hole.hole_id}:${partner.player_id}`} partner={partner} sync={sync} par={hole.par}
          hole={hole.hole_number} strokes={player.handicap_strokes} disabled={!editable || !sync.storageReady} /> : null
      })}</div>
      <FourBallResult card={card} hole={hole} />
      <div className="four-ball-actions"><button type="button" disabled={locked || hole.hole_number <= 1} onClick={() => props.onHole(hole.hole_number - 1)}>Forrige hull</button>
        <button type="button" disabled={locked || hole.hole_number >= card.visible_hole_count} onClick={() => props.onHole(hole.hole_number + 1)}>Neste hull</button></div>
      <FourBallTotals card={card} />
    </> : <>
      <FourBallSummary card={card} disabled={locked} onHole={props.onHole} />
      <section className="four-ball-confirm" aria-label="Bekreft lagets scorekort">
        {!card.complete && <p>Minst én partner må ha en numerisk score på hvert av de 18 hullene før laget kan bekrefte.</p>}
        {card.complete && !card.confirmed && <>
          {blanks && <label><input type="checkbox" checked={acknowledged} onChange={event => setAcknowledged(event.target.checked)} />Jeg bekrefter at tomme partnerfelt ikke bidrar med score.</label>}
          <button type="button" disabled={locked || !props.canWrite || confirmation.blocked || props.recovering || (blanks && !acknowledged)} onClick={confirmation.confirm}>
            {confirmation.confirming ? 'Bekrefter …' : 'Bekreft lagets scorekort'}</button>
        </>}
        {confirmation.blocked && <p>Alle endringer fra begge partnere må være levert og kontrollert før bekreftelse.</p>}
        {confirmation.errorMessage && <p role="alert">{confirmation.errorMessage}</p>}
      </section>
    </>}
  </>
}
