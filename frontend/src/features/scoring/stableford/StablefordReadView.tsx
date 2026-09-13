import { inputLabel } from '../../../api/fourBall'
import type { StablefordCard, StablefordHole } from '../../../api/stableford'
import type { ScoreView } from '../selection'
import { StrokeBadge } from '../fourBall/FourBallSummary'
import { scoreToParLabel } from '../../leaderboards/format'
import '../fourBall/fourBall.css'
import './stableford.css'
export function StablefordTotals({ card }: { card: StablefordCard }) {
  return <><dl className="scorecard-strip" aria-label="Summer fra serveren">
    <div><dt>Bruttopoeng</dt><dd>{card.values?.gross_points ?? '–'}</dd></div>
    <div><dt>Nettopoeng</dt><dd>{card.values?.net_points ?? '–'}</dd></div>
    <div><dt>Hull løst</dt><dd>{card.holes_scored}/{card.visible_hole_count}</dd></div>
  </dl>{card.values && <p>Sammenlagtekvivalent: brutto {scoreToParLabel(card.values.gross_equivalent)} · netto {scoreToParLabel(card.values.net_equivalent)}.</p>}
    {card.values?.actual_gross_total !== null && card.values?.actual_gross_total !== undefined
      ? <p>Faktiske slag: brutto {card.values.actual_gross_total} · netto {card.values.actual_net_total}.</p>
      : <p>Faktiske totalslag vises bare når alle 18 hull har numerisk score.</p>}</>
}
export function StablefordResult({ hole }: { hole: StablefordHole }) {
  return <dl className="four-ball-result stableford-result" aria-label={`Serverpoeng på hull ${hole.hole_number}`}>
    <div><dt>Bruttopoeng</dt><dd>{hole.gross_points ?? '–'}</dd></div>
    <div><dt>Nettopoeng</dt><dd>{hole.net_points ?? '–'}</dd></div>
    <div><dt>Nettoslag</dt><dd>{hole.net_strokes ?? '–'}</dd></div>
  </dl>
}
export function StablefordReadView({ card, view, holeNumber, onHole, disabled = false }: {
  card: StablefordCard; view: ScoreView; holeNumber: number; onHole?: (hole: number) => void; disabled?: boolean
}) {
  const holes = view === 'summary' ? card.holes : card.holes.filter(h => h.hole_number === holeNumber)
  return <section className="four-ball-summary" aria-label="Stableford-scorekort">
    <StablefordTotals card={card} />
    <p>Poeng og ekvivalenter er beregnet av serveren. Lokale endringer venter på levering. Høyest poeng vinner runden; lavest ekvivalent teller sammenlagt. Fullt kort: 36 minus poeng. Delvis kort: 2 per løste hull minus poeng. Pickup gir 0 poeng og +2 ekvivalent; tomme hull bidrar ikke.</p>
    {card.visibility.mode === 'front_nine' && <p>Kun de første ni hullene vises. Bekreftelse og fullføring er skjult.</p>}
    <ol>{holes.map(hole => <li key={hole.hole_id}>
      {onHole && view === 'summary' ? <button type="button" disabled={disabled} onClick={() => onHole(hole.hole_number)}>Hull {hole.hole_number} · par {hole.par}</button> : <h3>Hull {hole.hole_number} · par {hole.par} · indeks {hole.stroke_index}</h3>}
      <p>{inputLabel(hole.score?.input ?? null)} <StrokeBadge strokes={hole.handicap_strokes} hole={hole.hole_number} /></p>
      <StablefordResult hole={hole} />
    </li>)}</ol>
    {onHole && view === 'hole' && <div className="four-ball-actions">
      <button type="button" disabled={disabled || holeNumber <= 1} onClick={() => onHole(holeNumber - 1)}>Forrige hull</button>
      <button type="button" disabled={disabled || holeNumber >= card.visible_hole_count} onClick={() => onHole(holeNumber + 1)}>Neste hull</button>
    </div>}
  </section>
}
