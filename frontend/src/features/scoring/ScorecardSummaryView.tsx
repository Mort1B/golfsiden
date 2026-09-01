import { CheckCircle2, Circle } from 'lucide-react'
import type { ScorecardSummary } from '../../api/scorecards'

interface ScorecardSummaryViewProps {
  card: ScorecardSummary
  disabled: boolean
  readOnly: boolean
  confirming: boolean
  confirmationError: string | null
  confirmationRetryable: boolean
  onHole: (holeNumber: number) => void
  onConfirm: () => void
}

export function ScorecardSummaryView(props: ScorecardSummaryViewProps) {
  const visibleHoleCount = props.card.projection === 'read'
    ? props.card.visible_hole_count
    : props.card.number_of_holes
  return (
    <section className="scorecard-summary" aria-labelledby="summary-heading">
      <header><div><p>Scorekort</p><h2 id="summary-heading">Oppsummering</h2></div><strong>{props.card.holes_scored}/{visibleHoleCount}</strong></header>
      <dl className="score-totals">
        <div><dt>Brutto</dt><dd>{props.card.holes_scored > 0 ? props.card.gross_total : '–'}</dd></div>
        <div><dt>Netto</dt><dd>{props.card.holes_scored > 0 ? props.card.net_total : '–'}</dd></div>
        <div><dt>Spille-HCP</dt><dd>{props.card.playing_handicap}</dd></div>
      </dl>
      <ol className="scorecard-holes">
        {props.card.holes.map((hole) => (
          <li key={hole.hole_id}>
            <button type="button" disabled={props.disabled} onClick={() => props.onHole(hole.hole_number)}>
              {hole.score ? <CheckCircle2 aria-hidden="true" /> : <Circle aria-hidden="true" />}
              <span><strong>Hull {hole.hole_number}</strong><small>Par {hole.par} · Index {hole.stroke_index}</small></span>
              <span><strong>{hole.score?.gross_strokes ?? '–'}</strong><small>Netto {hole.net_strokes ?? '–'}</small></span>
            </button>
          </li>
        ))}
      </ol>
      {props.card.confirmed && <p className="confirmation-state">Scorekortet er bekreftet</p>}
      {!props.card.confirmed && props.card.complete && !props.readOnly && (
        <button type="button" className="confirm-scorecard" disabled={props.disabled || props.confirming} onClick={props.onConfirm}>
          {props.confirming ? 'Bekrefter …' : 'Bekreft fullført scorekort'}
        </button>
      )}
      {props.confirmationError && <div className="confirmation-error" role="alert"><p>{props.confirmationError}</p>{props.confirmationRetryable && props.card.complete && !props.card.confirmed && <button type="button" onClick={props.onConfirm}>Prøv bekreftelse igjen</button>}</div>}
    </section>
  )
}
