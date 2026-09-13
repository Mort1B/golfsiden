import { validateMatchEvidence } from './coherence'
import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeUuid, invalidData } from '../decoder'
import { decodeScoreRevision } from '../scorecards/revision'
import { decodeScoreVisibility } from '../visibility'
import type { RoundStatus } from '../types'
import type { MatchCard, MatchScoringCard, MatchOpponent, MatchFinish } from './contracts'
import { decodeEvent, exact, outcome } from './eventDecoder'
export function nullableBoolean(value: unknown): boolean | null { return value === null ? null : decodeBoolean(value, 'boolean') }
export function status(value: unknown): RoundStatus {
  if (value !== 'draft' && value !== 'open' && value !== 'completed' && value !== 'locked') invalidData('matchdata', 'status')
  return value
}
function opponent(value: unknown): MatchOpponent {
  const d = decodeObject(value, 'opponent'); exact(d, ['player_id', 'display_name', 'playing_handicap'])
  return { player_id: decodeUuid(d.player_id, 'player'), display_name: decodeString(d.display_name, 'name'), playing_handicap: d.playing_handicap === null ? null : decodeInteger(d.playing_handicap, 'handicap', -32768, 32767) }
}
function finish(value: unknown): MatchFinish | null {
  if (value === null) return null
  const d = decodeObject(value, 'finish')
  if (d.type === 'draw') { exact(d, ['type']); return { type: 'draw' } }
  const winner = outcome(d.winner); if (winner === 'halved') invalidData('matchdata', 'winner')
  if (d.type === 'on_holes') {
    exact(d, ['type', 'winner', 'margin', 'holes_remaining'])
    return { type: d.type, winner, margin: decodeInteger(d.margin, 'margin', 1, 18), holes_remaining: decodeInteger(d.holes_remaining, 'remaining', 0, 17) }
  }
  if (d.type === 'conceded' || d.type === 'awarded') { exact(d, ['type', 'winner']); return { type: d.type, winner } }
  return invalidData('matchdata', 'finish')
}
export function decodeCard(value: unknown, round: string, match: string, scoring: true): MatchScoringCard
export function decodeCard(value: unknown, round: string, match: string, scoring?: false): MatchCard
export function decodeCard(value: unknown, round: string, match: string, scoring = false): MatchCard | MatchScoringCard {
  const d = decodeObject(value, 'match')
  exact(d, ['format', 'match_id', 'round_id', 'tournament_id', 'round_status', 'mode', 'opponents', 'relative_handicaps', 'holes', 'notes', 'events', 'resolved_holes', 'lead', 'finish', 'confirmed', 'correction_pending', 'half_points', 'visibility', ...(scoring ? ['revision', 'accepted_events'] : [])])
  if (d.format !== 'singles_match_play' || d.mode !== 'net' && d.mode !== 'gross') invalidData('matchdata', 'format')
  const players = decodeArray(d.opponents, 'opponents', opponent)
  const [first, second] = players
  if (!first || !second || players.length !== 2 || first.player_id === second.player_id) invalidData('matchdata', 'opponents')
  const relative = decodeArray(d.relative_handicaps, 'relative', v => decodeInteger(v, 'relative', 0, 65535))
  const [a, b] = relative
  if (a === undefined || b === undefined || relative.length !== 2 || Math.min(a, b) !== 0 || d.mode === 'gross' && a + b !== 0) invalidData('matchdata', 'allocation')
  const points = d.half_points === null ? null : decodeArray(d.half_points, 'points', v => decodeInteger(v, 'points', 0, 2))
  const p1 = points?.[0], p2 = points?.[1]
  if (points && (points.length !== 2 || p1 === undefined || p2 === undefined || p1 + p2 !== 2)) invalidData('matchdata', 'points')
  const card: MatchCard = {
    format: 'singles_match_play', match_id: decodeUuid(d.match_id, 'match_id'), round_id: decodeUuid(d.round_id, 'round_id'), tournament_id: decodeUuid(d.tournament_id, 'tournament_id'), round_status: status(d.round_status), mode: d.mode,
    opponents: [first, second], relative_handicaps: [a, b],
    holes: decodeArray(d.holes, 'holes', v => { const h = decodeObject(v, 'hole'); exact(h, ['hole_number', 'par', 'stroke_index']); return { hole_number: decodeInteger(h.hole_number, 'hole', 1, 18), par: decodeInteger(h.par, 'par', 1, 10), stroke_index: decodeInteger(h.stroke_index, 'index', 1, 18) } }),
    notes: decodeArray(d.notes, 'notes', v => { const n = decodeObject(v, 'note'); exact(n, ['player_id', 'hole_number', 'gross_strokes']); return { player_id: decodeUuid(n.player_id, 'player'), hole_number: decodeInteger(n.hole_number, 'hole', 1, 18), gross_strokes: n.gross_strokes === null ? null : decodeInteger(n.gross_strokes, 'gross', 1, 20) } }),
    events: decodeArray(d.events, 'events', decodeEvent), resolved_holes: decodeInteger(d.resolved_holes, 'resolved', 0, 18), lead: decodeInteger(d.lead, 'lead', -18, 18), finish: finish(d.finish), confirmed: nullableBoolean(d.confirmed), correction_pending: nullableBoolean(d.correction_pending),
    half_points: p1 !== undefined && p2 !== undefined ? [p1, p2] : null, visibility: decodeScoreVisibility(d.visibility, 'visibility', 'matchdata'),
  }
  if (card.round_id !== round || card.match_id !== match) invalidData('matchdata', 'identity')
  const limit = card.visibility.mode === 'front_nine' ? 9 : 18
  if (card.holes.some((h, i) => h.hole_number !== i + 1 || h.hole_number > limit) || new Set(card.holes.map(h => h.stroke_index)).size !== card.holes.length
    || card.round_status !== 'draft' && card.holes.length !== limit || card.notes.some(n => !players.some(p => p.player_id === n.player_id) || n.hole_number > limit)
    || new Set(card.notes.map(n => `${n.player_id}:${n.hole_number}`)).size !== card.notes.length) invalidData('matchdata', 'holes/notes')
  if (card.visibility.mode === 'front_nine' && (card.confirmed !== null || card.correction_pending !== null || card.half_points !== null || scoring)) invalidData('matchdata', 'hidden metadata')
  if (card.visibility.mode === 'full' && (card.confirmed === null || card.correction_pending === null)) invalidData('matchdata', 'metadata')
  if ((card.confirmed === true) !== (card.half_points !== null) || card.confirmed && !card.finish) invalidData('matchdata', 'confirmation')
  let resolved = 0, lead = 0, terminal = false
  for (const event of card.events) {
    if (terminal) invalidData('matchdata', 'post-finish event')
    if (event.type === 'hole') {
      if (event.hole_number !== ++resolved || resolved > limit) invalidData('matchdata', 'sequence')
      lead += event.outcome === 'first' ? 1 : event.outcome === 'second' ? -1 : 0
      terminal = Math.abs(lead) > 18 - resolved || resolved === 18
    } else {
      if (event.after_hole !== resolved || limit === 9 && event.after_hole >= 9) invalidData('matchdata', 'effective point')
      terminal = true
    }
  }
  if (resolved !== card.resolved_holes || lead !== card.lead || terminal !== (card.finish !== null)) invalidData('matchdata', 'derived state')
  validateMatchEvidence(card)
  if (!scoring) return card
  const accepted = decodeArray(d.accepted_events, 'accepted', v => { const e = decodeObject(v, 'accepted'); exact(e, ['id', 'event']); return { id: decodeUuid(e.id, 'event.id'), event: decodeEvent(e.event) } })
  if (new Set(accepted.map(e => e.id)).size !== accepted.length || JSON.stringify(accepted.map(e => e.event)) !== JSON.stringify(card.events)) invalidData('matchdata', 'accepted')
  return { ...card, revision: decodeScoreRevision(d.revision), accepted_events: accepted }
}
