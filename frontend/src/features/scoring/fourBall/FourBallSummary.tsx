import { inputLabel, type FourBallCard, type FourBallHole, type FourBallSelected } from '../../../api/fourBall'
import './fourBall.css'
export function StrokeBadge({ strokes, hole }: { strokes: number; hole: number }) {
  if (strokes === 0) return null
  return <span className="handicap-stroke-badge" role="img" aria-label={strokes > 0 ? `${strokes} ekstra slag på hull ${hole}` : `${Math.abs(strokes)} slag gis tilbake på hull ${hole}`}>
    {strokes > 0 ? '+' : '−'}{Math.abs(strokes)}
  </span>
}
function winner(value: FourBallSelected | null, card: FourBallCard): string {
  if (!value) return 'Ingen tellende score'
  return `${value.strokes} · ${value.player_ids.map(id => card.partners.find(p => p.player_id === id)?.display_name ?? '').join(' / ')}`
}
export function FourBallResult({ card, hole }: { card: FourBallCard; hole: FourBallHole }) {
  return <dl className="four-ball-result" aria-label={`Lagets serverscore på hull ${hole.hole_number}`}>
    <div><dt>Brutto</dt><dd>{winner(hole.gross, card)}</dd></div>
    <div><dt>Netto</dt><dd>{winner(hole.net, card)}</dd></div>
  </dl>
}
export function FourBallTotals({ card }: { card: FourBallCard }) {
  return <dl className="scorecard-strip" aria-label="Summer fra serveren">
    <div><dt>Brutto</dt><dd>{card.gross_total ?? '–'}</dd></div>
    <div><dt>Netto</dt><dd>{card.net_total ?? '–'}</dd></div>
    <div><dt>Hull</dt><dd>{card.holes_scored}/{card.visible_hole_count}</dd></div>
  </dl>
}
export function FourBallSummary({ card, onHole, disabled = false }: { card: FourBallCard; onHole?: (hole: number) => void; disabled?: boolean }) {
  return <section className="four-ball-summary" aria-label="Oppsummering av four-ball">
    <FourBallTotals card={card} />
    {card.visibility.mode === 'front_nine' && <p>Kun de første ni hullene vises. Bekreftelse og samlet fullføring er skjult.</p>}
    <p>Lagets laveste brutto og netto velges hver for seg. Summer viser lagrede scorer. Lokale endringer venter på levering.</p>
    <ol>{card.holes.map(hole => <li key={hole.hole_id}>
      {onHole ? <button type="button" disabled={disabled} onClick={() => onHole(hole.hole_number)}>Hull {hole.hole_number} · par {hole.par}</button> : <h3>Hull {hole.hole_number} · par {hole.par}</h3>}
      {hole.players.map(player => <p className="four-ball-partner-summary" key={player.player_id}>
        <span>{card.partners.find(partner => partner.player_id === player.player_id)?.display_name} <StrokeBadge strokes={player.handicap_strokes} hole={hole.hole_number} /></span>
        <strong>{inputLabel(player.score?.input ?? null)}</strong>
      </p>)}
      <FourBallResult card={card} hole={hole} />
    </li>)}</ol>
  </section>
}
