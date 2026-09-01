import { Check, ChevronLeft, ChevronRight, Minus, Plus, RefreshCw, RotateCcw } from 'lucide-react'
import type { ScorecardHole, ScoringScorecard } from '../../api/scorecards'
import type { ScoreSyncSnapshot } from './scoreCoordinator'

interface HoleEntryProps {
  card: ScoringScorecard
  hole: ScorecardHole
  sync: ScoreSyncSnapshot
  canEdit: boolean
  navigationLocked: boolean
  onScore: (value: number) => void
  onRetry: () => void
  onDiscard: () => void
  onPrevious: () => void
  onNext: () => void
}

function syncLabel(sync: ScoreSyncSnapshot): string {
  if (sync.phase === 'saving') return 'Lagrer …'
  if (sync.phase === 'queued') return 'Ny endring venter …'
  if (sync.phase === 'verifying') return 'Kontrollerer lagret score …'
  if (sync.phase === 'synced') return 'Synkronisert'
  if (sync.phase === 'failed') return 'Lagring feilet'
  return sync.serverValue === null ? 'Ikke registrert' : 'Lagret'
}

export function HoleEntry(props: HoleEntryProps) {
  const value = props.sync.desiredValue
  const hasVerifiedNet = (props.sync.phase === 'idle' || props.sync.phase === 'synced')
    && value !== null
    && value === props.sync.serverValue
    && value === props.hole.score?.gross_strokes
  const net = hasVerifiedNet ? props.hole.net_strokes : null
  const controlsDisabled = !props.canEdit || props.sync.phase === 'failed'
  return (
    <section className="hole-entry" aria-labelledby="current-hole-heading">
      <header>
        <div><p>Hull</p><h2 id="current-hole-heading">{props.hole.hole_number}</h2></div>
        <dl><div><dt>Par</dt><dd>{props.hole.par}</dd></div><div><dt>Index</dt><dd>{props.hole.stroke_index}</dd></div></dl>
      </header>

      <div className="score-stepper">
        <button type="button" title="Trekk fra ett slag" aria-label="Trekk fra ett slag" disabled={controlsDisabled || (value !== null && value <= 1)} onClick={() => props.onScore(value === null ? props.hole.par - 1 : value - 1)}><Minus aria-hidden="true" /></button>
        {value === null ? (
          <button type="button" className="record-par" disabled={controlsDisabled} onClick={() => props.onScore(props.hole.par)}>Registrer par <strong>{props.hole.par}</strong></button>
        ) : (
          <output aria-live="polite">
            <strong>{value}</strong>
            <span>{net === null ? 'Netto beregnes' : `Netto ${net}`}</span>
          </output>
        )}
        <button type="button" title="Legg til ett slag" aria-label="Legg til ett slag" disabled={controlsDisabled || (value !== null && value >= 20)} onClick={() => props.onScore(value === null ? props.hole.par + 1 : value + 1)}><Plus aria-hidden="true" /></button>
      </div>

      <div className={`score-sync score-sync-${props.sync.phase}`} role="status">
        {props.sync.phase === 'synced' && <Check aria-hidden="true" />}{syncLabel(props.sync)}
      </div>
      {props.sync.error && (
        <div className="score-save-error" role="alert">
          <p>{props.sync.error.message}</p>
          <div>
            {props.sync.error.retryable && <button type="button" onClick={props.onRetry}><RefreshCw aria-hidden="true" />Prøv igjen</button>}
            <button type="button" onClick={props.onDiscard}><RotateCcw aria-hidden="true" />Forkast</button>
          </div>
        </div>
      )}

      <footer>
        <button type="button" disabled={props.navigationLocked || props.hole.hole_number === 1} onClick={props.onPrevious}><ChevronLeft aria-hidden="true" />Forrige</button>
        <span>{props.card.holes_scored} av {props.card.number_of_holes} registrert</span>
        <button type="button" disabled={props.navigationLocked || props.hole.hole_number === props.card.holes.length} onClick={props.onNext}>Neste<ChevronRight aria-hidden="true" /></button>
      </footer>
    </section>
  )
}
