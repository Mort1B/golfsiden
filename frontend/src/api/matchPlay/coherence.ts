import { invalidData } from '../decoder'
import type { MatchCard, MatchFinish } from './contracts'
function fail(): never { return invalidData('matchdata', 'accepted evidence') }
export function validateMatchEvidence(card: MatchCard): void {
  const [first, second] = card.opponents
  const a = first.playing_handicap, b = second.playing_handicap
  if (card.round_status !== 'draft' && (a === null || b === null)) fail()
  if (a !== null && b !== null && card.mode === 'net') {
    if (card.relative_handicaps[0] !== a - Math.min(a, b) || card.relative_handicaps[1] !== b - Math.min(a, b)) fail()
  }
  const slot = (id: string): 'first' | 'second' => id === first.player_id ? 'first' : id === second.player_id ? 'second' : fail()
  const other = (id: string): 'first' | 'second' => slot(id) === 'first' ? 'second' : 'first'
  const received = (relative: number, index: number) => Math.floor(relative / 18) + Number(index <= relative % 18)
  let lead = 0, resolved = 0, finish: MatchFinish | null = null
  for (const event of card.events) {
    if (finish) fail()
    if (event.type === 'concession') {
      if (!event.communicated || event.after_hole !== resolved) fail()
      finish = { type: 'conceded', winner: other(event.conceding_player_id) }
    } else if (event.type === 'award') {
      if (event.after_hole !== resolved) fail()
      finish = { type: 'awarded', winner: slot(event.winner_player_id) }
    } else {
      const basis = event.basis
      if (basis.type === 'numeric' || basis.type === 'next_stroke_concession') {
        const index = card.holes.find(h => h.hole_number === event.hole_number)?.stroke_index
        if (!basis.agreed || index === undefined) fail()
        const delta = basis.first_gross - received(card.relative_handicaps[0], index) - basis.second_gross + received(card.relative_handicaps[1], index)
        if (event.outcome !== (delta === 0 ? 'halved' : delta < 0 ? 'first' : 'second')) fail()
        if (basis.type === 'next_stroke_concession') { slot(basis.conceding_player_id); if (!basis.communicated) fail() }
      } else if (basis.type === 'hole_concession') {
        if (!basis.communicated || event.outcome !== other(basis.conceding_player_id)) fail()
      } else if (basis.type === 'agreed_halve' && (!basis.play_begun || !basis.mutual_agreement || event.outcome !== 'halved')) fail()
      resolved++
      lead += event.outcome === 'first' ? 1 : event.outcome === 'second' ? -1 : 0
      if (Math.abs(lead) > 18 - resolved) finish = { type: 'on_holes', winner: lead > 0 ? 'first' : 'second', margin: Math.abs(lead), holes_remaining: 18 - resolved }
      else if (resolved === 18) finish = { type: 'draw' }
    }
  }
  if (JSON.stringify(finish) !== JSON.stringify(card.finish)) fail()
  if (card.confirmed && finish) {
    const points = finish.type === 'draw' ? [1, 1] : finish.winner === 'first' ? [2, 0] : [0, 2]
    if (JSON.stringify(points) !== JSON.stringify(card.half_points)) fail()
  }
}
