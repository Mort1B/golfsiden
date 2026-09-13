import type { MatchCard, MatchEvent, MatchOutcome } from '../../api/matchPlay'
export const matchUrl = (round: string, match?: string, scoring = false): string => `/rounds/${round}/matches${match ? `/${match}${scoring ? '/score' : ''}` : ''}`
export const tableUrl = (tournament: string, player?: string): string => `/tournaments/${tournament}/match-results${player ? `?player=${encodeURIComponent(player)}` : ''}`
export function pointsLabel(half: number): string { return `${Math.floor(half / 2)}${half % 2 ? '½' : ''}`.replace(/^0½$/, '½') }
export function matchResult(card: MatchCard): string {
  const f = card.finish
  if (!f) return card.resolved_holes === 0 ? 'Ikke startet' : card.lead === 0 ? `Lik etter ${card.resolved_holes} hull` : `${card.opponents[card.lead > 0 ? 0 : 1].display_name} leder ${Math.abs(card.lead)} etter ${card.resolved_holes} hull`
  if (f.type === 'draw') return 'Delt match'
  const winner = card.opponents[f.winner === 'first' ? 0 : 1].display_name
  return `${winner} vant ${f.type === 'on_holes' ? `${f.margin}&${f.holes_remaining}` : f.type === 'conceded' ? 'ved gitt match' : 'etter arrangøravgjørelse'}`
}
export function strokes(relative: number, index: number): number { return Math.floor(relative / 18) + Number(index <= relative % 18) }
export function proposal(card: MatchCard, hole: number, first: number, second: number): MatchOutcome {
  const index = card.holes.find(h => h.hole_number === hole)?.stroke_index
  if (index === undefined) throw new Error('Hull mangler')
  const difference = first - strokes(card.relative_handicaps[0], index) - second + strokes(card.relative_handicaps[1], index)
  return difference === 0 ? 'halved' : difference < 0 ? 'first' : 'second'
}
export function eventLabel(event: MatchEvent, card: MatchCard): string {
  if (event.type === 'concession') return `Match gitt av ${card.opponents.find(p => p.player_id === event.conceding_player_id)?.display_name ?? 'motstander'}`
  if (event.type === 'award') return `Arrangøravgjørelse: ${event.reason}`
  const labels = { numeric: 'avtalt numerisk score', next_stroke_concession: 'gitt neste slag', hole_concession: 'gitt hull', agreed_halve: 'avtalt deling', organizer_ruling: 'arrangøravgjørelse' }
  return `Hull ${event.hole_number}: ${event.outcome === 'halved' ? 'Delt' : card.opponents[event.outcome === 'first' ? 0 : 1].display_name} · ${labels[event.basis.type]}`
}
