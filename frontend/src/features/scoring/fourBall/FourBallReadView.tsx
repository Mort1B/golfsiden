import { inputLabel, type FourBallReadCard } from '../../../api/fourBall'
import type { ScoreView } from '../selection'
import { FourBallSummary, FourBallResult, FourBallTotals, StrokeBadge } from './FourBallSummary'
export function FourBallReadView({ card, view, holeNumber, onHole }: {
  card: FourBallReadCard; view: ScoreView; holeNumber: number; onHole: (hole: number) => void
}) {
  const hole = card.holes.find(item => item.hole_number === holeNumber)
  if (view === 'summary') return <FourBallSummary card={card} onHole={onHole} />
  if (!hole) return <p>Ingen synlige hull.</p>
  return <>
    {card.visibility.mode === 'front_nine' && <p>Kun de første ni hullene vises.</p>}
    <h3>Hull {hole.hole_number} · par {hole.par} · indeks {hole.stroke_index}</h3>
    {hole.players.map(player => <p className="four-ball-partner-summary" key={player.player_id}>
      <span>{card.partners.find(partner => partner.player_id === player.player_id)?.display_name} <StrokeBadge strokes={player.handicap_strokes} hole={hole.hole_number} /></span>
      <strong>{inputLabel(player.score?.input ?? null)}</strong>
    </p>)}
    <FourBallResult card={card} hole={hole} />
    <div className="four-ball-actions"><button type="button" disabled={hole.hole_number <= 1} onClick={() => onHole(hole.hole_number - 1)}>Forrige hull</button>
      <button type="button" disabled={hole.hole_number >= card.visible_hole_count} onClick={() => onHole(hole.hole_number + 1)}>Neste hull</button></div>
    <FourBallTotals card={card} />
  </>
}
