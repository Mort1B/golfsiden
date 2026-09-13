import { useRef, useState } from 'react'
import { inputLabel, type FourBallPartner } from '../../../api/fourBall'
import type { FourBallSync } from './useFourBallSync'
import { StrokeBadge } from './FourBallSummary'
export function PartnerInput({ partner, sync, par, hole, strokes, disabled, stableford = false }: {
  partner: FourBallPartner; sync: FourBallSync; par: number; hole: number; strokes: number; disabled: boolean; stableford?: boolean
}) {
  const [pickup, setPickup] = useState(false)
  const pickupButton = useRef<HTMLButtonElement>(null)
  const confirmPickup = useRef<HTMLButtonElement>(null)
  const openPickup = () => { setPickup(true); requestAnimationFrame(() => confirmPickup.current?.focus()) }
  const closePickup = () => { setPickup(false); requestAnimationFrame(() => pickupButton.current?.focus()) }
  const value = sync.desired?.type === 'numeric' ? sync.desired.gross_strokes : par
  const state = sync.phase === 'queued' ? 'Lagret på denne enheten. Venter på levering.' : sync.phase === 'saving' ? 'Lagrer på enheten …'
    : sync.phase === 'refreshing' || sync.phase === 'verifying' ? 'Kontrollerer serverscore …' : sync.phase === 'conflict' ? 'Velg mellom lokal score og serverscore under Lokale scoreendringer.'
      : sync.phase === 'blocked' ? 'Levering er blokkert. Endringen beholdes under Lokale scoreendringer.' : sync.phase === 'idle' ? 'Synkronisert' : ''
  return <fieldset className="four-ball-partner">
    <legend>{partner.display_name}</legend>
    <p>Spille-HCP {partner.playing_handicap} <StrokeBadge strokes={strokes} hole={hole} /></p>
    <p>Server: {inputLabel(sync.server)}</p>
    <div className="four-ball-stepper">
      <button type="button" aria-label={`Ett slag mindre for ${partner.display_name}`} disabled={disabled || value <= 1} onClick={() => sync.setInput({ type: 'numeric', gross_strokes: value - 1 })}>−</button>
      <output aria-label={`Score for ${partner.display_name}`}>{sync.desired?.type === 'numeric' ? sync.desired.gross_strokes : sync.desired?.type === 'no_score' ? 'Plukket opp' : '–'}</output>
      <button type="button" aria-label={`Ett slag mer for ${partner.display_name}`} disabled={disabled || value >= 20} onClick={() => sync.setInput({ type: 'numeric', gross_strokes: value + 1 })}>+</button>
    </div>
    <div className="four-ball-actions">
      <button type="button" className="four-ball-record-par" disabled={disabled} onClick={() => sync.setInput({ type: 'numeric', gross_strokes: par })}>Registrer par ({par})</button>
      <button type="button" ref={pickupButton} disabled={disabled} onClick={openPickup}>Plukket opp</button>
    </div>
    {pickup && <div className="scoring-notice warning" role="group" aria-label={`Bekreft pickup for ${partner.display_name}`} onKeyDown={event => { if (event.key === 'Escape') { event.preventDefault(); closePickup() } }}>
      <p>Registrere at {partner.display_name} plukket opp? {stableford ? 'Dette gir 0 brutto- og nettopoeng og løser hullet.' : 'Dette gir ingen tellende score fra spilleren på hullet.'}</p>
      <button type="button" ref={confirmPickup} disabled={disabled} onClick={() => { sync.setInput({ type: 'no_score' }); closePickup() }}>Ja, plukket opp</button>
      <button type="button" onClick={closePickup}>Avbryt</button>
    </div>}
    {state && <p role="status">{sync.phase !== 'idle' ? `Lokalt: ${inputLabel(sync.desired)}. ` : ''}{state}</p>}
    {sync.error && <div role="alert"><p>{sync.error}</p><button type="button" onClick={sync.retry}>Prøv lagring igjen</button><button type="button" onClick={sync.discard}>Forkast ulagret endring</button></div>}
  </fieldset>
}
