import { useRef, useState } from 'react'
import type { MatchCommand, MatchEvent, MatchScoringCard } from '../../api/matchPlay'
import { MatchEventForm } from './MatchEventForm'
import { eventLabel } from './format'
export function MatchCorrection({ card, disabled, submit }: { card: MatchScoringCard; disabled: boolean; submit: (command: MatchCommand) => void }) {
  const [editing, setEditing] = useState(false), [keep, setKeep] = useState(card.events.length), [kind, setKind] = useState<'recording_error' | 'organizer_ruling'>('recording_error'), [reason, setReason] = useState(''), [error, setError] = useState<string | null>(null)
  const opener = useRef<HTMLButtonElement>(null)
  const cancel = () => { setEditing(false); opener.current?.focus() }
  const prefix = card.events.slice(0, keep)
  const holes = prefix.filter(e => e.type === 'hole'), lead = holes.reduce((sum, e) => sum + (e.outcome === 'first' ? 1 : e.outcome === 'second' ? -1 : 0), 0)
  const terminal = prefix.some(e => e.type !== 'hole') || Math.abs(lead) > 18 - holes.length || holes.length === 18
  const proposed = { ...card, events: prefix, resolved_holes: holes.length, lead, finish: null, confirmed: false }
  const correct = (event?: MatchEvent) => {
    if (!reason.trim()) { setError('Forklar hva som var feil registrert eller hvilken avgjørelse som er tatt.'); return }
    submit({ type: 'correct', kind, reason: reason.trim(), superseded_event_ids: card.accepted_events.map(e => e.id), replacement: [...prefix, ...(event ? [event] : [])] })
  }
  return <section className="match-panel"><h3>Administratorkorrigering</h3><p>Rett en registreringsfeil eller registrer en tillatt arrangøravgjørelse. En reell gitt match, hull eller slag kan ikke trekkes tilbake her. Korrigeringen fjerner bekreftelse og matchpoeng; resultatet må bekreftes på nytt.</p>
    <button ref={opener} type="button" disabled={disabled} aria-expanded={editing} onClick={() => setEditing(!editing)}>{editing ? 'Avbryt korrigering' : 'Korriger aksepterte rapporter'}</button>
    {editing && <div onKeyDown={e => { if (e.key === 'Escape') cancel() }}>
      <label>Korrigeringstype<select disabled={disabled} value={kind} onChange={e => setKind(e.target.value === 'organizer_ruling' ? 'organizer_ruling' : 'recording_error')}><option value="recording_error">Feilregistrering</option><option value="organizer_ruling">Arrangøravgjørelse</option></select></label>
      <label>Begrunnelse<textarea disabled={disabled} value={reason} onChange={e => setReason(e.target.value)} maxLength={1000} required /></label>
      <label>Behold rapporter frem til<select aria-label="Behold rapporter frem til" disabled={disabled} value={keep} onChange={e => setKeep(Number(e.target.value))}><option value={0}>Ingen · erstatt fra starten</option>{card.events.map((e, i) => <option key={i} value={i + 1}>{eventLabel(e, card)}</option>)}</select></label>
      <p>{card.events.length - keep} senere rapporter blir uttrykkelig erstattet. De gjenopprettes ikke automatisk.</p>
      <button type="button" disabled={disabled || !reason.trim()} onClick={() => correct()}>Lagre korrigert rapportrekke</button>
      {!terminal && <MatchEventForm card={proposed} admin disabled={disabled || !reason.trim()} onSubmit={correct} label="Lagre korrigering med ny rapport" />}
      {error && <p role="alert">{error}</p>}
    </div>}
  </section>
}
