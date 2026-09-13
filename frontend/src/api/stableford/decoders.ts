import { decodeArray, decodeBoolean, decodeInteger, decodeObject, decodeString, decodeTimestamp, decodeUuid, invalidData } from '../decoder'
import { entry, readEntry } from '../fourBall/decoders'
import type { FourBallReadEntry } from '../fourBall/contracts'
import { decodeScoreVisibility } from '../visibility'
import type { StablefordReadCard, StablefordScoringCard, StablefordValues } from './contracts'
const fail = (path: string): never => invalidData('Stableford-scorekort', path)
const integer = (value: unknown, path: string, min?: number, max?: number) => decodeInteger(value, path, min, max)
const nullable = (value: unknown, path: string) => value === null ? null : integer(value, path)
export function decodeStablefordValues(value: unknown, resolved: number, path: string): StablefordValues {
  const d = decodeObject(value, path)
  const names = ['gross_points', 'net_points', 'gross_equivalent', 'net_equivalent', 'actual_gross_total', 'actual_net_total']
  if (Object.keys(d).length !== names.length || names.some(key => !(key in d))) fail(path)
  const result = { gross_points: integer(d.gross_points, path, 0), net_points: integer(d.net_points, path, 0),
    gross_equivalent: integer(d.gross_equivalent, path), net_equivalent: integer(d.net_equivalent, path),
    actual_gross_total: nullable(d.actual_gross_total, path), actual_net_total: nullable(d.actual_net_total, path) }
  if ((resolved === 0 && (result.gross_points !== 0 || result.net_points !== 0))
    || result.gross_equivalent !== 2 * resolved - result.gross_points || result.net_equivalent !== 2 * resolved - result.net_points
    || (result.actual_gross_total === null) !== (result.actual_net_total === null)
    || (resolved !== 18 && result.actual_gross_total !== null)
    || (result.actual_gross_total !== null && (result.actual_gross_total < 18 || result.actual_gross_total > 360))) fail(path)
  return result
}
function card<E extends FourBallReadEntry>(value: unknown, round: string, player: string,
  decodeEntry: (value: unknown, path: string, round: string, hole: string, player: string) => E) {
  const data = decodeObject(value, 'card'); const owner = decodeObject(data.owner, 'owner')
  if (data.format !== 'individual_stableford' || data.round_id !== round || owner.type !== 'player' || owner.id !== player) fail('identity')
  const playing_handicap = integer(data.playing_handicap, 'playing_handicap', -32768, 32767)
  const visibility = decodeScoreVisibility(data.visibility, 'visibility', 'Stableford-scorekort')
  const restricted = visibility.mode === 'front_nine'; const visible = restricted ? 9 : 18
  if (data.number_of_holes !== 18 || data.visible_hole_count !== visible) fail('holes')
  const holes = decodeArray(data.holes, 'holes', (value, path) => {
    const h = decodeObject(value, path); const hole_id = decodeUuid(h.hole_id, path)
    const stroke_index = integer(h.stroke_index, path, 1, 18); const par = integer(h.par, path, 1, 10)
    const handicap_strokes = integer(h.handicap_strokes, path); const magnitude = Math.abs(playing_handicap)
    const allocated = Math.sign(playing_handicap) * (Math.floor(magnitude / 18)
      + Number(playing_handicap < 0 ? stroke_index > 18 - magnitude % 18 : stroke_index <= magnitude % 18))
    if (allocated !== handicap_strokes) fail(`${path}.handicap_strokes`)
    const score = h.score === null ? null : decodeEntry(h.score, path, round, hole_id, player)
    const net_strokes = nullable(h.net_strokes, path); const gross_points = nullable(h.gross_points, path); const net_points = nullable(h.net_points, path)
    const numeric = score?.input.type === 'numeric' ? score.input.gross_strokes : null
    if (net_strokes !== (numeric === null ? null : numeric - handicap_strokes)
      || gross_points !== (score === null ? null : numeric === null ? 0 : Math.max(0, 2 + par - numeric))
      || net_points !== (score === null ? null : numeric === null ? 0 : Math.max(0, 2 + par - numeric + handicap_strokes))) fail(`${path}.points`)
    return { hole_id, hole_number: integer(h.hole_number, path, 1, visible), par, stroke_index, handicap_strokes, score, net_strokes, gross_points, net_points }
  })
  if (holes.length !== visible || new Set(holes.map(h => h.hole_id)).size !== visible || new Set(holes.map(h => h.hole_number)).size !== visible
    || new Set(holes.map(h => h.stroke_index)).size !== visible) fail('holes.identity')
  const scored = holes.filter(h => h.score !== null); const resolved = scored.length
  const values = data.values === null ? null : decodeStablefordValues(data.values, resolved, 'values')
  if (data.holes_scored !== resolved || (values === null) !== (resolved === 0)) fail('progress')
  if (values) {
    const fullNumeric = holes.length === 18 && holes.every(h => h.score?.input.type === 'numeric')
    if (values.gross_points !== scored.reduce((sum, h) => sum + (h.gross_points ?? 0), 0)
      || values.net_points !== scored.reduce((sum, h) => sum + (h.net_points ?? 0), 0)
      || values.actual_gross_total !== (fullNumeric ? holes.reduce((sum, h) => sum + (h.score?.input.type === 'numeric' ? h.score.input.gross_strokes : 0), 0) : null)
      || values.actual_net_total !== (fullNumeric ? holes.reduce((sum, h) => sum + (h.net_strokes ?? 0), 0) : null)) fail('totals')
  }
  const complete = data.complete === null ? null : decodeBoolean(data.complete, 'complete')
  const confirmed = data.confirmed === null ? null : decodeBoolean(data.confirmed, 'confirmed')
  const confirmed_at = data.confirmed_at === null ? null : decodeTimestamp(data.confirmed_at, 'confirmed_at')
  if (restricted ? complete !== null || confirmed !== null || confirmed_at !== null
    : complete !== (resolved === 18) || confirmed === null || (confirmed && !complete) || confirmed !== (confirmed_at !== null)) fail('completion')
  return { format: 'individual_stableford' as const, round_id: round, owner: { type: 'player' as const, id: player },
    owner_name: decodeString(data.owner_name, 'owner_name'), playing_handicap, holes, values, holes_scored: resolved, number_of_holes: 18,
    visible_hole_count: visible, complete, confirmed, confirmed_at, visibility }
}
export function decodeStablefordRead(value: unknown, round: string, player: string): StablefordReadCard {
  return { ...card(value, round, player, readEntry), projection: 'read' }
}
export function decodeStablefordScoring(value: unknown, round: string, player: string): StablefordScoringCard {
  const result = card(value, round, player, entry)
  if (result.visibility.mode !== 'full') fail('scoring.visibility')
  return { ...result, projection: 'scoring' }
}
