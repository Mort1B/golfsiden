import { useState } from 'react'
import type { MatchCard, MatchEvent, MatchOutcome } from '../../api/matchPlay'
import { proposal } from './format'
interface Props { card: MatchCard; admin: boolean; disabled: boolean; onSubmit: (event: MatchEvent) => void; label?: string }
export function MatchEventForm({ card, admin, disabled, onSubmit, label = 'Rapporter avtalt resultat' }: Props) {
  const [kind, setKind] = useState('numeric'), [first, setFirst] = useState(''), [second, setSecond] = useState('')
  const [player, setPlayer] = useState(card.opponents[0].player_id), [outcome, setOutcome] = useState<MatchOutcome>('halved')
  const [reason, setReason] = useState(''), [agreed, setAgreed] = useState(false), [communicated, setCommunicated] = useState(false), [begun, setBegun] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const number = card.resolved_holes + 1
  const numeric = kind === 'numeric' || kind === 'next_stroke_concession'
  const a = Number(first), b = Number(second), validNumeric = first !== '' && second !== '' && Number.isInteger(a) && Number.isInteger(b) && a >= 1 && a <= 20 && b >= 1 && b <= 20
  const proposed = numeric && validNumeric && number <= 18 ? proposal(card, number, a, b) : null
  const submit = () => {
    setError(null)
    let event: MatchEvent
    if (kind === 'concession') {
      if (!communicated) { setError('Bekreft at den navngitte spilleren ga matchen.'); return }
      event = { type: 'concession', conceding_player_id: player, communicated, after_hole: card.resolved_holes }
    } else if (kind === 'award') {
      if (!admin || !reason.trim()) { setError('Arrangøravgjørelse krever administrator og begrunnelse.'); return }
      event = { type: 'award', winner_player_id: player, reason: reason.trim(), after_hole: card.resolved_holes }
    } else {
      if (number > 18) { setError('Alle hull er rapportert.'); return }
      if (numeric) {
        if (!validNumeric || !agreed || !proposed || kind === 'next_stroke_concession' && !communicated) { setError('Fyll inn begge avtalte scorer og bekreft attestasjonene.'); return }
        event = { type: 'hole', hole_number: number, outcome: proposed, basis: kind === 'numeric' ? { type: 'numeric', first_gross: a, second_gross: b, agreed } : { type: 'next_stroke_concession', first_gross: a, second_gross: b, agreed, conceding_player_id: player, communicated } }
      } else if (kind === 'hole_concession') {
        if (!communicated) { setError('Bekreft at spilleren kommuniserte det gitte hullet.'); return }
        event = { type: 'hole', hole_number: number, outcome: player === card.opponents[0].player_id ? 'second' : 'first', basis: { type: 'hole_concession', conceding_player_id: player, communicated } }
      } else if (kind === 'agreed_halve') {
        if (!begun || !agreed) { setError('Spillet på hullet må ha begynt, og begge må ha avtalt delingen.'); return }
        event = { type: 'hole', hole_number: number, outcome: 'halved', basis: { type: 'agreed_halve', play_begun: begun, mutual_agreement: agreed } }
      } else {
        if (!admin || !reason.trim()) { setError('Begrunn arrangøravgjørelsen.'); return }
        event = { type: 'hole', hole_number: number, outcome, basis: { type: 'organizer_ruling', reason: reason.trim() } }
      }
    }
    onSubmit(event)
  }
  return <form className="match-panel" onSubmit={e => { e.preventDefault(); submit() }}>
    <h3>Neste rapport · hull {Math.min(number, 18)}</h3>
    <p>En numerisk sammenligning er bare et forslag. Registrer hva motstanderne faktisk avtalte. Bare den faktiske spilleren kan gi et slag, hull eller en match.</p>
    <fieldset disabled={disabled}><legend>Rapportgrunnlag</legend>
      <label>Type rapport<select aria-label="Type rapport" value={kind} onChange={e => { setKind(e.target.value); setAgreed(false); setCommunicated(false); setBegun(false) }}>
        <option value="numeric">Begge fullførte · numerisk score</option><option value="next_stroke_concession">Gitt neste slag · medregnet i score</option><option value="hole_concession">Gitt hull</option><option value="agreed_halve">Avtalt delt hull</option><option value="concession">Gitt hele matchen</option>
        {admin && <><option value="organizer_ruling">Arrangøravgjørelse for hullet</option><option value="award">Arrangør tildeler matchseier</option></>}
      </select></label>
      {numeric && <>{card.opponents.map((p, i) => <label key={p.player_id}>Avtalt bruttoscore · {p.display_name}<input type="number" min={1} max={20} inputMode="numeric" value={i === 0 ? first : second} onChange={e => i === 0 ? setFirst(e.target.value) : setSecond(e.target.value)} /></label>)}
        {proposed && <p>Forslag i offisiell {card.mode === 'net' ? 'netto' : 'brutto'}: {proposed === 'halved' ? 'Delt hull' : card.opponents[proposed === 'first' ? 0 : 1].display_name}</p>}</>}
      {(kind.includes('concession') || kind === 'award') && <label>{kind === 'award' ? 'Tildelt vinner' : 'Spilleren som faktisk ga slaget, hullet eller matchen'}<select aria-label={kind === 'award' ? 'Tildelt vinner' : 'Spilleren som faktisk ga slaget, hullet eller matchen'} value={player} onChange={e => setPlayer(e.target.value)}>{card.opponents.map(p => <option key={p.player_id} value={p.player_id}>{p.display_name}</option>)}</select></label>}
      {kind.includes('concession') && <label className="match-check"><input type="checkbox" checked={communicated} onChange={e => setCommunicated(e.target.checked)} />Jeg bekrefter at den navngitte spilleren kommuniserte dette.</label>}
      {kind === 'agreed_halve' && <label className="match-check"><input type="checkbox" checked={begun} onChange={e => setBegun(e.target.checked)} />Spillet på hullet var påbegynt.</label>}
      {(numeric || kind === 'agreed_halve') && <label className="match-check"><input type="checkbox" checked={agreed} onChange={e => setAgreed(e.target.checked)} />Begge motstanderne har avtalt dette hullresultatet.</label>}
      {kind === 'organizer_ruling' && <label>Avgjort hullresultat<select value={outcome} onChange={e => { const v = e.target.value; if (v === 'first' || v === 'second' || v === 'halved') setOutcome(v) }}><option value="halved">Delt</option><option value="first">{card.opponents[0].display_name}</option><option value="second">{card.opponents[1].display_name}</option></select></label>}
      {(kind === 'organizer_ruling' || kind === 'award') && <label>Begrunnelse<textarea required value={reason} onChange={e => setReason(e.target.value)} maxLength={1000} /></label>}
      <button type="submit">{label}</button>
    </fieldset>{error && <p role="alert">{error}</p>}
  </form>
}
